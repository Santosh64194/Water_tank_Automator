use crate::enum_and_transition::{
    self, FaultReason, PumpEvent, PumpOutput, PumpState, pumpstate_transition,
};

#[derive(Debug)]
pub struct PumpController {
    state: enum_and_transition::PumpState,
    faultreason: Option<enum_and_transition::FaultReason>,
    //something
}

impl PumpController {
    pub fn new() -> Self {
        PumpController {
            state: PumpState::Off,
            faultreason: None,
        }
    }

    pub fn set_fault(&mut self, reason: FaultReason) {
        self.state = enum_and_transition::PumpState::Fault;
        self.faultreason = Some(reason);
    }

    pub fn handle_event(&mut self, event: PumpEvent) -> PumpOutput {
        match event {
            PumpEvent::Fault { reason_of_fault } => {
                self.set_fault(reason_of_fault);
                PumpOutput::TurnOff
            }

            PumpEvent::Reset => {
                self.state = pumpstate_transition(self.state, event);
                self.faultreason = None;
                PumpOutput::TurnOff
            }

            _ => {
                self.state = pumpstate_transition(self.state, event);
                match self.state {
                    PumpState::Off => PumpOutput::TurnOff,
                    PumpState::Running => PumpOutput::TurnOn,
                    PumpState::Fault => {
                        //sets the fault reason
                        PumpOutput::TurnOff
                    }
                }
            }
        }
    }

    pub fn state(&self) -> PumpState {
        self.state
    }

    pub fn faultreason(&self) -> &Option<enum_and_transition::FaultReason> {
        &self.faultreason
    }
}
