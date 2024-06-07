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
        let range_finder = HcSr04::new(TemperatureInterval::new::<degree_celsius>(23.0), d8, d3);

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
