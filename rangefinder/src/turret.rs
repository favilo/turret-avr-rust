use arduino_hal::{delay_ms, hal::port::PD3, prelude::*};

use crate::{
    hc_sr04::HcSr04,
    ir::{self, fetch_message},
    // arduino::Servo,
    servo::{Servo, ServoAttached},
    Serial,
};

pub const PITCH_MOVE_SPEED: u8 = 8;
pub const YAW_MOVE_SPEED: u8 = 90;
pub const YAW_STOP_SPEED: u8 = 90;
pub const ROLL_MOVE_SPEED: u8 = 90;
pub const ROLL_STOP_SPEED: u8 = 90;

pub const YAW_PRECISION: u16 = 75;
pub const ROLL_PRECISION: u16 = 115;

pub const PITCH_MAX: u8 = 175;
pub const PITCH_MIN: u8 = 10;

mod builder {
    use arduino_hal::{
        hal::port::{PB0, PB2, PB3, PB4, PD3},
        port::{
            mode::{Floating, Input, Output},
            Pin,
        },
    };
    use uom::si::{f32::TemperatureInterval, temperature_interval::degree_celsius};

    use crate::{
        hc_sr04::HcSr04,
        servo::{Servo, ServoAttached, ServoDetached, ServoError},
    };

    use super::Turret;

    #[derive(Default)]
    pub struct NoYaw;
    pub struct Yaw(Servo<ServoAttached>);

    #[derive(Default)]
    pub struct NoPitch;
    pub struct Pitch(Servo<ServoAttached>);

    #[derive(Default)]
    pub struct NoRoll;
    pub struct Roll(Servo<ServoAttached>);

    #[derive(Default)]
    pub struct NoRangeFinder;
    pub struct RangeFinder(HcSr04<PD3>);

    #[derive(Default)]
    pub struct Builder<Yaw, Pitch, Roll, RangeFinder> {
        yaw: Yaw,
        pitch: Pitch,
        roll: Roll,

        range_finder: RangeFinder,
    }

    impl<Pitch, Roll, RangeFinder> Builder<NoYaw, Pitch, Roll, RangeFinder> {
        pub fn yaw(
            self,
            pin: Pin<Output, PB2>,
        ) -> Result<Builder<Yaw, Pitch, Roll, RangeFinder>, ServoError> {
            let Self {
                pitch,
                roll,
                range_finder,
                ..
            } = self;
            let servo = Servo::<ServoDetached>::new(pin)?;

            Ok(Builder {
                yaw: Yaw(servo.attach()),
                pitch,
                roll,
                range_finder,
            })
        }
    }

    impl<Yaw, Roll, RangeFinder> Builder<Yaw, NoPitch, Roll, RangeFinder> {
        pub fn pitch(
            self,
            pin: Pin<Output, PB3>,
        ) -> Result<Builder<Yaw, Pitch, Roll, RangeFinder>, ServoError> {
            let Self {
                yaw,
                roll,
                range_finder,
                ..
            } = self;
            let servo = Servo::<ServoDetached>::new(pin)?;

            Ok(Builder {
                yaw,
                pitch: Pitch(servo.attach()),
                roll,
                range_finder,
            })
        }
    }

    impl<Yaw, Pitch, RangeFinder> Builder<Yaw, Pitch, NoRoll, RangeFinder> {
        pub fn roll(
            self,
            pin: Pin<Output, PB4>,
        ) -> Result<Builder<Yaw, Pitch, Roll, RangeFinder>, ServoError> {
            let Self {
                yaw,
                pitch,
                range_finder,
                ..
            } = self;
            let servo = Servo::<ServoDetached>::new(pin)?;
            Ok(Builder {
                yaw,
                pitch,
                roll: Roll(servo.attach()),
                range_finder,
            })
        }
    }

    impl<Yaw, Pitch, Roll> Builder<Yaw, Pitch, Roll, NoRangeFinder> {
        pub fn range_finder(
            self,
            d8: Pin<Output, PB0>,
            d3: Pin<Input<Floating>, PD3>,
        ) -> Builder<Yaw, Pitch, Roll, RangeFinder> {
            let Self {
                yaw, pitch, roll, ..
            } = self;
            let range_finder =
                HcSr04::new(TemperatureInterval::new::<degree_celsius>(23.0), d8, d3);

            Builder {
                yaw,
                pitch,
                roll,
                range_finder: RangeFinder(range_finder),
            }
        }
    }

    impl Builder<Yaw, Pitch, Roll, RangeFinder> {
        pub fn build(self) -> Turret {
            Turret {
                yaw: self.yaw.0,
                pitch: self.pitch.0,
                roll: self.roll.0,

                pitch_value: 100,
                range_finder: self.range_finder.0,
            }
        }
    }
}

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
    pitch_value: u8,

    #[allow(unused)]
    range_finder: HcSr04<PD3>,
}

impl Turret {
    pub fn builder(
    ) -> builder::Builder<builder::NoYaw, builder::NoPitch, builder::NoRoll, builder::NoRangeFinder>
    {
        builder::Builder::default()
    }

    pub fn move_up(&mut self, moves: u32, serial: &mut Serial) {
        for _ in 0..moves {
            if self.pitch_value > PITCH_MIN {
                self.pitch_value -= PITCH_MOVE_SPEED;
                self.pitch.write(self.pitch_value, serial);
                delay_ms(50);
            }
        }
    }

    pub fn move_down(&mut self, moves: u32, serial: &mut Serial) {
        for _ in 0..moves {
            if self.pitch_value < PITCH_MAX {
                self.pitch_value += PITCH_MOVE_SPEED;
                self.pitch.write(self.pitch_value, serial);
                delay_ms(50);
            }
        }
    }

    pub fn move_left(&mut self, moves: u32, serial: &mut Serial) {
        for _ in 0..moves {
            self.yaw.write(YAW_STOP_SPEED + YAW_MOVE_SPEED, serial);
            delay_ms(YAW_PRECISION);
            self.yaw.write(YAW_STOP_SPEED, serial);
            delay_ms(5);
        }
    }

    pub fn move_right(&mut self, moves: u32, serial: &mut Serial) {
        for _ in 0..moves {
            self.yaw.write(YAW_STOP_SPEED - YAW_MOVE_SPEED, serial);
            delay_ms(YAW_PRECISION);
            self.yaw.write(YAW_STOP_SPEED, serial);
            delay_ms(5);
        }
    }

    pub fn fire(&mut self, serial: &mut Serial) {
        self.roll.write(ROLL_STOP_SPEED - ROLL_MOVE_SPEED, serial);
        delay_ms(ROLL_PRECISION);
        self.roll.write(ROLL_STOP_SPEED, serial);
        delay_ms(5);
    }

    pub fn fire_all(&mut self, serial: &mut Serial) {
        self.roll.write(ROLL_STOP_SPEED - ROLL_MOVE_SPEED, serial);
        delay_ms(ROLL_PRECISION * 6);
        self.roll.write(ROLL_STOP_SPEED, serial);
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
