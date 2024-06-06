use core::{
    cell::Cell,
    sync::atomic::{AtomicU8, Ordering},
};

use arduino_hal::{
    delay_us,
    hal::port::Dynamic,
    pac::EXINT,
    port::{
        mode::{Floating, Input, Output},
        Pin, PinOps,
    },
};
use avr_device::interrupt::Mutex;
use fugit::Duration;
use uom::si::{
    f32::*, quantities::Time, temperature_interval::degree_celsius, time::microsecond,
    velocity::meter_per_second,
};

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

    speed_of_sound: Velocity,

    timeout: Duration<u32, 1, 40_000>,
}

impl<ECHO> HcSr04<ECHO>
where
    Pin<Input<Floating>, ECHO>: AttachHwInterrupt,
    ECHO: PinOps,
{
    #[allow(dead_code)]
    pub fn new<TRIGGER>(
        temperature: TemperatureInterval,
        trigger: Pin<Output, TRIGGER>,
        echo: Pin<Input<Floating>, ECHO>,
    ) -> Self
    where
        TRIGGER: arduino_hal::port::PinOps<Dynamic = Dynamic>,
    {
        let trigger = trigger.downgrade();
        let speed_of_sound = Velocity::new::<meter_per_second>(
            331.0 + (0.606 * temperature.get::<degree_celsius>()),
        );
        let timeout_seconds = 4.0 / speed_of_sound.get::<meter_per_second>() * 2.0;
        let timeout_ticks = timeout_seconds * 80_000.0;
        let timeout = Duration::<u32, 1, 40_000>::from_ticks(timeout_ticks as u32);

        Self {
            trigger,
            echo,

            trigger_time: 10,
            wait_time: 10,

            speed_of_sound,
            timeout,
        }
    }

    #[allow(dead_code)]
    pub fn measure_us(&mut self, exint: &EXINT) -> Result<Duration<u32, 1, 40_000>, HcSr04Error> {
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
            if checked_duration_since > self.timeout {
                // Set waiting to false, because we timedout
                // waiting = false;
                break;
            }
            delay_us(1);

            let trigger = avr_device::interrupt::free(|cs| TRIGGER_TIME.borrow(cs).get());
            if trigger > 0 && STATE.load(Ordering::SeqCst) == HcSr04State::Triggered as u8 {
                STATE.store(HcSr04State::Measuring as u8, Ordering::SeqCst);

                // Attach interrupt to echo pin for the ending point
                self.echo.attach_hw_int(&exint, ExtIntMode::Falling);
            }

            let echo = avr_device::interrupt::free(|cs| ECHO_TIME.borrow(cs).get());
            if trigger > 0
                && echo > 0
                && STATE.load(Ordering::SeqCst) == HcSr04State::Measuring as u8
            {
                break;
            }
        }

        // Detach interrupt from echo pin
        self.echo.detach_hw_int(&exint);
        STATE.store(HcSr04State::Idle as u8, Ordering::SeqCst);

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

    pub fn measure_distance(&mut self, exint: &EXINT) -> Result<Length, HcSr04Error> {
        let duration = self.measure_us(exint)?;
        let duration = Time::new::<microsecond>(duration.to_micros() as f32);
        Ok(self.speed_of_sound * duration / 2.0)
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
