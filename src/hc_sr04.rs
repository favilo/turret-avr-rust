use core::{
    cell::Cell,
    sync::atomic::{AtomicU8, Ordering},
};

use arduino_hal::{
    delay_us,
    hal::port::{Dynamic, PD0, PD1},
    pac::{EXINT, USART0},
    port::{
        mode::{Floating, Input, Output},
        Pin, PinOps,
    },
    Usart,
};
use avr_device::interrupt::Mutex;
use fugit::Duration;

use crate::{
    clock::CLOCK,
    interrupt::{AttachHwInterrupt, ExtIntMode},
};

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
enum HcSr04State {
    Idle = 0,
    Triggered = 1,
    Measuring = 2,
}

impl From<u8> for HcSr04State {
    fn from(value: u8) -> Self {
        match value {
            1 => HcSr04State::Triggered,
            2 => HcSr04State::Measuring,
            _ => HcSr04State::Idle,
        }
    }
}

#[derive(Clone, Copy, Debug, ufmt::derive::uDebug, PartialEq)]
pub enum HcSr04Error {
    InvalidResult,
    NoEcho,
    NoTrigger,
}

static STATE: AtomicU8 = AtomicU8::new(HcSr04State::Idle as u8);
static TRIGGER_TIME: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));
static ECHO_TIME: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

pub struct HcSr04<ECHO> {
    trigger: Pin<Output, Dynamic>,
    echo: Pin<Input<Floating>, ECHO>,

    trigger_time: u32,
    wait_time: u32,
}

impl<ECHO> HcSr04<ECHO>
where
    Pin<Input<Floating>, ECHO>: AttachHwInterrupt,
    ECHO: PinOps,
{
    #[allow(dead_code)]
    pub fn new<TRIGGER>(trigger: Pin<Output, TRIGGER>, echo: Pin<Input<Floating>, ECHO>) -> Self
    where
        TRIGGER: arduino_hal::port::PinOps<Dynamic = Dynamic>,
    {
        let trigger = trigger.downgrade();

        Self {
            trigger,
            echo,

            trigger_time: 10,
            wait_time: 10,
        }
    }

    #[allow(dead_code)]
    pub fn measure_us(
        &mut self,
        _serial: &mut Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>>,
        exint: &EXINT,
        timeout: Option<Duration<u32, 1, 40_000>>,
    ) -> Result<Duration<u32, 1, 40_000>, HcSr04Error> {
        assert!(STATE.load(Ordering::SeqCst) == HcSr04State::Idle as u8);
        let start = CLOCK.now_instant();

        avr_device::interrupt::free(|cs| {
            TRIGGER_TIME.borrow(cs).set(0);
            ECHO_TIME.borrow(cs).set(0);
        });

        // Ensure trigger pin is low
        self.trigger.set_low();
        arduino_hal::delay_us(4);

        // Hold trigger pin high for 10 microseconds (default), which signals
        // the sensor to measure distance
        self.trigger.set_high();
        arduino_hal::delay_us(self.trigger_time);

        // Set trigger pin low again, and wait to give time for sending the
        // signal without interference
        self.trigger.set_low();
        arduino_hal::delay_us(self.wait_time);

        STATE.store(HcSr04State::Triggered as u8, Ordering::SeqCst);
        // Attach interrupt to echo pin for the starting point
        // TODO: add static status atomic to check state
        self.echo.attach_hw_int(&exint, ExtIntMode::Rising);

        loop {
            let checked_duration_since = CLOCK
                .now_instant()
                .checked_duration_since(start)
                .expect("Should be in the future");
            if let Some(timeout) = timeout {
                if checked_duration_since > timeout {
                    // Set waiting to false, because we timedout
                    // waiting = false;
                    break;
                }
            }
            delay_us(1);

            let trigger = avr_device::interrupt::free(|cs| TRIGGER_TIME.borrow(cs).get());
            if trigger > 0 && STATE.load(Ordering::SeqCst) == HcSr04State::Triggered as u8 {
                STATE.store(HcSr04State::Measuring as u8, Ordering::SeqCst);

                // ufmt::uwriteln!(serial, "Trigger found: {}\nSetting echo interrupt", trigger)
                //     .unwrap_infallible();
                // Attach interrupt to echo pin for the ending point
                self.echo.attach_hw_int(&exint, ExtIntMode::Falling);
            }
            // Slow
            if self.echo.is_high() {
                // ufmt::uwriteln!(serial, "echo is high").unwrap_infallible();
                let now = CLOCK.now();
                avr_device::interrupt::free(|cs| TRIGGER_TIME.borrow(cs).set(now));
                continue;
            }

            let echo = avr_device::interrupt::free(|cs| ECHO_TIME.borrow(cs).get());
            if trigger > 0
                && echo > 0
                && STATE.load(Ordering::SeqCst) == HcSr04State::Measuring as u8
            {
                // ufmt::uwriteln!(serial, "Echo found: {}", echo).unwrap_infallible();
                break;
            }
            // if trigger > 0 && self.echo.is_low() {
            //     // ufmt::uwriteln!(serial, "echo is low").unwrap_infallible();
            //     let now = CLOCK.now();
            //     avr_device::interrupt::free(|cs| ECHO_TIME.borrow(cs).set(now));
            //     continue;
            // }
        }

        // ufmt::uwriteln!(serial, "detach trigger").unwrap_infallible();
        // Detach interrupt from echo pin
        self.echo.detach_hw_int(&exint);
        STATE.store(HcSr04State::Idle as u8, Ordering::SeqCst);

        // ufmt::uwriteln!(serial, "fetching time").unwrap_infallible();
        let (trigger, echo) = avr_device::interrupt::free(|cs| {
            (TRIGGER_TIME.borrow(cs).get(), ECHO_TIME.borrow(cs).get())
        });

        if trigger == 0 {
            return Err(HcSr04Error::NoTrigger);
        }
        if echo == 0 {
            return Err(HcSr04Error::NoEcho);
        }
        if echo <= trigger {
            return Err(HcSr04Error::InvalidResult);
        }
        return Ok(Duration::<u32, 1, 40_000>::from_ticks(echo - trigger));
    }
}

/// External Interrupt 1
/// This is for 3, or PD3
#[avr_device::interrupt(atmega328p)]
fn INT1() {
    match STATE.load(Ordering::SeqCst).into() {
        HcSr04State::Triggered => {
            // Start measuring
            avr_device::interrupt::free(|cs| {
                TRIGGER_TIME.borrow(cs).set(CLOCK.now());
            });
        }
        HcSr04State::Measuring => {
            // Stop measuring
            avr_device::interrupt::free(|cs| {
                ECHO_TIME.borrow(cs).set(CLOCK.now());
            });
        }
        _ => {}
    }
}
