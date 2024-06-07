use core::cell::RefCell;

use arduino_hal::{
    hal::port::Dynamic,
    pac::TC1,
    port::{mode::Output, Pin, PinOps},
    prelude::_unwrap_infallible_UnwrapInfallible,
};
use avr_device::interrupt::Mutex;
use heapless::Vec;
use vcell::VolatileCell;

use crate::Serial;

const MAX_SERVOS: usize = 12;
const REFRESH_INTERVAL: u16 = 20_000;
const MIN_PULSE_WIDTH: i16 = 544;
const MAX_PULSE_WIDTH: i16 = 2400;
const DEFAULT_PULSE_WIDTH: u16 = 1500;
// compensation ticks to trim adjust for digitalWrite delays // 12 August 2009
const TRIM_DURATION: i16 = 0;

// Don't judge me, I want to be correct to the Arduino definition
const CLOCK_CYCLES_PER_MICROSECOND: u32 = 16_000_000 / 1_000_000;

static SERVOS: Mutex<RefCell<Vec<ServoInternal, MAX_SERVOS>>> =
    Mutex::new(RefCell::new(Vec::new()));
static mut TC1: Option<TC1> = None;
static CHANNEL: Mutex<VolatileCell<i8>> = Mutex::new(VolatileCell::new(0));

#[derive(Debug)]
pub enum ServoError {
    NotInitialized,
    TooManyServos,
}

struct ServoInternal {
    pin: Pin<Output>,
    ticks: VolatileCell<u16>,
    attached: bool,
}

#[derive(Debug)]
pub struct ServoAttached;
#[derive(Debug)]
pub struct ServoDetached;

#[derive(Debug)]
pub struct Servo<State> {
    index: usize,
    min: i16,
    max: i16,
    _phantom: core::marker::PhantomData<State>,
}

impl<State> Servo<State> {
    /// Create a new servo on the given pin
    pub fn new<PIN: PinOps<Dynamic = Dynamic>>(
        pin: Pin<Output, PIN>,
    ) -> Result<Servo<ServoDetached>, ServoError> {
        if unsafe { TC1.is_none() } {
            return Err(ServoError::NotInitialized);
        }
        avr_device::interrupt::free(|cs| {
            let mut servos = SERVOS.borrow(cs).borrow_mut();
            let index = servos.len();
            servos
                .push(ServoInternal {
                    pin: pin.downgrade(),
                    ticks: VolatileCell::new(DEFAULT_PULSE_WIDTH),
                    attached: false,
                })
                .map_err(|_| ServoError::TooManyServos)?;
            Ok(Servo {
                index,
                min: 0,
                max: 0,
                _phantom: core::marker::PhantomData,
            })
        })
    }

    fn is_timer_active() -> bool {
        avr_device::interrupt::free(|cs| {
            let servos = SERVOS.borrow(cs).borrow();
            servos.iter().any(|s| s.attached)
        })
    }

    /// Copied from
    /// [Servo.h](https://github.com/arduino-libraries/Servo/blob/85e8cdd3b1dc26402b3529f86955830b47e19df6/src/avr/Servo.cpp#L126-L138)]
    fn init_timer() {
        // Safety: TC1 must be initialized before calling Servo::new()
        let tc1 = unsafe { TC1.as_ref().unwrap() };
        // TCCR1A = 0;             // normal counting mode
        tc1.tccr1a.write(|w| w.wgm1().bits(0));
        // TCCR1B = _BV(CS11);     // set prescaler of 8
        tc1.tccr1b.write(|w| w.cs1().prescale_8());
        // TCNT1 = 0;              // clear the timer count
        tc1.tcnt1.write(|w| w.bits(0));
        // TIFR1 |= _BV(OCF1A);     // clear any pending interrupts
        tc1.tifr1.write(|w| w.ocf1a().set_bit());
        // TIMSK1 |=  _BV(OCIE1A) ; // enable the output compare interrupt
        tc1.timsk1.write(|w| w.ocie1a().set_bit());
    }

    #[allow(dead_code)]
    fn disable_timer() {
        // Safety: TC1 must be initialized before calling Servo::new()
        let tc1 = unsafe { TC1.as_ref().unwrap() };
        tc1.timsk1.write(|w| w.ocie1a().clear_bit());
    }

    fn servo_min(&self) -> i16 {
        (MIN_PULSE_WIDTH - self.min * 4) as i16
    }

    fn servo_max(&self) -> i16 {
        (MAX_PULSE_WIDTH - self.max * 4) as i16
    }

    #[allow(dead_code)]
    fn is_attached(&self) -> bool {
        avr_device::interrupt::free(|cs| {
            let servos = SERVOS.borrow(cs).borrow();
            servos[self.index].attached
        })
    }

    #[allow(dead_code)]
    pub fn read(&self) -> u8 {
        map(
            self.read_us() as i16,
            self.servo_min(),
            self.servo_max(),
            0,
            180,
        ) as u8
    }

    #[allow(dead_code)]
    fn read_us(&self) -> u16 {
        let ticks = avr_device::interrupt::free(|cs| {
            let servos = SERVOS.borrow(cs).borrow();
            servos[self.index].ticks.get()
        });
        ticks_to_us(ticks as u32) as u16 + TRIM_DURATION as u16
    }
}

impl Servo<ServoDetached> {
    pub fn attach(self) -> Servo<ServoAttached> {
        self.attach_with_limits(MIN_PULSE_WIDTH, MAX_PULSE_WIDTH)
    }

    /// Attach the servo to the pin
    pub fn attach_with_limits(self, min: i16, max: i16) -> Servo<ServoAttached> {
        let min = ((MIN_PULSE_WIDTH - min) / 4) as i16;
        let max = ((MAX_PULSE_WIDTH - max) / 4) as i16;
        if !Self::is_timer_active() {
            // Start the timer
            Self::init_timer();
        }
        avr_device::interrupt::free(|cs| {
            let mut servos = SERVOS.borrow(cs).borrow_mut();
            let servo = &mut servos[self.index];
            servo.attached = true;
        });
        Servo {
            index: self.index,
            min,
            max,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl Servo<ServoAttached> {
    #[allow(dead_code)]
    pub fn detach(self) -> Servo<ServoDetached> {
        avr_device::interrupt::free(|cs| {
            let mut servos = SERVOS.borrow(cs).borrow_mut();
            let servo = &mut servos[self.index];
            servo.attached = false;
        });

        if Self::is_timer_active() {
            // Stop the timer
            Self::disable_timer();
        }
        Servo {
            index: self.index,
            min: self.max,
            max: self.max,
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn write(&self, value: u8, serial: &mut Serial) {
        ufmt::uwriteln!(serial, "Writing {} in range\r", value).unwrap_infallible();
        let value = value.clamp(0, 180);
        let value = map(value as i16, 0, 180, self.servo_min(), self.servo_max());
        self.write_us(value, serial);
    }

    pub fn write_us(&self, value: i16, serial: &mut Serial) {
        // ensure pulse width is valid
        let value = value.clamp(self.servo_min(), self.servo_max());

        // convert to ticks after compensating for interrupt overhead - 12 Aug 2009
        let value = value - TRIM_DURATION;
        let value = us_to_ticks(value as u32);

        ufmt::uwriteln!(serial, "Writing {} us\r", value).unwrap_infallible();

        avr_device::interrupt::free(|cs| {
            let mut servos = SERVOS.borrow(cs).borrow_mut();
            // This can't panic because the servo was successfully constructed
            let servo = &mut servos[self.index];
            servo.ticks.set(value as u16);
        });
    }
}

pub fn donate_tc1(tc1: TC1) {
    // Safety: TC1 must be initialized before calling Servo::new()
    // So no Servo instances can exist before this function is called
    // So there is no ability to use TC1 before it is donated
    unsafe {
        TC1 = Some(tc1);
    }
}

/// Copied from
/// [Servo.h](https://github.com/arduino-libraries/Servo/blob/85e8cdd3b1dc26402b3529f86955830b47e19df6/src/avr/Servo.cpp#L52-L75)
#[avr_device::interrupt(atmega328p)]
fn TIMER1_COMPA() {
    let tc1 = unsafe { TC1.as_ref().unwrap() };
    avr_device::interrupt::free(|cs| {
        let channel = CHANNEL.borrow(cs);
        // if( Channel[timer] < 0 )
        if channel.get() < 0 {
            //   *TCNTn = 0; // channel set to -1 indicated that refresh interval completed so reset the timer
            unsafe { tc1.tcnt1.write_with_zero(|w| w.bits(0)) };
        } else {
            //   if( SERVO_INDEX(timer,Channel[timer]) < ServoCount && SERVO(timer,Channel[timer]).Pin.isActive == true )
            if let Some(servo) = SERVOS
                .borrow(cs)
                .borrow_mut()
                .get_mut(channel.get() as usize)
            {
                if servo.attached {
                    //     digitalWrite( SERVO(timer,Channel[timer]).Pin.nbr,LOW); // pulse this channel low if activated
                    servo.pin.set_low();
                }
            }
        }
        // Channel[timer]++;    // increment to the next channel
        channel.set(channel.get() + 1);

        // if( SERVO_INDEX(timer,Channel[timer]) < ServoCount && Channel[timer] < SERVOS_PER_TIMER) {
        if let Some(servo) = SERVOS
            .borrow(cs)
            .borrow_mut()
            .get_mut(channel.get() as usize)
        {
            //   *OCRnA = *TCNTn + SERVO(timer,Channel[timer]).ticks;
            tc1.ocr1a
                .write(|w| w.bits(tc1.tcnt1.read().bits() + servo.ticks.get()));
            //   if(SERVO(timer,Channel[timer]).Pin.isActive == true)     // check if activated
            if servo.attached {
                //     digitalWrite( SERVO(timer,Channel[timer]).Pin.nbr,HIGH); // its an active channel so pulse it high
                servo.pin.set_high();
            }
        } else {
            // finished all channels so wait for the refresh period to expire before starting over
            //   if( ((unsigned)*TCNTn) + 4 < usToTicks(REFRESH_INTERVAL) )  // allow a few ticks to ensure the next OCR1A not missed
            if tc1.tcnt1.read().bits() as u32 + 4 > us_to_ticks(REFRESH_INTERVAL as u32) {
                //     *OCRnA = (unsigned int)usToTicks(REFRESH_INTERVAL);
                tc1.ocr1a
                    .write(|w| w.bits(us_to_ticks(REFRESH_INTERVAL as u32) as u16));
            } else {
                //     *OCRnA = *TCNTn + 4;  // at least REFRESH_INTERVAL has elapsed
                tc1.ocr1a.write(|w| w.bits(tc1.tcnt1.read().bits() + 4));
            }
            //   Channel[timer] = -1; // this will get incremented at the end of the refresh period to start again at the first channel
            channel.set(-1);
        }
    });
}

/// Convert microseconds to timer ticks
/// Assumes prescaler of 8
#[inline(always)]
fn us_to_ticks(us: u32) -> u32 {
    (us as u32 * CLOCK_CYCLES_PER_MICROSECOND) / 8
}

fn ticks_to_us(ticks: u32) -> u32 {
    (ticks as u32 * 8) / CLOCK_CYCLES_PER_MICROSECOND
}

/// Re-maps a number from one range to another.
/// That is, a value of fromLow would get mapped to toLow,
/// a value of fromHigh to toHigh, values in-between to values in-between, etc.
fn map(value: i16, from_low: i16, from_high: i16, to_low: i16, to_high: i16) -> i16 {
    (value - from_low) * (to_high - to_low) / (from_high - from_low) + to_low
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arduino::handle_interrupts;

    #[test]
    fn test_timer1_compa() {}
}
