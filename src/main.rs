use crate::enum_and_transition::{PumpEvent, PumpOutput};

mod enum_and_transition;
mod structure;

trait PumpHardware {
    fn turnon(&mut self);
    fn turnoff(&mut self);
}

struct FakePumpHardware {
    pump_is_on: bool,
}

impl PumpHardware for FakePumpHardware {
    fn turnoff(&mut self) {
        self.pump_is_on = false;
    }

    fn turnon(&mut self) {
        self.pump_is_on = true;
    }
}

fn apply_output<H: PumpHardware>(output: enum_and_transition::PumpOutput, hardware: &mut H) {
    match output {
        PumpOutput::TurnOn => {
            hardware.turnon();
        }

        PumpOutput::TurnOff => {
            hardware.turnoff();
        }
    }
}

fn main() {
    let mut controller = structure::PumpController::new();

    let mut hardware = FakePumpHardware { pump_is_on: false };
    apply_output(PumpOutput::TurnOn, &mut hardware);

    println!("{}", hardware.pump_is_on);

    apply_output(PumpOutput::TurnOff, &mut hardware);
    println!("{}", hardware.pump_is_on);
}
