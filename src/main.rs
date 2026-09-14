use crate::enum_and_transition::{PumpEvent, PumpOutput};

mod enum_and_transition;
mod structure;

trait PumpHardware {
    fn turn_on(&mut self);
    fn turn_off(&mut self);
}

struct FakePumpHardware {
    pump_is_on: bool,
}

impl PumpHardware for FakePumpHardware {
    fn turn_off(&mut self) {
        self.pump_is_on = false;
    }

    fn turn_on(&mut self) {
        self.pump_is_on = true;
    }
}

fn apply_output<H: PumpHardware>(output: enum_and_transition::PumpOutput, hardware: &mut H) {
    match output {
        PumpOutput::TurnOn => {
            hardware.turn_on();
        }

        PumpOutput::TurnOff => {
            hardware.turn_off();
        }
    }
}

fn main() {
    let mut controller = structure::PumpController::new();

    controller.record_valid_packet(4000);
    println!("{:?}", controller.observe_last_valid_packet());
}
