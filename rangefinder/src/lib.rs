#![no_std]
#![allow(incomplete_features)]
#![feature(abi_avr_interrupt)]
#![feature(generic_const_exprs)]

use arduino_hal::{
    hal::port::{PD0, PD1},
    pac::USART0,
    port::{
        mode::{Input, Output},
        Pin,
    },
    Usart,
};

// #[allow(dead_code)]
// pub mod arduino;
pub mod clock;
pub mod hc_sr04;
pub mod interrupt;
pub mod ir;
pub mod servo;
pub mod turret;

pub type Serial = Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>>;
