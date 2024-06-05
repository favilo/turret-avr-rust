#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(incomplete_features)]
#![feature(abi_avr_interrupt)]
#![feature(generic_const_exprs)]

use arduino_hal::{prelude::*, Pins, Usart};
use fugit::Duration;
use interrupt::AttachPCInterrupt;
use panic_halt as _;

#[allow(dead_code)]
mod arduino;
mod clock;
mod hc_sr04;
mod interrupt;
mod ir;
mod turret;

use crate::{
    clock::CLOCK,
    ir::{fetch_message, init_receiver},
    turret::Turret,
};

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins: Pins = arduino_hal::pins!(dp);
    let mut serial: Usart<_, _, _> = arduino_hal::default_serial!(dp, pins, 57600);

    // Disable interrupts while we initialize them
    avr_device::interrupt::disable();

    // Monotonic clock to keep track of the time.
    CLOCK.start(dp.TC0);

    pins.d9.attach_pc_int(&dp.EXINT);

    init_receiver(pins.d9);

    // Enable interrupts now that receiver is initialized
    unsafe { avr_device::interrupt::enable() };

    ufmt::uwriteln!(&mut serial, "Ready to receive IR signals").unwrap_infallible();

    let mut range_finder = hc_sr04::HcSr04::new(pins.d8.into_output(), pins.d3);

    let mut turret = Turret::new();
    turret.attach();

    let mut counter = 0;

    loop {
        if let Some(cmd) = fetch_message() {
            // ufmt::uwriteln!(
            //     &mut serial, "Command(Addr: {}, Cmd: {}, Rpt: {})",
            //     cmd.addr,
            //     cmd.cmd,
            //     cmd.repeat
            // )
            // .unwrap_infallible();
            match cmd.cmd {
                ir::UP => {
                    turret.move_up(1);
                    ufmt::uwriteln!(&mut serial, "UP").unwrap_infallible();
                }
                ir::DOWN => {
                    turret.move_down(1);
                    ufmt::uwriteln!(&mut serial, "DOWN").unwrap_infallible();
                }
                ir::LEFT => {
                    turret.move_left(1);
                    ufmt::uwriteln!(&mut serial, "LEFT").unwrap_infallible();
                }
                ir::RIGHT => {
                    turret.move_right(1);
                    ufmt::uwriteln!(&mut serial, "RIGHT").unwrap_infallible();
                }
                ir::OK => {
                    if !cmd.repeat {
                        turret.fire();
                        ufmt::uwriteln!(&mut serial, "FIRE").unwrap_infallible();
                    } else {
                        ufmt::uwriteln!(&mut serial, "Too soon").unwrap_infallible();
                    }
                }
                ir::STAR => {
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

        if counter % 100 == 0 {
            ufmt::uwriteln!(&mut serial, "Clock: {}", CLOCK.now()).unwrap_infallible();
            // TODO: Make range finder work in the background
        }

        counter += 1;
        arduino_hal::delay_ms(5);
    }
}
