//! Converts the two wired float-switch readings into a tank level.
//!
//! `low_active` and `full_active` must be logical states after applying
//! the actual interface board's polarity (true means the switch threshold
//! is active). Do not connect 12 V directly to an ESP32 GPIO.

use crate::enum_and_transition::TankLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatInputs {
    pub low_active: bool,
    pub full_active: bool,
}

/// Interpret the two threshold switches.
///
/// Assumption: LOW is active at/below the low threshold and FULL is active
/// at/above the full threshold. Both active simultaneously is treated as a
/// contradictory sensor state.
pub fn interpret_tank_level(inputs: FloatInputs) -> TankLevel {
    match (inputs.low_active, inputs.full_active) {
        (true, false) => TankLevel::Low,
        (false, false) => TankLevel::Normal,
        (false, true) => TankLevel::Full,
        (true, true) => TankLevel::Fault,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_only_means_low() {
        assert_eq!(
            interpret_tank_level(FloatInputs { low_active: true, full_active: false }),
            TankLevel::Low
        );
    }

    #[test]
    fn neither_active_means_normal() {
        assert_eq!(
            interpret_tank_level(FloatInputs { low_active: false, full_active: false }),
            TankLevel::Normal
        );
    }

    #[test]
    fn full_only_means_full() {
        assert_eq!(
            interpret_tank_level(FloatInputs { low_active: false, full_active: true }),
            TankLevel::Full
        );
    }

    #[test]
    fn both_active_means_fault() {
        assert_eq!(
            interpret_tank_level(FloatInputs { low_active: true, full_active: true }),
            TankLevel::Fault
        );
    }
}
