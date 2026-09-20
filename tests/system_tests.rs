use enum_001::enum_and_transition::{FaultReason, PumpEvent, PumpOutput, PumpState, TankLevel};

use enum_001::pump_controller::{FakePumpHardware, PumpController, apply_output};

use enum_001::lora::{CommState, FakeLoraHardware, LoraPacket, LoraSender, SenderState};

// ============================================================
// TEST HELPERS
// ============================================================

fn make_packet(device_id: u8, tank_level: TankLevel, seq: u32) -> LoraPacket {
    let mut packet = LoraPacket {
        device_id,
        tank_level,
        seq,
        crc: 0,
    };

    packet.crc = calculate_test_crc(&packet);

    packet
}

// This is the same CRC-16/CCITT-FALSE algorithm used by
// the production protocol.
//
// We calculate it independently here so the test does not
// simply call the production CRC function and test itself.
fn calculate_test_crc(packet: &LoraPacket) -> u16 {
    let mut data = [0u8; 6];

    data[0] = packet.device_id;
    data[1] = tank_level_to_test_byte(packet.tank_level);

    let seq_bytes = packet.seq.to_be_bytes();

    data[2] = seq_bytes[0];
    data[3] = seq_bytes[1];
    data[4] = seq_bytes[2];
    data[5] = seq_bytes[3];

    let mut crc: u16 = 0xFFFF;

    for byte in data {
        crc ^= (byte as u16) << 8;

        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }

    crc
}

fn tank_level_to_test_byte(level: TankLevel) -> u8 {
    match level {
        TankLevel::Unknown => 0,
        TankLevel::Low => 1,
        TankLevel::Normal => 2,
        TankLevel::Full => 3,
        TankLevel::Fault => 4,
    }
}

// ============================================================
// PUMP BASIC BEHAVIOUR
// ============================================================

#[test]
fn low_starts_pump() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Running);
    assert!(hardware.pump_is_on);
}

#[test]
fn normal_keeps_pump_off_when_already_off() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Normal,
        },
        0,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Off);
    assert!(!hardware.pump_is_on);
}

#[test]
fn normal_does_not_stop_running_pump() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Running);
    assert!(hardware.pump_is_on);

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Normal,
        },
        1_000,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Running);
    assert!(hardware.pump_is_on);
}

#[test]
fn full_stops_pump() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut hardware);

    assert!(hardware.pump_is_on);

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Full,
        },
        1_000,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Off);
    assert!(!hardware.pump_is_on);
}

// ============================================================
// UNKNOWN / INVALID TANK STATES
// ============================================================

#[test]
fn unknown_does_not_start_pump() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Unknown,
        },
        0,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Off);
    assert!(!hardware.pump_is_on);
}

#[test]
fn unknown_stops_running_pump() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut hardware);

    assert!(hardware.pump_is_on);

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Unknown,
        },
        1_000,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Off);
    assert!(!hardware.pump_is_on);
}

// ============================================================
// SENSOR FAULT
// ============================================================

#[test]
fn tank_sensor_fault_stops_pump_and_enters_fault() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut hardware);

    assert!(hardware.pump_is_on);

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Fault,
        },
        1_000,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Fault);
    assert!(!hardware.pump_is_on);
}

#[test]
fn controller_fault_event_stops_pump() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut hardware);

    let output = controller.handle_event(
        PumpEvent::Fault {
            reason_of_fault: FaultReason::TankSensorFault,
        },
        1_000,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Fault);
    assert!(!hardware.pump_is_on);
    assert_eq!(controller.faultreason(), Some(FaultReason::TankSensorFault));
}

// ============================================================
// FAULT LATCHING
// ============================================================

#[test]
fn fault_state_is_latched() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    controller.handle_event(
        PumpEvent::Fault {
            reason_of_fault: FaultReason::TankSensorFault,
        },
        0,
    );

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        1_000,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Fault);
    assert!(!hardware.pump_is_on);
}

// ============================================================
// RESET
// ============================================================

#[test]
fn reset_clears_fault() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    controller.handle_event(
        PumpEvent::Fault {
            reason_of_fault: FaultReason::TankSensorFault,
        },
        0,
    );

    assert_eq!(controller.state(), PumpState::Fault);

    let output = controller.handle_event(PumpEvent::Reset, 1_000);

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Off);
    assert_eq!(controller.faultreason(), None);
    assert!(!hardware.pump_is_on);
}

#[test]
fn reset_clears_communication_timer() {
    let mut controller = PumpController::new();

    controller.record_valid_packet(0);

    assert_eq!(controller.observe_last_valid_packet(), Some(0));

    controller.handle_event(
        PumpEvent::Fault {
            reason_of_fault: FaultReason::CommTimeOut,
        },
        20_000,
    );

    controller.handle_event(PumpEvent::Reset, 21_000);

    assert_eq!(controller.observe_last_valid_packet(), None);

    assert!(!controller.check_comm_timeout(100_000));
}

// ============================================================
// COMMUNICATION TIMEOUT
// ============================================================

#[test]
fn communication_timeout_occurs_after_limit() {
    let mut controller = PumpController::new();

    controller.record_valid_packet(0);

    assert!(!controller.check_comm_timeout(9_999));
    assert!(controller.check_comm_timeout(10_001));
}

#[test]
fn communication_timeout_faults_running_pump() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    controller.record_valid_packet(0);

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut hardware);

    assert!(hardware.pump_is_on);

    assert!(controller.check_comm_timeout(10_001));

    let output = controller.handle_event(
        PumpEvent::Fault {
            reason_of_fault: FaultReason::CommTimeOut,
        },
        10_001,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Fault);
    assert!(!hardware.pump_is_on);
}

// ============================================================
// MAXIMUM PUMP RUNTIME
// ============================================================

#[test]
fn maximum_runtime_not_reached_early() {
    let mut controller = PumpController::new();

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    assert_eq!(output, PumpOutput::TurnOn);

    assert!(!controller.check_max_run_timeout(59_999));
}

#[test]
fn maximum_runtime_is_detected() {
    let mut controller = PumpController::new();

    controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    assert!(controller.check_max_run_timeout(60_001));
}

#[test]
fn maximum_runtime_faults_pump() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut hardware);

    assert!(hardware.pump_is_on);

    controller.handle_event(
        PumpEvent::Fault {
            reason_of_fault: FaultReason::MaximumRunTime,
        },
        60_001,
    );

    let output = PumpOutput::TurnOff;
    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Fault);
    assert!(!hardware.pump_is_on);
}

// ============================================================
// CRC
// ============================================================

#[test]
fn valid_crc_is_accepted() {
    let packet = make_packet(1, TankLevel::Low, 1);

    let mut comm = CommState::new();
    let mut hardware = FakeLoraHardware::new();

    let result = comm.process_packet(packet, &mut hardware);

    assert_eq!(result, Some(TankLevel::Low));
    assert_eq!(hardware.last_ack, Some(1));
}

#[test]
fn corrupted_crc_is_rejected() {
    let mut packet = make_packet(1, TankLevel::Low, 1);

    packet.crc ^= 0x0001;

    let mut comm = CommState::new();
    let mut hardware = FakeLoraHardware::new();

    let result = comm.process_packet(packet, &mut hardware);

    assert_eq!(result, None);
    assert_eq!(hardware.last_ack, None);
}

// ============================================================
// SEQUENCE NUMBERS
// ============================================================

#[test]
fn newer_sequence_is_accepted() {
    let packet1 = make_packet(1, TankLevel::Low, 1);
    let packet2 = make_packet(1, TankLevel::Normal, 2);

    let mut comm = CommState::new();
    let mut hardware = FakeLoraHardware::new();

    assert_eq!(
        comm.process_packet(packet1, &mut hardware),
        Some(TankLevel::Low)
    );

    assert_eq!(
        comm.process_packet(packet2, &mut hardware),
        Some(TankLevel::Normal)
    );
}

#[test]
fn duplicate_sequence_is_not_processed_twice() {
    let packet = make_packet(1, TankLevel::Low, 1);

    let mut comm = CommState::new();
    let mut hardware = FakeLoraHardware::new();

    let first = comm.process_packet(packet.clone(), &mut hardware);

    assert_eq!(first, Some(TankLevel::Low));

    let second = comm.process_packet(packet, &mut hardware);

    assert_eq!(second, None);

    // ACK must still be generated for the duplicate.
    assert_eq!(hardware.last_ack, Some(1));
}

#[test]
fn old_sequence_is_rejected() {
    let packet1 = make_packet(1, TankLevel::Normal, 10);
    let packet2 = make_packet(1, TankLevel::Low, 9);

    let mut comm = CommState::new();
    let mut hardware = FakeLoraHardware::new();

    assert_eq!(
        comm.process_packet(packet1, &mut hardware),
        Some(TankLevel::Normal)
    );

    assert_eq!(comm.process_packet(packet2, &mut hardware), None);
}

#[test]
fn sequence_zero_is_valid_initial_sequence() {
    let packet = make_packet(1, TankLevel::Low, 0);

    let mut comm = CommState::new();
    let mut hardware = FakeLoraHardware::new();

    assert_eq!(
        comm.process_packet(packet, &mut hardware),
        Some(TankLevel::Low)
    );
}

#[test]
fn sequence_wraparound_is_supported() {
    let packet_max = make_packet(1, TankLevel::Normal, u32::MAX);

    let packet_zero = make_packet(1, TankLevel::Low, 0);

    let mut comm = CommState::new();
    let mut hardware = FakeLoraHardware::new();

    assert_eq!(
        comm.process_packet(packet_max, &mut hardware),
        Some(TankLevel::Normal)
    );

    assert_eq!(
        comm.process_packet(packet_zero, &mut hardware),
        Some(TankLevel::Low)
    );
}

// ============================================================
// ACK GENERATION
// ============================================================

#[test]
fn ack_contains_received_sequence() {
    let packet = make_packet(1, TankLevel::Low, 12345);

    let mut comm = CommState::new();
    let mut hardware = FakeLoraHardware::new();

    comm.process_packet(packet, &mut hardware);

    assert_eq!(hardware.last_ack, Some(12345));
}

// ============================================================
// SENDER ACK HANDLING
// ============================================================

#[test]
fn sender_starts_waiting_for_ack() {
    let mut sender = LoraSender::new();
    let mut hardware = FakeLoraHardware::new();

    let packet = make_packet(1, TankLevel::Low, 1);

    sender.start_transmission(&packet, 0, &mut hardware);

    assert_eq!(sender.state, SenderState::WaitingForAck);

    assert_eq!(sender.pending_packet.as_ref().unwrap().seq, 1);
}

#[test]
fn correct_ack_returns_sender_to_idle() {
    let mut sender = LoraSender::new();
    let mut hardware = FakeLoraHardware::new();

    let packet = make_packet(1, TankLevel::Low, 1);

    sender.start_transmission(&packet, 0, &mut hardware);

    assert!(sender.receive_ack(1));

    assert_eq!(sender.state, SenderState::Idle);

    assert!(sender.pending_packet.is_none());
}

#[test]
fn wrong_ack_is_rejected() {
    let mut sender = LoraSender::new();
    let mut hardware = FakeLoraHardware::new();

    let packet = make_packet(1, TankLevel::Low, 1);

    sender.start_transmission(&packet, 0, &mut hardware);

    assert!(!sender.receive_ack(999));

    assert_eq!(sender.state, SenderState::WaitingForAck);
}

#[test]
fn ack_when_sender_is_idle_is_rejected() {
    let mut sender = LoraSender::new();

    assert!(!sender.receive_ack(1));

    assert_eq!(sender.state, SenderState::Idle);
}

// ============================================================
// LOST PACKET / RETRY
// ============================================================

#[test]
fn lost_packet_causes_retry() {
    let mut sender = LoraSender::new();
    let mut hardware = FakeLoraHardware::new();

    let packet = make_packet(1, TankLevel::Low, 1);

    sender.start_transmission(&packet, 0, &mut hardware);

    assert_eq!(sender.retry_count, 0);

    sender.handle_ack_timeout(5_000, &mut hardware);

    assert_eq!(sender.state, SenderState::WaitingForAck);

    assert_eq!(sender.retry_count, 1);
}

#[test]
fn sender_does_not_retry_before_timeout() {
    let mut sender = LoraSender::new();
    let mut hardware = FakeLoraHardware::new();

    let packet = make_packet(1, TankLevel::Low, 1);

    sender.start_transmission(&packet, 0, &mut hardware);

    sender.handle_ack_timeout(4_999, &mut hardware);

    assert_eq!(sender.retry_count, 0);

    assert_eq!(sender.state, SenderState::WaitingForAck);
}

// ============================================================
// LOST ACK / DUPLICATE PACKET
// ============================================================

#[test]
fn lost_ack_causes_duplicate_packet_but_only_one_application_event() {
    let packet = make_packet(1, TankLevel::Low, 1);

    let mut receiver = CommState::new();
    let mut receiver_hardware = FakeLoraHardware::new();

    // First packet reaches receiver.
    let first = receiver.process_packet(packet.clone(), &mut receiver_hardware);

    assert_eq!(first, Some(TankLevel::Low));

    // ACK is assumed lost.

    // Sender retransmits same packet.
    let second = receiver.process_packet(packet, &mut receiver_hardware);

    // Application must NOT receive the tank event twice.
    assert_eq!(second, None);

    // But receiver must send ACK again.
    assert_eq!(receiver_hardware.last_ack, Some(1));
}

// ============================================================
// RETRY LIMIT
// ============================================================

#[test]
fn sender_enters_fault_after_maximum_retries() {
    let mut sender = LoraSender::new();
    let mut hardware = FakeLoraHardware::new();

    let packet = make_packet(1, TankLevel::Low, 1);

    sender.start_transmission(&packet, 0, &mut hardware);

    // Five retries.
    sender.handle_ack_timeout(5_000, &mut hardware);
    sender.handle_ack_timeout(10_000, &mut hardware);
    sender.handle_ack_timeout(15_000, &mut hardware);
    sender.handle_ack_timeout(20_000, &mut hardware);
    sender.handle_ack_timeout(25_000, &mut hardware);

    assert_eq!(sender.retry_count, 5);

    assert_eq!(sender.state, SenderState::WaitingForAck);

    // Next timeout -> fault.
    sender.handle_ack_timeout(30_000, &mut hardware);

    assert_eq!(sender.state, SenderState::Fault);
}

// ============================================================
// ACK AFTER FAULT
// ============================================================

#[test]
fn ack_after_sender_fault_is_rejected() {
    let mut sender = LoraSender::new();
    let mut hardware = FakeLoraHardware::new();

    let packet = make_packet(1, TankLevel::Low, 1);

    sender.start_transmission(&packet, 0, &mut hardware);

    sender.handle_ack_timeout(5_000, &mut hardware);
    sender.handle_ack_timeout(10_000, &mut hardware);
    sender.handle_ack_timeout(15_000, &mut hardware);
    sender.handle_ack_timeout(20_000, &mut hardware);
    sender.handle_ack_timeout(25_000, &mut hardware);
    sender.handle_ack_timeout(30_000, &mut hardware);

    assert_eq!(sender.state, SenderState::Fault);

    assert!(!sender.receive_ack(1));

    assert_eq!(sender.state, SenderState::Fault);
}

// ============================================================
// TIMER WRAPAROUND
// ============================================================

#[test]
fn communication_timeout_handles_u32_wraparound() {
    let mut controller = PumpController::new();

    let start = u32::MAX - 5_000;

    controller.record_valid_packet(start);

    assert!(!controller.check_comm_timeout(start.wrapping_add(9_000)));

    assert!(controller.check_comm_timeout(start.wrapping_add(10_001)));
}

#[test]
fn maximum_runtime_handles_u32_wraparound() {
    let mut controller = PumpController::new();

    let start = u32::MAX - 5_000;

    controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        start,
    );

    assert!(!controller.check_max_run_timeout(start.wrapping_add(59_999)));

    assert!(controller.check_max_run_timeout(start.wrapping_add(60_001)));
}

// ============================================================
// FULL SAFETY CHAIN
// ============================================================

#[test]
fn full_safety_sequence() {
    let mut controller = PumpController::new();
    let mut hardware = FakePumpHardware::new();

    // 1. Tank LOW -> pump ON.
    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Running);

    assert!(hardware.pump_is_on);

    // 2. Communication becomes valid.
    controller.record_valid_packet(0);

    // 3. FULL -> pump OFF.
    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Full,
        },
        1_000,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Off);

    assert!(!hardware.pump_is_on);

    // 4. LOW again -> pump ON.
    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        2_000,
    );

    apply_output(output, &mut hardware);

    assert!(hardware.pump_is_on);

    // 5. Sensor fault -> OFF + FAULT.
    let output = controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Fault,
        },
        3_000,
    );

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Fault);

    assert!(!hardware.pump_is_on);

    // 6. Reset -> OFF.
    let output = controller.handle_event(PumpEvent::Reset, 4_000);

    apply_output(output, &mut hardware);

    assert_eq!(controller.state(), PumpState::Off);

    assert!(!hardware.pump_is_on);
}

#[test]
fn end_to_end_low_tank_starts_pump() {
    // ---------------------------------------------------------
    // 1. Create the system components
    // ---------------------------------------------------------
    let mut receiver = CommState::new();
    let mut lora_hardware = FakeLoraHardware::new();

    let mut sender = LoraSender::new();

    let mut pump_controller = PumpController::new();
    let mut pump_hardware = FakePumpHardware::new();

    // ---------------------------------------------------------
    // 2. Tank reports LOW
    // ---------------------------------------------------------
    let tank_level = TankLevel::Low;

    // ---------------------------------------------------------
    // 3. Create the LoRa packet
    // ---------------------------------------------------------
    let mut packet = LoraPacket {
        device_id: 1,
        tank_level,
        seq: 1,
        crc: 0,
    };

    // Generate CRC using the receiver's protocol implementation.
    packet.crc = CommState::generate_crc(&packet);

    // ---------------------------------------------------------
    // 4. Top-floor sender transmits the packet
    // ---------------------------------------------------------
    sender.start_transmission(&packet, 0, &mut lora_hardware);

    assert_eq!(sender.state, SenderState::WaitingForAck);

    // Verify packet reached the LoRa hardware.
    assert_eq!(lora_hardware.last_packet, Some(1));

    // ---------------------------------------------------------
    // 5. Ground-floor receiver processes the packet
    // ---------------------------------------------------------
    let received_level = receiver.process_packet(packet, &mut lora_hardware);

    // LOW should be accepted.
    assert_eq!(received_level, Some(TankLevel::Low));

    // ---------------------------------------------------------
    // 6. Receiver sends ACK
    // ---------------------------------------------------------
    assert_eq!(lora_hardware.last_ack, Some(1));

    // ---------------------------------------------------------
    // 7. Sender receives the ACK
    // ---------------------------------------------------------
    let ack_accepted = sender.receive_ack(1);

    assert!(ack_accepted);
    assert_eq!(sender.state, SenderState::Idle);

    // ---------------------------------------------------------
    // 8. Pump controller receives the accepted tank level
    // ---------------------------------------------------------
    let output = pump_controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: received_level.unwrap(),
        },
        0,
    );

    // ---------------------------------------------------------
    // 9. Apply pump controller output to fake hardware
    // ---------------------------------------------------------
    apply_output(output, &mut pump_hardware);

    // ---------------------------------------------------------
    // 10. Verify final system state
    // ---------------------------------------------------------
    assert_eq!(pump_controller.state(), PumpState::Running);
    assert!(pump_hardware.pump_is_on);
}

#[test]
fn end_to_end_communication_loss_stops_running_pump() {
    // ---------------------------------------------------------
    // 1. Create the system components
    // ---------------------------------------------------------
    let mut receiver = CommState::new();
    let mut lora_hardware = FakeLoraHardware::new();

    let mut sender = LoraSender::new();

    let mut pump_controller = PumpController::new();
    let mut pump_hardware = FakePumpHardware::new();

    // ---------------------------------------------------------
    // 2. Tank reports LOW
    // ---------------------------------------------------------
    let mut packet = LoraPacket {
        device_id: 1,
        tank_level: TankLevel::Low,
        seq: 1,
        crc: 0,
    };

    packet.crc = CommState::generate_crc(&packet);

    // ---------------------------------------------------------
    // 3. Send LOW packet
    // ---------------------------------------------------------
    sender.start_transmission(&packet, 0, &mut lora_hardware);

    assert_eq!(sender.state, SenderState::WaitingForAck);

    // ---------------------------------------------------------
    // 4. Receiver accepts LOW
    // ---------------------------------------------------------
    let received_level = receiver.process_packet(packet, &mut lora_hardware);

    assert_eq!(received_level, Some(TankLevel::Low));

    // ---------------------------------------------------------
    // 5. Sender receives ACK
    // ---------------------------------------------------------
    let ack_accepted = sender.receive_ack(1);

    assert!(ack_accepted);
    assert_eq!(sender.state, SenderState::Idle);

    // ---------------------------------------------------------
    // 6. Record the valid communication
    // ---------------------------------------------------------
    pump_controller.record_valid_packet(0);

    // ---------------------------------------------------------
    // 7. Deliver LOW to pump controller
    // ---------------------------------------------------------
    let output = pump_controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut pump_hardware);

    // Pump must now be running.
    assert_eq!(pump_controller.state(), PumpState::Running);
    assert!(pump_hardware.pump_is_on);

    // ---------------------------------------------------------
    // 8. Simulate communication loss
    //
    // No valid packets arrive for more than 10 seconds.
    // ---------------------------------------------------------
    let timeout_occurred = pump_controller.check_comm_timeout(10_001);

    assert!(timeout_occurred);

    // ---------------------------------------------------------
    // 9. Handle communication timeout
    // ---------------------------------------------------------
    let output = pump_controller.handle_comm_timeout(10_001);

    apply_output(output, &mut pump_hardware);

    // ---------------------------------------------------------
    // 10. SAFETY CHECK
    // ---------------------------------------------------------
    assert_eq!(pump_controller.state(), PumpState::Fault);

    assert_eq!(
        pump_controller.faultreason(),
        Some(FaultReason::CommTimeOut)
    );

    assert!(!pump_hardware.pump_is_on);
}

#[test]
fn end_to_end_full_tank_stops_pump() {
    // ---------------------------------------------------------
    // 1. Create system components
    // ---------------------------------------------------------
    let mut receiver = CommState::new();
    let mut lora_hardware = FakeLoraHardware::new();

    let mut sender = LoraSender::new();

    let mut pump_controller = PumpController::new();
    let mut pump_hardware = FakePumpHardware::new();

    // ---------------------------------------------------------
    // 2. First send LOW so the pump starts
    // ---------------------------------------------------------
    let mut low_packet = LoraPacket {
        device_id: 1,
        tank_level: TankLevel::Low,
        seq: 1,
        crc: 0,
    };

    low_packet.crc = CommState::generate_crc(&low_packet);

    sender.start_transmission(&low_packet, 0, &mut lora_hardware);

    let received_level = receiver.process_packet(low_packet, &mut lora_hardware);

    assert_eq!(received_level, Some(TankLevel::Low));

    assert!(sender.receive_ack(1));

    let output = pump_controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut pump_hardware);

    assert_eq!(pump_controller.state(), PumpState::Running);
    assert!(pump_hardware.pump_is_on);

    // ---------------------------------------------------------
    // 3. Tank now reports FULL
    // ---------------------------------------------------------
    let mut full_packet = LoraPacket {
        device_id: 1,
        tank_level: TankLevel::Full,
        seq: 2,
        crc: 0,
    };

    full_packet.crc = CommState::generate_crc(&full_packet);

    sender.start_transmission(&full_packet, 1000, &mut lora_hardware);

    let received_level = receiver.process_packet(full_packet, &mut lora_hardware);

    assert_eq!(received_level, Some(TankLevel::Full));

    assert!(sender.receive_ack(2));

    // ---------------------------------------------------------
    // 4. Deliver FULL to pump controller
    // ---------------------------------------------------------
    let output = pump_controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Full,
        },
        1000,
    );

    apply_output(output, &mut pump_hardware);

    // ---------------------------------------------------------
    // 5. SAFETY CHECK
    // ---------------------------------------------------------
    assert_eq!(pump_controller.state(), PumpState::Off);
    assert!(!pump_hardware.pump_is_on);
}

#[test]
fn end_to_end_sensor_fault_stops_pump() {
    // ---------------------------------------------------------
    // 1. Create system components
    // ---------------------------------------------------------
    let mut receiver = CommState::new();
    let mut lora_hardware = FakeLoraHardware::new();

    let mut sender = LoraSender::new();

    let mut pump_controller = PumpController::new();
    let mut pump_hardware = FakePumpHardware::new();

    // ---------------------------------------------------------
    // 2. Start the pump with LOW
    // ---------------------------------------------------------
    let mut low_packet = LoraPacket {
        device_id: 1,
        tank_level: TankLevel::Low,
        seq: 1,
        crc: 0,
    };

    low_packet.crc = CommState::generate_crc(&low_packet);

    sender.start_transmission(&low_packet, 0, &mut lora_hardware);

    let received_level = receiver.process_packet(low_packet, &mut lora_hardware);

    assert_eq!(received_level, Some(TankLevel::Low));
    assert!(sender.receive_ack(1));

    let output = pump_controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut pump_hardware);

    assert_eq!(pump_controller.state(), PumpState::Running);
    assert!(pump_hardware.pump_is_on);

    // ---------------------------------------------------------
    // 3. Sensor now reports FAULT
    // ---------------------------------------------------------
    let mut fault_packet = LoraPacket {
        device_id: 1,
        tank_level: TankLevel::Fault,
        seq: 2,
        crc: 0,
    };

    fault_packet.crc = CommState::generate_crc(&fault_packet);

    sender.start_transmission(&fault_packet, 1000, &mut lora_hardware);

    let received_level = receiver.process_packet(fault_packet, &mut lora_hardware);

    assert_eq!(received_level, Some(TankLevel::Fault));
    assert!(sender.receive_ack(2));

    // ---------------------------------------------------------
    // 4. Deliver sensor fault to pump controller
    // ---------------------------------------------------------
    let output = pump_controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Fault,
        },
        1000,
    );

    apply_output(output, &mut pump_hardware);

    // ---------------------------------------------------------
    // 5. SAFETY CHECK
    // ---------------------------------------------------------
    assert_eq!(pump_controller.state(), PumpState::Fault);
    assert_eq!(
        pump_controller.faultreason(),
        Some(FaultReason::TankSensorFault)
    );
    assert!(!pump_hardware.pump_is_on);
}

#[test]
fn end_to_end_maximum_runtime_stops_pump() {
    // ---------------------------------------------------------
    // 1. Create system components
    // ---------------------------------------------------------
    let mut receiver = CommState::new();
    let mut lora_hardware = FakeLoraHardware::new();

    let mut sender = LoraSender::new();

    let mut pump_controller = PumpController::new();
    let mut pump_hardware = FakePumpHardware::new();

    // ---------------------------------------------------------
    // 2. Tank reports LOW
    // ---------------------------------------------------------
    let mut low_packet = LoraPacket {
        device_id: 1,
        tank_level: TankLevel::Low,
        seq: 1,
        crc: 0,
    };

    low_packet.crc = CommState::generate_crc(&low_packet);

    sender.start_transmission(&low_packet, 0, &mut lora_hardware);

    let received_level = receiver.process_packet(low_packet, &mut lora_hardware);

    assert_eq!(received_level, Some(TankLevel::Low));
    assert!(sender.receive_ack(1));

    // ---------------------------------------------------------
    // 3. Start pump at time 0
    // ---------------------------------------------------------
    let output = pump_controller.handle_event(
        PumpEvent::TankLevel {
            level_of_tank: TankLevel::Low,
        },
        0,
    );

    apply_output(output, &mut pump_hardware);

    assert_eq!(pump_controller.state(), PumpState::Running);
    assert!(pump_hardware.pump_is_on);

    // ---------------------------------------------------------
    // 4. Maximum runtime has not yet expired
    // ---------------------------------------------------------
    assert!(!pump_controller.check_max_run_timeout(59_999));

    assert_eq!(pump_controller.state(), PumpState::Running);
    assert!(pump_hardware.pump_is_on);

    // ---------------------------------------------------------
    // 5. Maximum runtime expires
    // ---------------------------------------------------------
    assert!(pump_controller.check_max_run_timeout(60_001));

    // ---------------------------------------------------------
    // 6. Handle maximum runtime fault
    // ---------------------------------------------------------
    let output = pump_controller.handle_max_run_timeout(60_001);

    apply_output(output, &mut pump_hardware);

    // ---------------------------------------------------------
    // 7. SAFETY CHECK
    // ---------------------------------------------------------
    assert_eq!(pump_controller.state(), PumpState::Fault);
    assert_eq!(
        pump_controller.faultreason(),
        Some(FaultReason::MaximumRunTime)
    );
    assert!(!pump_hardware.pump_is_on);
}
