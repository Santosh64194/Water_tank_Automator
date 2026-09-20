#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TankLevel {
    Unknown,
    Low,
    Normal,
    Full,
    Fault,
}

pub fn tanklevel_to_bytes(level: TankLevel) -> u8 {
    match level {
        TankLevel::Unknown => 0,
        TankLevel::Low => 1,
        TankLevel::Normal => 2,
        TankLevel::Full => 3,
        TankLevel::Fault => 4,
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PumpState {
    Off,
    Running,
    Fault,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum TankEvent {
    SensorLow,
    SensorNormal,
    SensorFull,
    SensorFault,
    Reset,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FaultReason {
    CommTimeOut,
    TankSensorFault,
    MaximumRunTime,
}

#[derive(Debug)]
pub enum PumpEvent {
    Fault { reason_of_fault: FaultReason },
    TankLevel { level_of_tank: TankLevel },
    Reset,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PumpOutput {
    TurnOn,
    TurnOff,
}

pub fn tanklevel_transition(state: TankLevel, event: TankEvent) -> TankLevel {
    match (state, event) {
        (_, TankEvent::Reset) => TankLevel::Unknown,

        (TankLevel::Fault, _) => TankLevel::Fault,

        (TankLevel::Unknown, TankEvent::SensorLow) => TankLevel::Low,
        (TankLevel::Unknown, TankEvent::SensorNormal) => TankLevel::Normal,
        (TankLevel::Unknown, TankEvent::SensorFull) => TankLevel::Full,
        (TankLevel::Unknown, TankEvent::SensorFault) => TankLevel::Fault,

        (TankLevel::Low, TankEvent::SensorLow) => TankLevel::Low,
        (TankLevel::Low, TankEvent::SensorNormal) => TankLevel::Normal,
        (TankLevel::Low, TankEvent::SensorFull) => TankLevel::Full,
        (TankLevel::Low, TankEvent::SensorFault) => TankLevel::Fault,

        (TankLevel::Normal, TankEvent::SensorLow) => TankLevel::Low,
        (TankLevel::Normal, TankEvent::SensorNormal) => TankLevel::Normal,
        (TankLevel::Normal, TankEvent::SensorFull) => TankLevel::Full,
        (TankLevel::Normal, TankEvent::SensorFault) => TankLevel::Fault,

        (TankLevel::Full, TankEvent::SensorLow) => TankLevel::Low,
        (TankLevel::Full, TankEvent::SensorNormal) => TankLevel::Normal,
        (TankLevel::Full, TankEvent::SensorFull) => TankLevel::Full,
        (TankLevel::Full, TankEvent::SensorFault) => TankLevel::Fault,
    }
}

pub fn pumpstate_transition(state: PumpState, event: PumpEvent) -> PumpState {
    match (state, event) {
        (_, PumpEvent::Reset) => PumpState::Off,

        (PumpState::Fault, _) => PumpState::Fault,
        //////////////////OFF////////////////////////////
        (
            PumpState::Off,
            PumpEvent::TankLevel {
                level_of_tank: TankLevel::Unknown,
            },
        ) => PumpState::Off,
        (
            PumpState::Off,
            PumpEvent::TankLevel {
                level_of_tank: TankLevel::Low,
            },
        ) => PumpState::Running,
        (
            PumpState::Off,
            PumpEvent::TankLevel {
                level_of_tank: TankLevel::Normal,
            },
        ) => PumpState::Off,
        (
            PumpState::Off,
            PumpEvent::TankLevel {
                level_of_tank: TankLevel::Full,
            },
        ) => PumpState::Off,
        (
            PumpState::Off,
            PumpEvent::TankLevel {
                level_of_tank: TankLevel::Fault,
            },
        ) => PumpState::Fault,
        (PumpState::Off, PumpEvent::Fault { reason_of_fault: _ }) => PumpState::Fault,
        //////////////////RUNNING////////////////////////////
        (
            PumpState::Running,
            PumpEvent::TankLevel {
                level_of_tank: TankLevel::Unknown,
            },
        ) => PumpState::Off,
        (
            PumpState::Running,
            PumpEvent::TankLevel {
                level_of_tank: TankLevel::Low,
            },
        ) => PumpState::Running,
        (
            PumpState::Running,
            PumpEvent::TankLevel {
                level_of_tank: TankLevel::Normal,
            },
        ) => PumpState::Running,
        (
            PumpState::Running,
            PumpEvent::TankLevel {
                level_of_tank: TankLevel::Full,
            },
        ) => PumpState::Off,
        (
            PumpState::Running,
            PumpEvent::TankLevel {
                level_of_tank: TankLevel::Fault,
            },
        ) => PumpState::Fault,
        (PumpState::Running, PumpEvent::Fault { reason_of_fault: _ }) => PumpState::Fault,
    }
}
