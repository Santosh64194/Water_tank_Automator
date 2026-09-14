use crate::enum_and_transition::{
    self,
    FaultReason::{self, CommTimeOut, MaximumRunTime},
    PumpEvent, PumpOutput, PumpState, pumpstate_transition,
};

#[derive(Debug)]
pub struct PumpController {
    state: enum_and_transition::PumpState,
    faultreason: Option<enum_and_transition::FaultReason>,
    last_valid_packet_ms: Option<u32>,
    pump_started_ms: Option<u32>,
    //something
}

impl PumpController {
    pub fn new() -> Self {
        PumpController {
            state: PumpState::Off,
            faultreason: None,
            last_valid_packet_ms: None,
            pump_started_ms: None,
        }
    }

    pub fn set_fault(&mut self, reason: FaultReason) {
        self.state = enum_and_transition::PumpState::Fault;
        self.faultreason = Some(reason);
    }

    pub fn handle_event(&mut self, event: PumpEvent, current_ms: u32) -> PumpOutput {
        let old_state = self.state;
        match event {
            PumpEvent::Fault { reason_of_fault } => {
                self.set_fault(reason_of_fault);
                self.pump_started_ms = None;
                PumpOutput::TurnOff
            }

            PumpEvent::Reset => {
                self.state = pumpstate_transition(self.state, event);
                self.faultreason = None;
                self.pump_started_ms = None;
                PumpOutput::TurnOff
            }

            _ => {
                self.state = pumpstate_transition(self.state, event);
                if old_state == PumpState::Off && self.state == PumpState::Running {
                    self.record_pump_start(current_ms);
                }

                if old_state == PumpState::Running && self.state == PumpState::Off {
                    self.pump_started_ms = None;
                }
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
                let elapsed_time = current_ms.wrapping_sub(value);
                elapsed_time > COMM_TIME_OUT_MS
            }

            None => false,
        }
    }

    pub fn handle_comm_timeout(&mut self, current_ms: u32) -> PumpOutput {
        if self.check_comm_timeout(current_ms) {
            let event = PumpEvent::Fault {
                reason_of_fault: CommTimeOut,
            };
            self.handle_event(event, current_ms)
        } else {
            match self.state {
                PumpState::Off => PumpOutput::TurnOff,
                PumpState::Running => PumpOutput::TurnOn,
                PumpState::Fault => PumpOutput::TurnOff,
            }
        }
    }

    pub fn record_pump_start(&mut self, current_ms: u32) {
        self.pump_started_ms = Some(current_ms);
    }

    pub fn check_max_run_timeout(&self, current_ms: u32) -> bool {
        const MAXRUN_TIMEOUT: u32 = 60_000;
        match self.pump_started_ms {
            Some(value) => {
                let elapsed_time = current_ms.wrapping_sub(value);
                elapsed_time > MAXRUN_TIMEOUT
            }

            None => false,
        }
    }

    pub fn handle_max_run_timeout(&mut self, current_ms: u32) -> PumpOutput {
        if self.check_max_run_timeout(current_ms) {
            let event = PumpEvent::Fault {
                reason_of_fault: MaximumRunTime,
            };
            self.handle_event(event, current_ms)
        } else {
            match self.state {
                PumpState::Off => PumpOutput::TurnOff,
                PumpState::Running => PumpOutput::TurnOn,
                PumpState::Fault => PumpOutput::TurnOff,
            }
        }
    }
}
