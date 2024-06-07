use core::cell::Cell;

use arduino_hal::{
    hal::port::PB1,
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

use crate::clock::{Clock, CLOCK};

pub const LEFT: u8 = 0x8;
pub const RIGHT: u8 = 0x5A;
pub const UP: u8 = 0x52;
pub const DOWN: u8 = 0x18;
pub const OK: u8 = 0x1C;
#[allow(dead_code)]
pub const CMD1: u8 = 0x45;
#[allow(dead_code)]
pub const CMD2: u8 = 0x46;
#[allow(dead_code)]
pub const CMD3: u8 = 0x47;
#[allow(dead_code)]
pub const CMD4: u8 = 0x44;
#[allow(dead_code)]
pub const CMD5: u8 = 0x40;
#[allow(dead_code)]
pub const CMD6: u8 = 0x43;
#[allow(dead_code)]
pub const CMD7: u8 = 0x7;
#[allow(dead_code)]
pub const CMD8: u8 = 0x15;
#[allow(dead_code)]
pub const CMD9: u8 = 0x9;
#[allow(dead_code)]
pub const CMD0: u8 = 0x19;
pub const STAR: u8 = 0x16;
#[allow(dead_code)]
pub const HASHTAG: u8 = 0xD;

type IRPin = Pin<Input<Floating>, PB1>;

static mut RECEIVER: Option<Receiver<Nec, IRPin, u32, NecCommand>> = None;
static CMD: Mutex<Cell<Option<NecCommand>>> = Mutex::new(Cell::new(None));

#[avr_device::interrupt(atmega328p)]
fn PCINT0() {
    let recv = unsafe { RECEIVER.as_mut().unwrap() };

    // NOTE: Clock frequency is 10x the speed of what Receiver expects;
    // ensure we divide by 2
    let now = CLOCK.now() >> 1;

    let event_instant = recv.event_instant(now).expect("Pin::Error is `Infallible`");
    if let Some(cmd) = event_instant {
        avr_device::interrupt::free(|cs| {
            let cmd_cell = CMD.borrow(cs);
            cmd_cell.set(Some(cmd));
        });
    }
}

pub fn fetch_message() -> Option<NecCommand> {
    avr_device::interrupt::free(|cs| CMD.borrow(cs).take())
}

fn replace_receiver(receiver: Receiver<Nec, Pin<Input<Floating>, PB1>, u32, NecCommand>) {
    unsafe { RECEIVER.replace(receiver) };
}

pub fn init_receiver(pin: Pin<Input<Floating>, PB1>) {
    let receiver = Receiver::with_pin(Clock::<20, 8>::FREQ, pin);
    replace_receiver(receiver);
}
