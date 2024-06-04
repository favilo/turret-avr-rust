use core::ffi::c_int;

use arduino_hal::delay_ms;

use crate::arduino::Servo;


pub const PITCH_MOVE_SPEED: c_int = 8;
pub const YAW_MOVE_SPEED: c_int = 90;
pub const YAW_STOP_SPEED: c_int = 90;
pub const ROLL_MOVE_SPEED: c_int = 90;
pub const ROLL_STOP_SPEED: c_int = 90;

pub const YAW_PRECISION: u16 = 75;
pub const ROLL_PRECISION: u16 = 158;

pub const PITCH_MAX: i16 = 175;
pub const PITCH_MIN: i16 = 10;

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

            pitch_value: 100,
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
