use crate::enum_and_transition::{
    self, FaultReason::{self, CommTimeOut}, PumpEvent, PumpOutput, PumpState, pumpstate_transition,
};

#[derive(Debug)]
pub struct PumpController {
    state: enum_and_transition::PumpState,
    faultreason: Option<enum_and_transition::FaultReason>,
    last_valid_packet_ms: Option<u32>,
    //something
}

impl PumpController {
    pub fn new() -> Self {
        PumpController {
            state: PumpState::Off,
            faultreason: None,
            last_valid_packet_ms: None,
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
                    PumpState::Fault => PumpOutput::TurnOff,
                }
            }
        }
    }

    pub fn state(&self) -> PumpState {
        self.state
    }

    pub fn faultreason(&self) -> Option<enum_and_transition::FaultReason> {
        self.faultreason
    }

    pub fn record_valid_packet(&mut self, current_ms: u32) {
        self.last_valid_packet_ms = Some(current_ms);
    }

    pub fn observe_last_valid_packet(&self) -> Option<u32> {
        self.last_valid_packet_ms
    }

    pub fn check_comm_timeout(&self, current_ms: u32) -> bool {
        const COMM_TIME_OUT_MS: u32 = 10_000;
        match self.last_valid_packet_ms {
            Some(value) => {
                let elapsed_time = current_ms - value;
                // if elapsed_time > COMM_TIME_OUT_MS {
                //     true
                // } else {
                //     false
                // }
                elapsed_time > COMM_TIME_OUT_MS
            },

            None => false
        }
    }

    pub fn handle_comm_timeout(&mut self, current_ms: u32) -> PumpOutput{
        if (self.check_comm_timeout(current_ms)) {
			let event = PumpEvent::Fault { reason_of_fault: CommTimeOut };
			self.handle_event(event)
        } else {
			match self.state {
				PumpState::Off => PumpOutput::TurnOff,
				PumpState::Running => PumpOutput::TurnOn,
				PumpState::Fault => PumpOutput::TurnOff
			}
		}
    }

}

