use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

const PADDING_SIZES: [usize; 3] = [1024, 5120, 10240];

pub struct TrafficMorpher {
    rng: rand::rngs::ThreadRng,
}

impl TrafficMorpher {
    pub fn new() -> Self {
        Self { rng: rand::thread_rng() }
    }

    pub fn pad_to_uniform(&mut self, payload: &[u8]) -> Vec<u8> {
        let target_size = PADDING_SIZES
            .iter()
            .find(|&&s| s >= payload.len())
            .copied()
            .unwrap_or_else(|| {
                let excess = payload.len() % PADDING_SIZES[2];
                payload.len() + (PADDING_SIZES[2] - excess)
            });

        let mut padded = payload.to_vec();
        let padding_needed = target_size - payload.len();
        let mut padding = vec![0u8; padding_needed];
        self.rng.fill(padding.as_mut_slice());
        padded.push(b'\x00');
        padded.extend_from_slice(&(padding_needed as u16).to_le_bytes());
        padded.extend_from_slice(&padding);
        padded
    }

    pub fn strip_padding(padded: &[u8]) -> Option<&[u8]> {
        if padded.len() < 3 {
            return None;
        }
        let payload_end = padded.iter().rposition(|&b| b != 0)?;
        let len_start = payload_end.saturating_sub(2);
        let padding_len =
            u16::from_le_bytes([padded[len_start], padded[len_start + 1]]) as usize;
        let real_end = padded.len().saturating_sub(padding_len + 3);
        Some(&padded[..real_end])
    }

    pub async fn randomized_delay(&mut self) {
        let delay_ms: u64 = self.rng.gen_range(50..=500);
        sleep(Duration::from_millis(delay_ms)).await;
    }

    pub fn dummy_packet(&mut self) -> Vec<u8> {
        let size = PADDING_SIZES[self.rng.gen_range(0..PADDING_SIZES.len())];
        let mut buf = vec![0u8; size];
        self.rng.fill(buf.as_mut_slice());
        buf
    }
}

impl Default for TrafficMorpher {
    fn default() -> Self {
        Self::new()
    }
}
