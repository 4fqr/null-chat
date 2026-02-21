use rand::RngCore;
use zeroize::Zeroize;

pub struct PanicEngine;

const WIPE_FRAME_BYTES: usize = 131072;

impl PanicEngine {
    pub fn execute() -> ! {
        Self::terminate_tor_processes();
        Self::wipe_stack_entropy_frames();
        Self::wipe_heap_decoy_region();
        std::process::exit(0)
    }

    fn terminate_tor_processes() {
        let _ = std::process::Command::new("pkill")
            .args(["-SIGKILL", "-x", "tor"])
            .status();
        let _ = std::process::Command::new("pkill")
            .args(["-SIGKILL", "-x", "tor-real"])
            .status();
    }

    fn wipe_stack_entropy_frames() {
        let mut rng = rand::thread_rng();
        let mut frame_a = [0u8; WIPE_FRAME_BYTES];
        let mut frame_b = [0u8; WIPE_FRAME_BYTES];
        rng.fill_bytes(&mut frame_a);
        rng.fill_bytes(&mut frame_b);
        frame_a.zeroize();
        frame_b.zeroize();
    }

    fn wipe_heap_decoy_region() {
        let mut rng = rand::thread_rng();
        let mut region = vec![0u8; 2 * 1024 * 1024];
        rng.fill_bytes(&mut region);
        region.zeroize();
    }
}
