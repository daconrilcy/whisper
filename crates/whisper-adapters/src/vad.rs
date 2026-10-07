use webrtc_vad::{SampleRate, Vad, VadMode};

pub const FRAME_SAMPLES: usize = 320;

pub struct VoiceActivity {
    vad: Vad,
}

impl VoiceActivity {
    pub fn new() -> Self {
        Self {
            vad: Vad::new_with_rate_and_mode(SampleRate::Rate16kHz, VadMode::LowBitrate),
        }
    }

    pub fn is_speech(&mut self, frame: &[i16]) -> Result<bool, String> {
        if frame.len() != FRAME_SAMPLES {
            return Err(format!(
                "VAD frame must contain exactly {FRAME_SAMPLES} samples"
            ));
        }
        self.vad
            .is_voice_segment(frame)
            .map_err(|_| "VAD rejected the PCM frame".to_owned())
    }
}

impl Default for VoiceActivity {
    fn default() -> Self {
        Self::new()
    }
}
