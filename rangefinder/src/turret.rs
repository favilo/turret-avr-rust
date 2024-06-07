use arduino_hal::{delay_ms, hal::port::PD3, prelude::*};
#[cfg(not(feature = "servo"))]
use arduino_hal::{
    hal::port::PB0,
    port::{
        mode::{Floating, Input, Output},
        Pin,
    },
};

#[cfg(not(feature = "servo"))]
use uom::si::{f32::TemperatureInterval, temperature_interval::degree_celsius};

use crate::{
    hc_sr04::HcSr04,
    ir::{self, fetch_message},
    Serial,
};
#[cfg(not(feature = "servo"))]
use arduino_sys::Servo;

#[cfg(feature = "servo")]
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

#[cfg(feature = "servo")]
mod builder;

#[cfg(feature = "servo")]
#[derive(Debug)]
pub struct Turret {
    /// Yaw Servo Motor (Horizontal))
    yaw: Servo<ServoAttached>,
    /// Pitch Servo Motor (Vertical)
    pitch: Servo<ServoAttached>,
    /// Roll Servo Motor (Fire)
    roll: Servo<ServoAttached>,

    /// Keep track of the current pitch value,
    /// so we don't go too far.
    pitch_value: i16,

    #[allow(unused)]
    range_finder: HcSr04<PD3>,
}

#[cfg(not(feature = "servo"))]
#[derive(Debug)]
pub struct Turret {
    /// Yaw Servo Motor (Horizontal))
    yaw: Servo,
    /// Pitch Servo Motor (Vertical)
    pitch: Servo,
    /// Roll Servo Motor (Fire)
    roll: Servo,

    /// Keep track of the current pitch value,
    /// so we don't go too far.
    pitch_value: i16,

    #[allow(unused)]
    range_finder: HcSr04<PD3>,
}

impl Turret {
    #[cfg(feature = "servo")]
    pub fn builder(
    ) -> builder::Builder<builder::NoYaw, builder::NoPitch, builder::NoRoll, builder::NoRangeFinder>
    {
        builder::Builder::default()
    }

    #[cfg(not(feature = "servo"))]
    pub fn new(d8: Pin<Output, PB0>, d3: Pin<Input<Floating>, PD3>) -> Self {
        let yaw = unsafe { Servo::new() };
        let pitch = unsafe { Servo::new() };
        let roll = unsafe { Servo::new() };

        let range_finder = HcSr04::new(TemperatureInterval::new::<degree_celsius>(23.0), d8, d3);

        Self {
            yaw,
            pitch,
            roll,

            pitch_value: 100,
            range_finder,
        }
    }

    #[cfg(not(feature = "servo"))]
    pub fn attach(&mut self) {
        unsafe { self.yaw.attach(10) };
        unsafe { self.pitch.attach(11) };
        unsafe { self.roll.attach(12) };
    }

    pub fn move_up(&mut self, moves: u32, #[cfg(feature = "servo")] serial: &mut Serial) {
        for _ in 0..moves {
            if self.pitch_value > PITCH_MIN {
                self.pitch_value -= PITCH_MOVE_SPEED;

                #[cfg(feature = "servo")]
                self.pitch.write(self.pitch_value as u8, serial);

                #[cfg(not(feature = "servo"))]
                unsafe {
                    self.pitch.write(self.pitch_value)
                };
                delay_ms(50);
            }
        }
    }

    pub fn move_down(&mut self, moves: u32, #[cfg(feature = "servo")] serial: &mut Serial) {
        for _ in 0..moves {
            if self.pitch_value < PITCH_MAX {
                self.pitch_value += PITCH_MOVE_SPEED;

                #[cfg(feature = "servo")]
                self.pitch.write(self.pitch_value as u8, serial);

                #[cfg(not(feature = "servo"))]
                unsafe {
                    self.pitch.write(self.pitch_value)
                };
                delay_ms(50);
            }
        }
    }

    pub fn move_left(&mut self, moves: u32, #[cfg(feature = "servo")] serial: &mut Serial) {
        for _ in 0..moves {
            #[cfg(feature = "servo")]
            self.yaw
                .write((YAW_STOP_SPEED + YAW_MOVE_SPEED) as u8, serial);

            #[cfg(not(feature = "servo"))]
            unsafe {
                self.yaw.write(YAW_STOP_SPEED + YAW_MOVE_SPEED)
            };
            delay_ms(YAW_PRECISION);

            #[cfg(feature = "servo")]
            self.yaw.write(YAW_STOP_SPEED as u8, serial);

            #[cfg(not(feature = "servo"))]
            unsafe {
                self.yaw.write(YAW_STOP_SPEED)
            };

            delay_ms(5);
        }
    }

    pub fn move_right(&mut self, moves: u32, #[cfg(feature = "servo")] serial: &mut Serial) {
        for _ in 0..moves {
            #[cfg(feature = "servo")]
            self.roll
                .write((YAW_STOP_SPEED - YAW_MOVE_SPEED) as u8, serial);

            #[cfg(not(feature = "servo"))]
            unsafe {
                self.yaw.write(YAW_STOP_SPEED - YAW_MOVE_SPEED)
            };

            delay_ms(YAW_PRECISION);

            #[cfg(feature = "servo")]
            self.roll.write(YAW_STOP_SPEED as u8, serial);

            #[cfg(not(feature = "servo"))]
            unsafe {
                self.yaw.write(YAW_STOP_SPEED)
            };

            delay_ms(5);
        }
    }

    pub fn fire(&mut self, #[cfg(feature = "servo")] serial: &mut Serial) {
        #[cfg(feature = "servo")]
        self.roll
            .write((ROLL_STOP_SPEED - ROLL_MOVE_SPEED) as u8, serial);

        #[cfg(not(feature = "servo"))]
        unsafe {
            self.roll.write(ROLL_STOP_SPEED - ROLL_MOVE_SPEED)
        };

        delay_ms(ROLL_PRECISION);

        #[cfg(feature = "servo")]
        self.roll.write(ROLL_STOP_SPEED as u8, serial);

        #[cfg(not(feature = "servo"))]
        unsafe {
            self.roll.write(ROLL_STOP_SPEED)
        };
        delay_ms(5);
    }

    pub fn fire_all(&mut self, #[cfg(feature = "servo")] serial: &mut Serial) {
        #[cfg(feature = "servo")]
        self.roll
            .write((ROLL_STOP_SPEED - ROLL_MOVE_SPEED) as u8, serial);

        #[cfg(not(feature = "servo"))]
        unsafe {
            self.roll.write(ROLL_STOP_SPEED - ROLL_MOVE_SPEED)
        };

        delay_ms(ROLL_PRECISION * 6);

        #[cfg(feature = "servo")]
        self.roll.write(ROLL_STOP_SPEED as u8, serial);

        #[cfg(not(feature = "servo"))]
        unsafe {
            self.roll.write(ROLL_STOP_SPEED)
        };
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
                    self.move_up(
                        1,
                        #[cfg(feature = "servo")]
                        serial,
                    );
                }
                ir::DOWN => {
                    ufmt::uwriteln!(serial, "DOWN").unwrap_infallible();
                    self.move_down(
                        1,
                        #[cfg(feature = "servo")]
                        serial,
                    );
                }
                ir::LEFT => {
                    ufmt::uwriteln!(serial, "LEFT").unwrap_infallible();
                    self.move_left(
                        1,
                        #[cfg(feature = "servo")]
                        serial,
                    );
                }
                ir::RIGHT => {
                    ufmt::uwriteln!(serial, "RIGHT").unwrap_infallible();
                    self.move_right(
                        1,
                        #[cfg(feature = "servo")]
                        serial,
                    );
                }
                ir::OK => {
                    if !cmd.repeat {
                        self.fire(
                            #[cfg(feature = "servo")]
                            serial,
                        );
                        ufmt::uwriteln!(serial, "FIRE").unwrap_infallible();
                    } else {
                        ufmt::uwriteln!(serial, "Too soon").unwrap_infallible();
                    }
                }
                ir::STAR => {
                    if !cmd.repeat {
                        ufmt::uwriteln!(serial, "BLASTOFF").unwrap_infallible();
                        self.fire_all(
                            #[cfg(feature = "servo")]
                            serial,
                        );
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
