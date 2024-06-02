#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![feature(abi_avr_interrupt)]

use core::ffi::c_int;

use arduino_hal::{delay_ms, prelude::*};
use infrared::Receiver;
use panic_halt as _;

#[allow(dead_code)]
mod arduino;
use arduino::Servo;

mod ir;
use ir::{fetch_message, replace_receiver, Clock, CLOCK};

const LEFT: u8 = 0x8;
const RIGHT: u8 = 0x5A;
const UP: u8 = 0x52;
const DOWN: u8 = 0x18;
const OK: u8 = 0x1C;
const CMD1: u8 = 0x45;
const CMD2: u8 = 0x46;
const CMD3: u8 = 0x47;
const CMD4: u8 = 0x44;
const CMD5: u8 = 0x40;
const CMD6: u8 = 0x43;
const CMD7: u8 = 0x7;
const CMD8: u8 = 0x15;
const CMD9: u8 = 0x9;
const CMD0: u8 = 0x19;
const STAR: u8 = 0x16;
const HASHTAG: u8 = 0xD;

const PITCH_MOVE_SPEED: c_int = 8;
const YAW_MOVE_SPEED: c_int = 90;
const YAW_STOP_SPEED: c_int = 90;
const ROLL_MOVE_SPEED: c_int = 90;
const ROLL_STOP_SPEED: c_int = 90;

const YAW_PRECISION: u16 = 75;
const ROLL_PRECISION: u16 = 158;

const PITCH_MAX: i16 = 175;
const PITCH_MIN: i16 = 10;

#[derive(Debug)]
struct Turret {
    yaw: Servo,
    pitch: Servo,
    roll: Servo,

    // yaw_value: i32,
    pitch_value: i16,
    // roll_value: i32,
}

impl Turret {
    pub fn new() -> Self {
        let yaw = unsafe { Servo::new() };
        let pitch = unsafe { Servo::new() };
        let roll = unsafe { Servo::new() };

        Self {
            yaw,
            pitch,
            roll,

            // yaw_value: 90,
            pitch_value: 100,
            // roll_value: 90,
        }
    }

    pub fn attach(&mut self) {
        unsafe { self.yaw.attach(10) };
        unsafe { self.pitch.attach(11) };
        unsafe { self.roll.attach(12) };
    }

    pub fn move_up(&mut self, moves: u32) {
        for _ in 0..moves {
            if self.pitch_value > PITCH_MIN {
                self.pitch_value -= PITCH_MOVE_SPEED;
                unsafe { self.pitch.write(self.pitch_value) };
                delay_ms(50);
            }
        }
    }

    pub fn move_down(&mut self, moves: u32) {
        for _ in 0..moves {
            if self.pitch_value < PITCH_MAX {
                self.pitch_value += PITCH_MOVE_SPEED;
                unsafe { self.pitch.write(self.pitch_value) };
                delay_ms(50);
            }
        }
    }

    pub fn move_left(&mut self, moves: u32) {
        for _ in 0..moves {
            unsafe { self.yaw.write(YAW_STOP_SPEED + YAW_MOVE_SPEED) };
            delay_ms(YAW_PRECISION);
            unsafe { self.yaw.write(YAW_STOP_SPEED) };
            delay_ms(5);
        }
    }

    pub fn move_right(&mut self, moves: u32) {
        for _ in 0..moves {
            unsafe { self.yaw.write(YAW_STOP_SPEED - YAW_MOVE_SPEED) };
            delay_ms(YAW_PRECISION);
            unsafe { self.yaw.write(YAW_STOP_SPEED) };
            delay_ms(5);
        }
    }

    pub fn fire(&mut self) {
        unsafe { self.roll.write(ROLL_STOP_SPEED - ROLL_MOVE_SPEED) };
        delay_ms(ROLL_PRECISION);
        unsafe { self.roll.write(ROLL_STOP_SPEED) };
        delay_ms(5);
    }

    pub fn fire_all(&mut self) {
        unsafe { self.roll.write(ROLL_STOP_SPEED - ROLL_MOVE_SPEED) };
        delay_ms(ROLL_PRECISION * 6);
        unsafe { self.roll.write(ROLL_STOP_SPEED) };
        delay_ms(5);
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut turret = Turret::new();
    turret.attach();

    // Monotonic clock to keep track of the time.
    CLOCK.start(dp.TC0);

    // INFO: see https://thewanderingengineer.com/2014/08/11/arduino-pin-change-interrupts/
    //
    // Enable PORTB
    dp.EXINT.pcicr.write(|w| unsafe { w.bits(0b001) });

    // Enable pin change interrupts on PCINT1 which is pin PB1 (=d9)
    dp.EXINT.pcmsk0.write(|w| w.bits(0b010));

    let ir = Receiver::with_pin(Clock::FREQ, pins.d9);
    replace_receiver(ir);

    // Enable interrupts now that receiver is initialized
    unsafe { avr_device::interrupt::enable() };

    ufmt::uwriteln!(&mut serial, "Ready to receive IR signals").unwrap_infallible();

    loop {
        if let Some(cmd) = fetch_message() {
            ufmt::uwriteln!(
                &mut serial,
                "Command(Addr: {}, Cmd: {}, Rpt: {})",
                cmd.addr,
                cmd.cmd,
                cmd.repeat
            )
            .unwrap_infallible();
            match cmd.cmd {
                UP => {
                    turret.move_up(1);
                    ufmt::uwriteln!(&mut serial, "UP").unwrap_infallible();
                }
                DOWN => {
                    turret.move_down(1);
                    ufmt::uwriteln!(&mut serial, "DOWN").unwrap_infallible();
                }
                LEFT => {
                    turret.move_left(1);
                    ufmt::uwriteln!(&mut serial, "LEFT").unwrap_infallible();
                }
                RIGHT => {
                    turret.move_right(1);
                    ufmt::uwriteln!(&mut serial, "RIGHT").unwrap_infallible();
                }
                OK => {
                    if !cmd.repeat {
                        turret.fire();
                        ufmt::uwriteln!(&mut serial, "FIRE").unwrap_infallible();
                    } else {
                        ufmt::uwriteln!(&mut serial, "Too soon").unwrap_infallible();
                    }
                }
                STAR => {
                    if !cmd.repeat {
                        turret.fire_all();
                        ufmt::uwriteln!(&mut serial, "BLASTOFF").unwrap_infallible();
                    }
                }
                _ => {
                    ufmt::uwriteln!(&mut serial, "Unknown").unwrap_infallible();
                }
            };
        } else {
            // ufmt::uwriteln!(&mut serial, "No command").unwrap_infallible();
        }
        arduino_hal::delay_ms(100);
    }
}
