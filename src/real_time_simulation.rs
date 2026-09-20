use std::io::{self, BufRead};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::lora::{CommState, FakeLoraHardware, LoraPacket, LoraSender, SenderState};

use crate::enum_and_transition::{PumpEvent, PumpState, TankLevel};

use crate::pump_controller::{FakePumpHardware, PumpController, apply_output};

// ============================================================
// REAL-TIME SIMULATION
// ============================================================

pub fn run() {
    println!();
    println!("============================================================");
    println!("        WATER TANK AUTOMATOR - REAL TIME SIMULATION");
    println!("============================================================");
    println!();

    println!("Commands:");
    println!("  l  -> Tank LOW");
    println!("  n  -> Tank NORMAL");
    println!("  f  -> Tank FULL");
    println!("  e  -> Tank SENSOR FAULT");
    println!("  x  -> RESET pump controller");
    println!();
    println!("  d  -> Drop next packet");
    println!("  a  -> Drop next ACK");
    println!("  c  -> Toggle LoRa communication link");
    println!("  s  -> Show current status");
    println!("  q  -> Quit");
    println!();

    // --------------------------------------------------------
    // Fake hardware
    // --------------------------------------------------------

    let mut sender_hardware = FakeLoraHardware::new();

    let mut receiver_hardware = FakeLoraHardware::new();

    let mut pump_hardware = FakePumpHardware::new();

    // --------------------------------------------------------
    // Communication
    // --------------------------------------------------------

    let mut sender = LoraSender::new();

    let mut receiver = CommState::new();

    // --------------------------------------------------------
    // Pump controller
    // --------------------------------------------------------

    let mut pump_controller = PumpController::new();

    // --------------------------------------------------------
    // Simulation state
    // --------------------------------------------------------

    let start_time = Instant::now();

    let mut current_tank_level = TankLevel::Normal;

    let mut next_sequence: u32 = 1;

    let mut drop_next_packet = false;

    let mut drop_next_ack = false;

    let mut lora_link_enabled = true;

    // --------------------------------------------------------
    // Keyboard input thread
    // --------------------------------------------------------

    let (command_sender, command_receiver) = mpsc::channel::<String>();

    thread::spawn(move || {
        let stdin = io::stdin();

        for line in stdin.lock().lines() {
            match line {
                Ok(command) => {
                    if command_sender.send(command).is_err() {
                        break;
                    }
                }

                Err(_) => {
                    break;
                }
            }
        }
    });

    // --------------------------------------------------------
    // Main real-time loop
    // --------------------------------------------------------

    loop {
        let current_ms = start_time.elapsed().as_millis() as u32;

        // ====================================================
        // Process user commands
        // ====================================================

        while let Ok(command) = command_receiver.try_recv() {
            let command = command.trim().to_lowercase();

            match command.as_str() {
                "l" => {
                    current_tank_level = TankLevel::Low;

                    println!();
                    println!("[USER] Tank -> LOW");

                    send_tank_packet(
                        current_tank_level,
                        current_ms,
                        &mut next_sequence,
                        &mut sender,
                        &mut sender_hardware,
                        &mut receiver,
                        &mut receiver_hardware,
                        &mut pump_controller,
                        &mut pump_hardware,
                        &mut lora_link_enabled,
                        &mut drop_next_packet,
                        &mut drop_next_ack,
                    );
                }

                "n" => {
                    current_tank_level = TankLevel::Normal;

                    println!();
                    println!("[USER] Tank -> NORMAL");

                    send_tank_packet(
                        current_tank_level,
                        current_ms,
                        &mut next_sequence,
                        &mut sender,
                        &mut sender_hardware,
                        &mut receiver,
                        &mut receiver_hardware,
                        &mut pump_controller,
                        &mut pump_hardware,
                        &mut lora_link_enabled,
                        &mut drop_next_packet,
                        &mut drop_next_ack,
                    );
                }

                "f" => {
                    current_tank_level = TankLevel::Full;

                    println!();
                    println!("[USER] Tank -> FULL");

                    send_tank_packet(
                        current_tank_level,
                        current_ms,
                        &mut next_sequence,
                        &mut sender,
                        &mut sender_hardware,
                        &mut receiver,
                        &mut receiver_hardware,
                        &mut pump_controller,
                        &mut pump_hardware,
                        &mut lora_link_enabled,
                        &mut drop_next_packet,
                        &mut drop_next_ack,
                    );
                }

                "e" => {
                    current_tank_level = TankLevel::Fault;

                    println!();
                    println!("[USER] Tank -> SENSOR FAULT");

                    send_tank_packet(
                        current_tank_level,
                        current_ms,
                        &mut next_sequence,
                        &mut sender,
                        &mut sender_hardware,
                        &mut receiver,
                        &mut receiver_hardware,
                        &mut pump_controller,
                        &mut pump_hardware,
                        &mut lora_link_enabled,
                        &mut drop_next_packet,
                        &mut drop_next_ack,
                    );
                }

                "x" => {
                    println!();
                    println!("[USER] RESET");

                    let output = pump_controller.handle_event(PumpEvent::Reset, current_ms);

                    apply_output(output, &mut pump_hardware);

                    println!("Pump controller reset.");
                }

                "d" => {
                    drop_next_packet = true;

                    println!();
                    println!("[TEST] Next packet will be LOST.");
                }

                "a" => {
                    drop_next_ack = true;

                    println!();
                    println!("[TEST] Next ACK will be LOST.");
                }

                "c" => {
                    lora_link_enabled = !lora_link_enabled;

                    println!();

                    if lora_link_enabled {
                        println!("[TEST] LoRa link -> ENABLED");
                    } else {
                        println!("[TEST] LoRa link -> DISABLED");
                    }
                }

                "s" => {
                    print_status(
                        current_ms,
                        current_tank_level,
                        &sender,
                        &pump_controller,
                        &pump_hardware,
                        lora_link_enabled,
                    );
                }

                "q" => {
                    println!();
                    println!("Simulation stopped.");
                    return;
                }

                "" => {}

                _ => {
                    println!();
                    println!("[USER] Unknown command: {}", command);
                }
            }
        }

        // ====================================================
        // ACK timeout / retry processing
        // ====================================================

        let retry_count_before = sender.retry_count;

        sender.handle_ack_timeout(current_ms, &mut sender_hardware);

        let retry_happened = sender.retry_count != retry_count_before;

        if retry_happened {
            println!();
            println!(
                "[{} ms] ACK TIMEOUT -> RETRY {}",
                current_ms, sender.retry_count
            );

            // ------------------------------------------------
            // The retry transmission is now travelling through
            // our simulated LoRa link.
            // ------------------------------------------------

            if !lora_link_enabled {
                println!("[{} ms] LoRa link DOWN -> retry lost", current_ms);
            } else if drop_next_packet {
                println!(
                    "[{} ms] TEST -> retry packet intentionally lost",
                    current_ms
                );

                drop_next_packet = false;
            } else {
                process_pending_packet(
                    current_ms,
                    &mut sender,
                    &mut receiver,
                    &mut receiver_hardware,
                    &mut pump_controller,
                    &mut pump_hardware,
                    &mut drop_next_ack,
                );
            }
        }

        // ====================================================
        // Sender retry exhaustion
        // ====================================================

        if sender.state == SenderState::Fault {
            println!();
            println!(
                "[{} ms] SENDER FAULT -> maximum retries exceeded",
                current_ms
            );
        }

        // ====================================================
        // Communication timeout
        // ====================================================

        if pump_controller.check_comm_timeout(current_ms)
            && pump_controller.state() != PumpState::Fault
        {
            println!();
            println!("[{} ms] COMMUNICATION TIMEOUT", current_ms);

            let output = pump_controller.handle_comm_timeout(current_ms);

            apply_output(output, &mut pump_hardware);

            println!("[{} ms] Pump -> OFF", current_ms);
            println!("[{} ms] Controller -> FAULT", current_ms);
        }

        // ====================================================
        // Maximum pump runtime
        // ====================================================

        if pump_controller.check_max_run_timeout(current_ms)
            && pump_controller.state() != PumpState::Fault
        {
            println!();
            println!("[{} ms] MAXIMUM PUMP RUNTIME EXCEEDED", current_ms);

            let output = pump_controller.handle_max_run_timeout(current_ms);

            apply_output(output, &mut pump_hardware);

            println!("[{} ms] Pump -> OFF", current_ms);
            println!("[{} ms] Controller -> FAULT", current_ms);
        }

        // ====================================================
        // Print periodic status
        // ====================================================

        if current_ms % 1_000 < 100 {
            print_status(
                current_ms,
                current_tank_level,
                &sender,
                &pump_controller,
                &pump_hardware,
                lora_link_enabled,
            );
        }

        // ====================================================
        // Real-time loop period
        // ====================================================

        thread::sleep(Duration::from_millis(100));
    }
}

// ============================================================
// SEND NEW TANK PACKET
// ============================================================
#[allow(clippy::too_many_arguments)]
fn send_tank_packet(
    tank_level: TankLevel,
    current_ms: u32,
    next_sequence: &mut u32,
    sender: &mut LoraSender,
    sender_hardware: &mut FakeLoraHardware,
    receiver: &mut CommState,
    receiver_hardware: &mut FakeLoraHardware,
    pump_controller: &mut PumpController,
    pump_hardware: &mut FakePumpHardware,
    lora_link_enabled: &mut bool,
    drop_next_packet: &mut bool,
    drop_next_ack: &mut bool,
) {
    let seq = *next_sequence;

    *next_sequence = next_sequence.wrapping_add(1);

    let mut packet = LoraPacket {
        device_id: 1,
        tank_level,
        seq,
        crc: 0,
    };

    packet.crc = CommState::generate_crc(&packet);

    println!();
    println!(
        "[{} ms] Sending packet: seq={}, level={:?}",
        current_ms, packet.seq, packet.tank_level
    );

    sender.start_transmission(&packet, current_ms, sender_hardware);

    // --------------------------------------------------------
    // Simulated LoRa link
    // --------------------------------------------------------

    if !*lora_link_enabled {
        println!("[{} ms] LoRa link DOWN -> packet lost", current_ms);
        return;
    }

    if *drop_next_packet {
        println!("[{} ms] TEST -> packet intentionally lost", current_ms);

        *drop_next_packet = false;
        return;
    }

    // --------------------------------------------------------
    // Packet reaches receiver
    // --------------------------------------------------------

    process_pending_packet(
        current_ms,
        sender,
        receiver,
        receiver_hardware,
        pump_controller,
        pump_hardware,
        drop_next_ack,
    );
}

// ============================================================
// PROCESS PENDING PACKET AT RECEIVER
// ============================================================

fn process_pending_packet(
    current_ms: u32,
    sender: &mut LoraSender,
    receiver: &mut CommState,
    receiver_hardware: &mut FakeLoraHardware,
    pump_controller: &mut PumpController,
    pump_hardware: &mut FakePumpHardware,
    drop_next_ack: &mut bool,
) {
    let packet = match sender.pending_packet.clone() {
        Some(packet) => packet,

        None => {
            return;
        }
    };

    println!("[{} ms] Receiver got packet seq={}", current_ms, packet.seq);

    // --------------------------------------------------------
    // Process packet
    // --------------------------------------------------------

    let tank_level = receiver.process_packet(packet, receiver_hardware);

    // --------------------------------------------------------
    // Packet was accepted as NEW
    // --------------------------------------------------------

    if let Some(level) = tank_level {
        println!(
            "[{} ms] Receiver accepted packet -> {:?}",
            current_ms, level
        );

        pump_controller.record_valid_packet(current_ms);

        let event = PumpEvent::TankLevel {
            level_of_tank: level,
        };

        let output = pump_controller.handle_event(event, current_ms);

        apply_output(output, pump_hardware);

        println!(
            "[{} ms] Pump state -> {:?}",
            current_ms,
            pump_controller.state()
        );

        println!(
            "[{} ms] Pump output -> {}",
            current_ms,
            if pump_hardware.pump_is_on {
                "ON"
            } else {
                "OFF"
            }
        );
    } else {
        println!(
            "[{} ms] Receiver did NOT generate a new tank event",
            current_ms
        );
    }

    // --------------------------------------------------------
    // ACK handling
    // --------------------------------------------------------

    if let Some(ack_seq) = receiver_hardware.last_ack {
        if *drop_next_ack {
            println!(
                "[{} ms] TEST -> ACK {} intentionally lost",
                current_ms, ack_seq
            );

            *drop_next_ack = false;
        } else {
            println!("[{} ms] ACK {} delivered to sender", current_ms, ack_seq);

            let accepted = sender.receive_ack(ack_seq);

            if accepted {
                println!("[{} ms] Sender -> IDLE", current_ms);
            } else {
                println!("[{} ms] Sender rejected ACK {}", current_ms, ack_seq);
            }
        }
    }
}

// ============================================================
// STATUS
// ============================================================

fn print_status(
    current_ms: u32,
    tank_level: TankLevel,
    sender: &LoraSender,
    pump_controller: &PumpController,
    pump_hardware: &FakePumpHardware,
    lora_link_enabled: bool,
) {
    println!(
        "[STATUS {:>7} ms] Tank={:?} | Sender={:?} | Pump={:?} | Output={} | LoRa={}",
        current_ms,
        tank_level,
        sender.state,
        pump_controller.state(),
        if pump_hardware.pump_is_on {
            "ON"
        } else {
            "OFF"
        },
        if lora_link_enabled { "UP" } else { "DOWN" },
    );
}
