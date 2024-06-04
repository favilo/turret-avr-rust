use arduino_hal::{hal::port::*, pac::EXINT, port::mode::Input};

pub trait AttachPCInterrupt {
    const PORT: u8;
    const PIN: u8;

    /// Attach a pin change interrupt to the pin
    /// INFO: see https://thewanderingengineer.com/2014/08/11/arduino-pin-change-interrupts/
    fn attach_pc_int(&self, exint: &EXINT) {
        // Enable PORT
        exint
            .pcicr
            .modify(|r, w| unsafe { w.bits(Self::PORT | r.bits()) });
        // Enable PC inetrrupt for PIN
        exint.pcmsk0.modify(|r, w| w.bits(Self::PIN | r.bits()));
    }
}

macro_rules! attach_pc_interrupt {
    (
        $name:ident = $port:literal; [$($pin:literal),+]
    ) => {
        $(
            paste::paste! {
                impl<MODE> AttachPCInterrupt for Pin<Input<MODE>, [<$name $pin>]> {
                    const PORT: u8 = $port;
                    const PIN: u8 = 1 << $pin;
                }
            }
        )+
    };
}

attach_pc_interrupt!(PB = 0b001; [0, 1, 2, 3, 4, 5, 6, 7]);
attach_pc_interrupt!(PC = 0b010; [0, 1, 2, 3, 4, 5, 6]);
attach_pc_interrupt!(PD = 0b100; [0, 1, 2, 3, 4, 5, 6, 7]);
