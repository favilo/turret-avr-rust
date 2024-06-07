use core::{
    cell::Cell,
    sync::atomic::{AtomicU8, Ordering},
};

use arduino_hal::pac::{tc0::tccr0b::CS0_A, TC0};
use avr_device::interrupt::Mutex;
use const_assert::{Assert, IsTrue};

pub static CLOCK: Clock<40, 8> = Clock::new();

const fn prescale_from_value<const PRESCALE: u32>() -> CS0_A {
    match PRESCALE {
        0 => CS0_A::NO_CLOCK,
        1 => CS0_A::DIRECT,
        8 => CS0_A::PRESCALE_8,
        64 => CS0_A::PRESCALE_64,
        256 => CS0_A::PRESCALE_256,
        1024 => CS0_A::PRESCALE_1024,
        _ => panic!("Invalid prescale value"),
    }
}

#[allow(dead_code)]
const fn prescale_value(prescale: CS0_A) -> u32 {
    match prescale {
        CS0_A::DIRECT => 1,
        CS0_A::PRESCALE_8 => 8,
        CS0_A::PRESCALE_64 => 64,
        CS0_A::PRESCALE_256 => 256,
        CS0_A::PRESCALE_1024 => 1024,
        _ => 0,
    }
}

/// Clock that ticks at 200kHz
///
/// interrupt frequency (Hz) = (16,000,000Hz) / (prescaler * (compare match register + 1))
/// TOP = [ 16MHz / (PRESCALER * FREQ)] - 1
pub struct Clock<const KHZ: u32, const PRESCALE: u32> {
    part: AtomicU8,
    counter: Mutex<Cell<u32>>,
}

impl<const KHZ: u32, const PRESCALE: u32> Clock<KHZ, PRESCALE>
where
    // Assert, at compile time, this fits into a u8
    Assert<{ (16_000_000 / (PRESCALE * KHZ * 1_000)) - 1 < 256 }>: IsTrue,
{
    pub const FREQ: u32 = KHZ * 1_000;
    const TOP: u8 = ((16_000_000 / (PRESCALE * Self::FREQ)) - 1) as u8;

    pub const fn new() -> Self {
        Self {
            part: AtomicU8::new(0),
            counter: Mutex::new(Cell::new(0)),
        }
    }

    pub fn start(&self, tc0: TC0) {
        // Configure the timer for the above interval (in CTC mode)
        tc0.tccr0a.write(|w| w.wgm0().ctc());
        tc0.ocr0a.write(|w| w.bits(Self::TOP));
        tc0.tccr0b
            .write(|w| w.cs0().variant(prescale_from_value::<PRESCALE>()));

        // Enable Interrupt
        tc0.timsk0.write(|w| w.ocie0a().set_bit());
    }

    pub fn now(&self) -> u32 {
        avr_device::interrupt::free(|cs| {
            self.counter.borrow(cs).get() + self.part.load(Ordering::SeqCst) as u32
        })
    }

    pub fn now_instant(&self) -> fugit::Instant<u32, 1, { KHZ * 1_000 }> {
        fugit::Instant::<u32, 1, { KHZ * 1_000 }>::from_ticks(self.now())
    }

    pub fn tick(&self) {
        avr_device::interrupt::free(|cs| {
            let part = self.part.load(Ordering::SeqCst);
            if part > 250 {
                self.part.store(0, Ordering::SeqCst);
                let counter = self.counter.borrow(cs);
                counter.set(counter.get().wrapping_add(part as u32));
            } else {
                self.part.store(part + 1, Ordering::SeqCst);
            }
        });
    }
}

#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    CLOCK.tick();
}
