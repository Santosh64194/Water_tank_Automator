mod Lora;
mod enum_and_transition;
mod pump_controller;

use crate::{Lora::*, enum_and_transition::*, pump_controller::*};

fn main() {
    // =========================================================
    // HARDWARE
    // =========================================================

    let mut lora_hardware = FakeLoraHardware::new();
    let mut pump_hardware = FakePumpHardware::new();

    // =========================================================
    // COMMUNICATION STATE
    // =========================================================

    let mut comm = CommState {
        last_received_seq: None,
    };

    // =========================================================
    // PUMP CONTROLLER
    // =========================================================

    let mut pump_controller = PumpController::new();

    // =========================================================
    // 1. RECEIVE LOW-TANK PACKET
    // =========================================================

    let current_ms = 0;

    let mut packet = LoraPacket {
        device_id: 1,
        tank_level: TankLevel::Low,
        seq: 1,
        crc: 0,
    };

    packet.crc = CommState::generate_crc(&packet);

    println!("==============================");
    println!("1. LOW TANK PACKET");
    println!("==============================");

    if let Some(level) = comm.process_packet(packet, &mut lora_hardware) {
        // Valid packet received.
        pump_controller.record_valid_packet(current_ms);

        let event = PumpEvent::TankLevel {
            level_of_tank: level,
        };

        let output = pump_controller.handle_event(event, current_ms);

        apply_output(output, &mut pump_hardware);
    }

    println!("Pump state: {:?}", pump_controller.state());
    println!("Pump ON: {}", pump_hardware.pump_is_on);
    println!("Last ACK: {:?}", lora_hardware.last_ack);

    // =========================================================
    // 2. PUMP RUNS FOR 30 SECONDS
    // =========================================================

    let current_ms = 30_000;

    println!();
    println!("==============================");
    println!("2. 30 SECONDS");
    println!("==============================");

    if pump_controller.check_max_run_timeout(current_ms) {
        let output = pump_controller.handle_max_run_timeout(current_ms);

        apply_output(output, &mut pump_hardware);
    }

    println!("Pump state: {:?}", pump_controller.state());
    println!("Pump ON: {}", pump_hardware.pump_is_on);

    // =========================================================
    // 3. ANOTHER VALID LOW PACKET
    // =========================================================

    let current_ms = 40_000;

    let mut packet = LoraPacket {
        device_id: 1,
        tank_level: TankLevel::Low,
        seq: 2,
        crc: 0,
    };

    packet.crc = CommState::generate_crc(&packet);

    println!();
    println!("==============================");
    println!("3. NEW LOW PACKET");
    println!("==============================");

    if let Some(level) = comm.process_packet(packet, &mut lora_hardware) {
        pump_controller.record_valid_packet(current_ms);

        let event = PumpEvent::TankLevel {
            level_of_tank: level,
        };

        let output = pump_controller.handle_event(event, current_ms);

        apply_output(output, &mut pump_hardware);
    }

    println!("Pump state: {:?}", pump_controller.state());
    println!("Pump ON: {}", pump_hardware.pump_is_on);
    println!("Last ACK: {:?}", lora_hardware.last_ack);

    // =========================================================
    // 4. MAXIMUM RUN TIME
    // =========================================================

    let current_ms = 100_001;

    println!();
    println!("==============================");
    println!("4. MAXIMUM RUN TIME");
    println!("==============================");

    if pump_controller.check_max_run_timeout(current_ms) {
        let output = pump_controller.handle_max_run_timeout(current_ms);

        apply_output(output, &mut pump_hardware);
    }

    println!("Pump state: {:?}", pump_controller.state());
    println!("Pump ON: {}", pump_hardware.pump_is_on);

    // =========================================================
    // 5. RESET
    // =========================================================

    println!();
    println!("==============================");
    println!("5. RESET");
    println!("==============================");

    let output = pump_controller.handle_event(PumpEvent::Reset, current_ms);

    apply_output(output, &mut pump_hardware);

    println!("Pump state: {:?}", pump_controller.state());
    println!("Pump ON: {}", pump_hardware.pump_is_on);

    println!();
    println!("==============================");
    println!("SIMULATION COMPLETE");
    println!("==============================");
}
