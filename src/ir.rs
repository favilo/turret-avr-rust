use core::cell::Cell;

use arduino_hal::{
    hal::port::PB1,
    pac::{tc0::tccr0b::CS0_A, TC0},
    port::{
        mode::{Floating, Input},
        Pin,
    },
};
use avr_device::interrupt::Mutex;
use infrared::{
    protocol::{nec::NecCommand, *},
    Receiver,
};

pub(crate) static CLOCK: Clock = Clock::new();
static mut RECEIVER: Option<Receiver<Nec, Pin<Input<Floating>, PB1>, u32, NecCommand>> = None;
static CMD: Mutex<Cell<Option<NecCommand>>> = Mutex::new(Cell::new(None));

#[avr_device::interrupt(atmega328p)]
fn PCINT0() {
    let recv = unsafe { RECEIVER.as_mut().unwrap() };

    let now = CLOCK.now();

    match recv.event_instant(now) {
        Ok(Some(cmd)) => {
            avr_device::interrupt::free(|cs| {
                let cmd_cell = CMD.borrow(cs);
                cmd_cell.set(Some(cmd));
            });
        }
        Ok(None) => {}
        Err(_) => {
            // TODO: handle error
        }
    }
}

#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    CLOCK.tick();
}

pub fn fetch_message() -> Option<NecCommand> {
    avr_device::interrupt::free(|cs| CMD.borrow(cs).take())
}

pub fn replace_receiver(receiver: Receiver<Nec, Pin<Input<Floating>, PB1>, u32, NecCommand>) {
    unsafe { RECEIVER.replace(receiver) };
}

pub struct Clock {
    counter: Mutex<Cell<u32>>,
}

impl Clock {
    pub const FREQ: u32 = 20_000;
    const PRESCALER: CS0_A = CS0_A::PRESCALE_8;
    const TOP: u8 = 99;

    pub const fn new() -> Self {
        Self {
            counter: Mutex::new(Cell::new(0)),
        }
    }

    pub fn start(&self, tc0: TC0) {
        // Configure the timer for the above interval (in CTC mode)
        tc0.tccr0a.write(|w| w.wgm0().ctc());
        tc0.ocr0a.write(|w| w.bits(Self::TOP));
        tc0.tccr0b.write(|w| w.cs0().variant(Self::PRESCALER));

        // Enable Interrupt
        tc0.timsk0.write(|w| w.ocie0a().set_bit());
    }

    pub fn now(&self) -> u32 {
        avr_device::interrupt::free(|cs| self.counter.borrow(cs).get())
    }

    pub fn tick(&self) {
        avr_device::interrupt::free(|cs| {
            let counter = self.counter.borrow(cs);
            counter.set(counter.get().wrapping_add(1));
        });
    }
}
