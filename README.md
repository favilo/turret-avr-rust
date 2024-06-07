rangefinder-avr
===============

Rust project for the [Infrared Turret](https://www.crunchlabs.com/products/ir-turret) from 
the CrunchLabs HackPack

## Build Instructions
NOTE: I've only tested this on Linux, so I've no idea what changes are needed for Windows or MacOS.

### Hardware Changes
In order to get the HC-SR04 connected, you need to attach the `Trig` pin to `D8`, and the `Echo` pin to `D3`.
And of course `Vcc` and `Gnd` go to the hot and ground lines respectively.

Other than that, it follows the instructions in the box.

### Firmware Building
1. Install prerequisites as described in the [`avr-hal` README] (`avr-gcc`, `avr-libc`, `avrdude`, [`ravedude`]).

1. Using the Arduino IDE, download the `Servo` library.
    - The code should live in `~/Arduino/libraries/Servo`

1. Update the git submodules
```
git submodule update
```

2. Run `cargo build` to build the firmware.

3. Run `cargo run` to flash the firmware to a connected board.  If `ravedude`
   fails to detect your board, check its documentation at
   <https://crates.io/crates/ravedude>.

4. `ravedude` will open a console session after flashing where you can interact
   with the UART console of your board.

[`avr-hal` README]: https://github.com/Rahix/avr-hal#readme
[`ravedude`]: https://crates.io/crates/ravedude

## License
Licensed under either of

 - Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 - MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
