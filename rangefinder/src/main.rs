#![no_std]
#![no_main]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(incomplete_features)]
#![feature(abi_avr_interrupt)]
#![feature(generic_const_exprs)]

use arduino_hal::{prelude::*, Pins, Usart};
use panic_halt as _;

use rangefinder::{
    clock::CLOCK, interrupt::AttachPCInterrupt, ir::init_receiver, servo, turret::Turret,
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

    servo::donate_tc1(dp.TC1);
    let mut turret = Turret::builder()
        .range_finder(pins.d8.into_output(), pins.d3)
        .yaw(pins.d10.into_output())
        .expect("Failed to initialize yaw servo")
        .pitch(pins.d11.into_output())
        .expect("Failed to initialize pitch servo")
        .roll(pins.d12.into_output())
        .expect("Failed to initialize roll servo")
        .build();

    // Enable interrupts now that receiver is initialized
    unsafe { avr_device::interrupt::enable() };

    ufmt::uwriteln!(&mut serial, "Ready to receive IR signals").unwrap_infallible();

    let mut counter = 0;

    loop {
        turret.handle_command(&mut serial);

        // TODO: Move this into turret code, and search for target after a specific button is
        // pressed
        //
        if counter % 100 == 0 {
            ufmt::uwriteln!(&mut serial, "Clock: {}", CLOCK.now()).unwrap_infallible();
            // ufmt::uwriteln!(&mut serial, "Measuring time").unwrap_infallible();
            // let distance = turret.range_finder_mut().measure_distance(&dp.EXINT);
            // if let Ok(distance) = distance {
            //     if distance > Length::new::<meter>(1.0) {
            //         ufmt::uwriteln!(
            //             &mut serial,
            //             "Distance: {} m",
            //             uFmt_f32::Two(distance.get::<meter>())
            //         )
            //         .unwrap_infallible();
            //     } else {
            //         ufmt::uwriteln!(
            //             &mut serial,
            //             "Distance: {} cm",
            //             uFmt_f32::Two(distance.get::<centimeter>())
            //         )
            //         .unwrap_infallible();
            //     }
            // } else {
            //     ufmt::uwriteln!(&mut serial, "Error: {:?}", distance.unwrap_err())
            //         .unwrap_infallible();
            // }
        }

        counter += 1;
        arduino_hal::delay_ms(5);
    }
}
