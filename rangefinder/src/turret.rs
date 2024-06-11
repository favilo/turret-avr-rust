use arduino_hal::{delay_ms, hal::port::PD3, prelude::*};
use arduino_hal::{
    hal::port::PB0,
    port::{
        mode::{Floating, Input, Output},
        Pin,
    },
};

use uom::si::{f32::TemperatureInterval, temperature_interval::degree_celsius};

use crate::{
    hc_sr04::HcSr04,
    ir::{self, fetch_message},
    Serial,
};

use crate::servo::{Servo, ServoAttached};

pub const PITCH_MOVE_SPEED: i16 = 8;
pub const YAW_MOVE_SPEED: i16 = 90;
pub const YAW_STOP_SPEED: i16 = 90;
pub const ROLL_MOVE_SPEED: i16 = 90;
pub const ROLL_STOP_SPEED: i16 = 90;

pub const YAW_PRECISION: u16 = 75;
pub const ROLL_PRECISION: u16 = 115;

pub const PITCH_MAX: i16 = 175;
pub const PITCH_MIN: i16 = 10;

mod builder;

#[derive(Debug)]
pub struct Turret<SERVO> {
    /// Yaw Servo Motor (Horizontal))
    yaw: SERVO,
    /// Pitch Servo Motor (Vertical)
    pitch: SERVO,
    /// Roll Servo Motor (Fire)
    roll: SERVO,

    /// Keep track of the current pitch value,
    /// so we don't go too far.
    pitch_value: i16,

    #[allow(unused)]
    range_finder: HcSr04<PD3>,
}

impl Turret<Servo<ServoAttached>> {
    // #[cfg(feature = "servo")]
    pub fn builder(
    ) -> builder::Builder<builder::NoYaw, builder::NoPitch, builder::NoRoll, builder::NoRangeFinder>
    {
        builder::Builder::default()
    }

    // #[cfg(not(feature = "rust_timer1_compa"))]
    pub fn ensure_interrupts() {
        let mut servo = unsafe { arduino_sys::Servo::new() };
        unsafe { servo.attach(1) };
        unreachable!("This should never be called")
    }
}

impl Turret<arduino_sys::Servo> {
    pub fn new(d8: Pin<Output, PB0>, d3: Pin<Input<Floating>, PD3>) -> Self {
        let yaw = unsafe { arduino_sys::Servo::new() };
        let pitch = unsafe { arduino_sys::Servo::new() };
        let roll = unsafe { arduino_sys::Servo::new() };

        let range_finder = HcSr04::new(TemperatureInterval::new::<degree_celsius>(23.0), d8, d3);

        Self {
            yaw,
            pitch,
            roll,

            pitch_value: 100,
            range_finder,
        }
    }

    pub fn attach(&mut self) {
        unsafe { self.yaw.attach(10) };
        unsafe { self.pitch.attach(11) };
        unsafe { self.roll.attach(12) };
    }
}

impl Turret<Servo<ServoAttached>> {
    pub fn move_up(&mut self, moves: u32, serial: &mut Serial) {
        for _ in 0..moves {
            if self.pitch_value > PITCH_MIN {
                self.pitch_value -= PITCH_MOVE_SPEED;

                self.pitch.write(self.pitch_value as u8, serial);

                delay_ms(50);
            }
        }
    }

    pub fn move_down(&mut self, moves: u32, serial: &mut Serial) {
        for _ in 0..moves {
            if self.pitch_value < PITCH_MAX {
                self.pitch_value += PITCH_MOVE_SPEED;

                self.pitch.write(self.pitch_value as u8, serial);

                delay_ms(50);
            }
        }
    }

    pub fn move_left(&mut self, moves: u32, serial: &mut Serial) {
        for _ in 0..moves {
            self.yaw
                .write((YAW_STOP_SPEED + YAW_MOVE_SPEED) as u8, serial);
            delay_ms(YAW_PRECISION);

            self.yaw.write(YAW_STOP_SPEED as u8, serial);

            delay_ms(5);
        }
    }

    pub fn move_right(&mut self, moves: u32, serial: &mut Serial) {
        for _ in 0..moves {
            self.roll
                .write((YAW_STOP_SPEED - YAW_MOVE_SPEED) as u8, serial);

            delay_ms(YAW_PRECISION);

            self.roll.write(YAW_STOP_SPEED as u8, serial);

            delay_ms(5);
        }
    }

    pub fn fire(&mut self, serial: &mut Serial) {
        self.roll
            .write((ROLL_STOP_SPEED - ROLL_MOVE_SPEED) as u8, serial);

        delay_ms(ROLL_PRECISION);

        self.roll.write(ROLL_STOP_SPEED as u8, serial);

        delay_ms(5);
    }

    pub fn fire_all(&mut self, serial: &mut Serial) {
        self.roll
            .write((ROLL_STOP_SPEED - ROLL_MOVE_SPEED) as u8, serial);

        delay_ms(ROLL_PRECISION * 6);

        self.roll.write(ROLL_STOP_SPEED as u8, serial);

        delay_ms(5);
    }

    pub fn handle_command(&mut self, serial: &mut Serial) {
        if let Some(cmd) = fetch_message() {
            ufmt::uwriteln!(
                serial,
                "Command(Addr: {}, Cmd: {}, Rpt: {})",
                cmd.addr,
                cmd.cmd,
                cmd.repeat
            )
            .unwrap_infallible();
            match cmd.cmd {
                ir::UP => {
                    ufmt::uwriteln!(serial, "UP").unwrap_infallible();
                    self.move_up(1, serial);
                }
                ir::DOWN => {
                    ufmt::uwriteln!(serial, "DOWN").unwrap_infallible();
                    self.move_down(1, serial);
                }
                ir::LEFT => {
                    ufmt::uwriteln!(serial, "LEFT").unwrap_infallible();
                    self.move_left(1, serial);
                }
                ir::RIGHT => {
                    ufmt::uwriteln!(serial, "RIGHT").unwrap_infallible();
                    self.move_right(1, serial);
                }
                ir::OK => {
                    if !cmd.repeat {
                        self.fire(serial);
                        ufmt::uwriteln!(serial, "FIRE").unwrap_infallible();
                    } else {
                        ufmt::uwriteln!(serial, "Too soon").unwrap_infallible();
                    }
                }
                ir::STAR => {
                    if !cmd.repeat {
                        ufmt::uwriteln!(serial, "BLASTOFF").unwrap_infallible();
                        self.fire_all(serial);
                    }
                }
                _ => {
                    ufmt::uwriteln!(serial, "Unknown").unwrap_infallible();
                }
            };
        }
    }

    #[allow(dead_code)]
    pub fn range_finder(&self) -> &HcSr04<PD3> {
        &self.range_finder
    }

    #[allow(dead_code)]
    pub fn range_finder_mut(&mut self) -> &mut HcSr04<PD3> {
        &mut self.range_finder
    }
}

impl Turret<arduino_sys::Servo> {
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

    pub fn handle_command(&mut self, serial: &mut Serial) {
        if let Some(cmd) = fetch_message() {
            ufmt::uwriteln!(
                serial,
                "Command(Addr: {}, Cmd: {}, Rpt: {})",
                cmd.addr,
                cmd.cmd,
                cmd.repeat
            )
            .unwrap_infallible();
            match cmd.cmd {
                ir::UP => {
                    ufmt::uwriteln!(serial, "UP").unwrap_infallible();
                    self.move_up(1);
                }
                ir::DOWN => {
                    ufmt::uwriteln!(serial, "DOWN").unwrap_infallible();
                    self.move_down(1);
                }
                ir::LEFT => {
                    ufmt::uwriteln!(serial, "LEFT").unwrap_infallible();
                    self.move_left(1);
                }
                ir::RIGHT => {
                    ufmt::uwriteln!(serial, "RIGHT").unwrap_infallible();
                    self.move_right(1);
                }
                ir::OK => {
                    if !cmd.repeat {
                        self.fire();
                        ufmt::uwriteln!(serial, "FIRE").unwrap_infallible();
                    } else {
                        ufmt::uwriteln!(serial, "Too soon").unwrap_infallible();
                    }
                }
                ir::STAR => {
                    if !cmd.repeat {
                        ufmt::uwriteln!(serial, "BLASTOFF").unwrap_infallible();
                        self.fire_all();
                    }
                }
                _ => {
                    ufmt::uwriteln!(serial, "Unknown").unwrap_infallible();
                }
            };
        }
    }

    #[allow(dead_code)]
    pub fn range_finder(&self) -> &HcSr04<PD3> {
        &self.range_finder
    }

    #[allow(dead_code)]
    pub fn range_finder_mut(&mut self) -> &mut HcSr04<PD3> {
        &mut self.range_finder
    }
}
