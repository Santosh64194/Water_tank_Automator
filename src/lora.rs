use crate::enum_and_transition::TankLevel;

// Fake Hardware

pub trait LoraHardware {
    fn send(&mut self, packet: &LoraPacket);
    fn send_ack(&mut self, ack: &AckPacket);
}

pub struct FakeLoraHardware {
    pub last_packet: Option<u32>,
    pub last_ack: Option<u32>,
}

impl Default for FakeLoraHardware {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeLoraHardware {
    pub fn new() -> Self {
        FakeLoraHardware {
            last_packet: None,
            last_ack: None,
        }
    }
}

impl LoraHardware for FakeLoraHardware {
    fn send(&mut self, packet: &LoraPacket) {
        self.last_packet = Some(packet.seq);

        println!("Fake LoRa: sent packet with sequence {}", packet.seq);
    }

    fn send_ack(&mut self, ack: &AckPacket) {
        self.last_ack = Some(ack.seq);

        println!("Fake LoRa: sent ACK for sequence {}", ack.seq);
    }
}

// ============================================================
// RECEIVED DATA
// ============================================================

#[derive(Debug, Clone)]
pub struct LoraPacket {
    pub device_id: u8,
    pub tank_level: TankLevel,
    pub seq: u32,
    pub crc: u16,
}

// ============================================================
// ACK
// ============================================================

#[derive(Debug, Clone)]
pub struct AckPacket {
    pub seq: u32,
}

pub fn create_ack(packet: &LoraPacket) -> AckPacket {
    AckPacket { seq: packet.seq }
}

// ============================================================
// Possible Packet Results
// ============================================================

pub enum PacketResult {
    Accepted {
        tanklevel: TankLevel,
        ack: AckPacket,
    },
    Duplicate {
        ack: AckPacket,
    },
    Invalid,
}

// ============================================================
// RECEIVER COMMUNICATION STATE
// ============================================================

pub struct CommState {
    pub last_received_seq: Option<u32>,
}

impl Default for CommState {
    fn default() -> Self {
        Self::new()
    }
}

impl CommState {
    pub fn new() -> Self {
        CommState {
            last_received_seq: None,
        }
    }

    fn is_newer_sequence(new_seq: u32, old_seq: u32) -> bool {
        let difference = new_seq.wrapping_sub(old_seq);

        difference != 0 && difference < (1u32 << 31)
    }

    pub fn accept_packet(&mut self, packet: LoraPacket) -> PacketResult {
        // 1. CRC check
        if !Self::verify_crc(&packet) {
            return PacketResult::Invalid;
        }

        // 2. Check sequence
        match self.last_received_seq {
            None => {
                self.last_received_seq = Some(packet.seq);

                PacketResult::Accepted {
                    tanklevel: packet.tank_level,
                    ack: create_ack(&packet),
                }
            }

            Some(old_seq) => {
                if Self::is_newer_sequence(packet.seq, old_seq) {
                    self.last_received_seq = Some(packet.seq);

                    PacketResult::Accepted {
                        tanklevel: packet.tank_level,
                        ack: create_ack(&packet),
                    }
                } else if packet.seq == old_seq {
                    PacketResult::Duplicate {
                        ack: create_ack(&packet),
                    }
                } else {
                    PacketResult::Invalid
                }
            }
        }
    }

    // ========================================================
    // CRC
    // ========================================================

    fn calculate_crc(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;

        for byte in data {
            crc ^= (*byte as u16) << 8;

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

    fn serialize_for_crc(packet: &LoraPacket) -> [u8; 6] {
        let seq_bytes = packet.seq.to_be_bytes();

        let mut data = [0u8; 6];

        data[0] = packet.device_id;
        data[1] = tanklevel_to_bytes(packet.tank_level);
        data[2..6].copy_from_slice(&seq_bytes);

        data
    }

    pub fn generate_crc(packet: &LoraPacket) -> u16 {
        let data = Self::serialize_for_crc(packet);

        Self::calculate_crc(&data)
    }

    pub fn verify_crc(packet: &LoraPacket) -> bool {
        let data = Self::serialize_for_crc(packet);

        let calculated_crc = Self::calculate_crc(&data);

        calculated_crc == packet.crc
    }

    pub fn process_packet<H: LoraHardware>(
        &mut self,
        packet: LoraPacket,
        hardware: &mut H,
    ) -> Option<TankLevel> {
        match self.accept_packet(packet) {
            PacketResult::Accepted { tanklevel, ack } => {
                hardware.send_ack(&ack);
                Some(tanklevel)
            }

            PacketResult::Duplicate { ack } => {
                hardware.send_ack(&ack);
                None
            }

            PacketResult::Invalid => None,
        }
    }
}

// ============================================================
// SENDER
// ============================================================

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SenderState {
    Idle,
    WaitingForAck,
    Fault,
}

pub struct LoraSender {
    pub state: SenderState,
    pub pending_packet: Option<LoraPacket>,
    pub sent_at_ms: Option<u32>,
    pub retry_count: u8,
}

impl Default for LoraSender {
    fn default() -> Self {
        Self::new()
    }
}

impl LoraSender {
    pub fn new() -> Self {
        LoraSender {
            state: SenderState::Idle,
            pending_packet: None,
            sent_at_ms: None,
            retry_count: 0,
        }
    }

    // --------------------------------------------------------
    // Start transmission
    // --------------------------------------------------------

    pub fn start_transmission<H: LoraHardware>(
        &mut self,
        packet: &LoraPacket,
        current_ms: u32,
        hardware: &mut H,
    ) {
        self.state = SenderState::WaitingForAck;
        self.sent_at_ms = Some(current_ms);
        self.pending_packet = Some(packet.clone());
        self.retry_count = 0;

        hardware.send(packet);
    }

    // --------------------------------------------------------
    // Receive ACK
    // --------------------------------------------------------

    pub fn receive_ack(&mut self, ack_seq: u32) -> bool {
        match self.pending_packet.as_ref() {
            Some(packet) => {
                if ack_seq != packet.seq || self.state != SenderState::WaitingForAck {
                    return false;
                }

                self.state = SenderState::Idle;
                self.sent_at_ms = None;
                self.pending_packet = None;
                self.retry_count = 0;

                true
            }

            None => false,
        }
    }

    // --------------------------------------------------------
    // Retry
    // --------------------------------------------------------

    pub fn retry_transmission<H: LoraHardware>(&mut self, current_ms: u32, hardware: &mut H) {
        if let Some(packet) = self.pending_packet.as_ref() {
            hardware.send(packet);
            self.retry_count += 1;
            self.sent_at_ms = Some(current_ms);
        }
    }

    // --------------------------------------------------------
    // ACK timeout
    // --------------------------------------------------------

    pub fn handle_ack_timeout<H: LoraHardware>(&mut self, current_ms: u32, hardware: &mut H) {
        const MAX_RETRIES: u8 = 5;
        const ACK_TIMEOUT_MS: u32 = 5_000;

        if let Some(sent_ms) = self.sent_at_ms {
            let elapsed_time = current_ms.wrapping_sub(sent_ms);

            if elapsed_time < ACK_TIMEOUT_MS {
                return;
            }

            if self.retry_count < MAX_RETRIES {
                self.retry_transmission(current_ms, hardware);
            } else {
                self.state = SenderState::Fault;
            }
        }
    }
}

// ============================================================
// TANK LEVEL SERIALIZATION
// ============================================================

pub fn tanklevel_to_bytes(level: TankLevel) -> u8 {
    match level {
        TankLevel::Unknown => 0,
        TankLevel::Low => 1,
        TankLevel::Normal => 2,
        TankLevel::Full => 3,
        TankLevel::Fault => 4,
    }
}
