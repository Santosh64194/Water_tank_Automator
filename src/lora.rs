use crate::{
    enum_and_transition::{TankLevel, tanklevel_to_bytes},
};

enum SenderState {
    Idle,
    WaitingForAck,
    Fault,
}

struct LoraSender {
    state: SenderState,
    waiting_seq: Option<u32>,
    sent_at_ms: Option<u32>,
    retry_count: u8,
}

impl LoraSender {
    pub fn new() -> Self {
        LoraSender {
            state: SenderState::Idle,
            waiting_seq: None,
            sent_at_ms: None,
            retry_count: 0,
        }
    }

	pub fn start_transmission(&mut self, seq: u32, current_ms: u32) {
		self.state = SenderState::WaitingForAck;
		self.sent_at_ms = Some(current_ms);
		self.waiting_seq = Some(seq);
	}

	pub fn receive_ack(&mut self, ack_seq: u32) -> bool {
		match self.waiting_seq {
			Some(value) => {
				if ack_seq != value {
					return false;
				}
				self.state = SenderState::Idle;
				self.sent_at_ms = None;
				self.waiting_seq = None;
				self.retry_count = 0;
				true
			},
			None => false
		}
	}

	pub fn check_ack_timeout(&mut self, current_ms: u32) -> bool {
		const MAX_RETRIES: u8 = 5;
		const ACK_TIMEOUT_MS: u32 = 5_000;

		match self.sent_at_ms {
			Some(sent_ms) => {
				let elapsed_time = current_ms.wrapping_sub(sent_ms);

				elapsed_time >= ACK_TIMEOUT_MS && self.retry_count < MAX_RETRIES
			}

			None => false,
		}
	}
}

pub struct LoraPacket {
    pub device_id: u8,
    pub tank_level: TankLevel,
    pub seq: u32,
    pub crc: u16,
}

pub struct CommState {
    pub last_received_seq: Option<u32>,
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

    fn accept_sequence(&mut self, seq: u32) -> bool {
        match self.last_received_seq {
            None => {
                self.last_received_seq = Some(seq);
                true
            }

            Some(old_seq) => {
                if CommState::is_newer_sequence(seq, old_seq) {
                    self.last_received_seq = Some(seq);
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn accept_packet(&mut self, packet: LoraPacket) -> Option<TankLevel> {
        if !Self::verify_crc(&packet) {
            return None;
        }

        if !self.accept_sequence(packet.seq) {
            return None;
        }

        Some(packet.tank_level)
    }

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
}
