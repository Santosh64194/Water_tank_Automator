use crate::{enum_and_transition::*, lora::*};

mod enum_and_transition;
mod lora;
mod pump_controller;

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
    // ========================================================
    // SETUP
    // ========================================================

    let mut hardware = FakeLoraHardware::new();
    let mut sender = LoraSender::new();

    let mut packet = LoraPacket {
        device_id: 1,
        tank_level: TankLevel::Low,
        seq: 1,
        crc: 0,
    };

    packet.crc = CommState::generate_crc(&packet);

    // ========================================================
    // 1. INITIAL TRANSMISSION
    // ========================================================

    println!("\n--- Test 1: Initial transmission ---");

    sender.start_transmission(&packet, 0, &mut hardware);

    assert_eq!(sender.state, SenderState::WaitingForAck);
    assert_eq!(sender.retry_count, 0);
    assert_eq!(sender.sent_at_ms, Some(0));
    assert_eq!(hardware.last_packet, Some(1));

    println!("PASS");


    // ========================================================
    // 2. BEFORE ACK TIMEOUT
    // ========================================================

    println!("\n--- Test 2: Before ACK timeout ---");

    sender.handle_ack_timeout(4_999, &mut hardware);

    assert_eq!(sender.retry_count, 0);
    assert_eq!(sender.state, SenderState::WaitingForAck);
    assert_eq!(hardware.last_packet, Some(1));

    println!("PASS");


    // ========================================================
    // 3. FIRST RETRY
    // ========================================================

    println!("\n--- Test 3: First retry ---");

    sender.handle_ack_timeout(5_000, &mut hardware);

    assert_eq!(sender.retry_count, 1);
    assert_eq!(sender.state, SenderState::WaitingForAck);
    assert_eq!(sender.sent_at_ms, Some(5_000));
    assert_eq!(hardware.last_packet, Some(1));

    println!("PASS");


    // ========================================================
    // 4. SECOND RETRY
    // ========================================================

    println!("\n--- Test 4: Second retry ---");

    sender.handle_ack_timeout(10_000, &mut hardware);

    assert_eq!(sender.retry_count, 2);
    assert_eq!(sender.state, SenderState::WaitingForAck);
    assert_eq!(sender.sent_at_ms, Some(10_000));

    println!("PASS");


    // ========================================================
    // 5. ACK RECEIVED
    // ========================================================

    println!("\n--- Test 5: Correct ACK ---");

    let accepted = sender.receive_ack(1);

    assert!(accepted);
    assert_eq!(sender.state, SenderState::Idle);
    assert_eq!(sender.retry_count, 0);
    assert_eq!(sender.sent_at_ms, None);
    assert!(sender.pending_packet.is_none());

    println!("PASS");


    // ========================================================
    // 6. WRONG ACK
    // ========================================================

    println!("\n--- Test 6: Wrong ACK ---");

    // Start another transmission.
    let mut packet2 = LoraPacket {
        device_id: 1,
        tank_level: TankLevel::Normal,
        seq: 2,
        crc: 0,
    };

    packet2.crc = CommState::generate_crc(&packet2);

    sender.start_transmission(&packet2, 20_000, &mut hardware);

    let accepted = sender.receive_ack(999);

    assert!(!accepted);
    assert_eq!(sender.state, SenderState::WaitingForAck);
    assert_eq!(sender.retry_count, 0);
    assert!(sender.pending_packet.is_some());

    println!("PASS");


    // ========================================================
    // 7. RETRY AFTER WRONG ACK
    // ========================================================

    println!("\n--- Test 7: Retry after wrong ACK ---");

    sender.handle_ack_timeout(25_000, &mut hardware);

    assert_eq!(sender.retry_count, 1);
    assert_eq!(sender.state, SenderState::WaitingForAck);
    assert_eq!(hardware.last_packet, Some(2));

    println!("PASS");


    // ========================================================
    // 8. MAX RETRIES
    // ========================================================

    println!("\n--- Test 8: Maximum retries ---");

    sender.handle_ack_timeout(30_000, &mut hardware);
    sender.handle_ack_timeout(35_000, &mut hardware);
    sender.handle_ack_timeout(40_000, &mut hardware);
    sender.handle_ack_timeout(45_000, &mut hardware);

    assert_eq!(sender.retry_count, 5);
    assert_eq!(sender.state, SenderState::WaitingForAck);

    println!("PASS");


    // ========================================================
    // 9. RETRY EXHAUSTED → FAULT
    // ========================================================

    println!("\n--- Test 9: Retry exhausted ---");

    sender.handle_ack_timeout(50_000, &mut hardware);

    assert_eq!(sender.retry_count, 5);
    assert_eq!(sender.state, SenderState::Fault);

    println!("PASS");


    // ========================================================
    // 10. ACK AFTER FAULT
    // ========================================================

    println!("\n--- Test 10: ACK after fault ---");

    let accepted = sender.receive_ack(2);

    assert!(accepted);
    assert_eq!(sender.state, SenderState::Idle);

    println!("PASS");


    println!("\n================================");
    println!("ALL TESTS PASSED");
    println!("================================");
}
