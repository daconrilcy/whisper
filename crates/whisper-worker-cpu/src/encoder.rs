use rusty_mp3::{Mp3Encoder, Mp3EncoderConfig};

pub struct LiveMp3Encoder {
    encoder: Mp3Encoder,
    finished: bool,
}

impl LiveMp3Encoder {
    pub fn new() -> Self {
        Self {
            encoder: Mp3Encoder::new(Mp3EncoderConfig {
                bitrate_kbps: 64,
                vbr_quality: None,
            }),
            finished: false,
        }
    }

    pub fn push_pcm(&mut self, samples: &[i16]) -> Result<Vec<Vec<u8>>, String> {
        if self.finished {
            return Err("InvalidRequest: live MP3 encoder is already finalized".into());
        }
        let mut packets = Vec::new();
        for chunk in samples.chunks(16_000) {
            self.encoder
                .push_pcm_s16(chunk, 1, 16_000)
                .map_err(|error| format!("MP3 encoding failed: {error}"))?;
            packets.extend(self.drain_packets()?);
        }
        Ok(packets)
    }

    pub fn finish(&mut self) -> Result<Vec<Vec<u8>>, String> {
        if !self.finished {
            self.encoder.finish();
            self.finished = true;
        }
        self.drain_packets()
    }

    fn drain_packets(&mut self) -> Result<Vec<Vec<u8>>, String> {
        let mut packets = Vec::new();
        loop {
            match self.encoder.next_packet() {
                Ok(packet) => packets.push(packet),
                Err(rusty_mp3::Error::Again | rusty_mp3::Error::Eof) => break,
                Err(error) => return Err(format!("MP3 encoding failed: {error}")),
            }
        }
        Ok(packets)
    }
}

impl Default for LiveMp3Encoder {
    fn default() -> Self {
        Self::new()
    }
}
