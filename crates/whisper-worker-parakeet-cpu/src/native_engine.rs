use std::path::Path;

use whisper_core::parakeet_ipc::Word;

use crate::{decoder::DecodedAudio, native_ffi::NativeRecognizer};

pub const MAX_DURATION_MS: u64 = 1_800_000;
pub const TDT_FRAME_MS: u32 = 80;

pub struct Engine {
    recognizer: NativeRecognizer,
    native_version: String,
}

impl Engine {
    pub fn load(native_dir: &Path, model: &Path) -> Result<Self, String> {
        let recognizer = NativeRecognizer::load(native_dir, model)?;
        let native_version = recognizer.version()?;
        Ok(Self {
            recognizer,
            native_version,
        })
    }

    pub fn native_version(&self) -> &str {
        &self.native_version
    }

    pub fn transcribe(&mut self, audio: &DecodedAudio) -> Result<(String, Vec<Word>), String> {
        let duration_ms = (audio.samples.len() as u64 * 1000) / 16_000;
        if duration_ms == 0 || duration_ms > MAX_DURATION_MS {
            return Err("AudioTooLong: input must be at most 1,800 seconds".into());
        }
        let result = self.recognizer.transcribe(&audio.samples)?;
        let words = validate_words(&result.words, duration_ms)?;
        Ok((result.text, words))
    }
}

fn validate_words(
    native_words: &[crate::native_ffi::NativeWord],
    duration_ms: u64,
) -> Result<Vec<Word>, String> {
    let mut words = Vec::with_capacity(native_words.len());
    let mut previous_start = 0_u32;
    for (index, word) in native_words.iter().enumerate() {
        if word.start_ms < 0 || word.end_ms <= word.start_ms {
            return Err(format!(
                "InvalidTimestamp: word {index} has non-positive duration"
            ));
        }
        let start_ms = word.start_ms as u32;
        let mut end_ms = word.end_ms as u32;
        if index > 0 && start_ms < previous_start {
            return Err(format!(
                "InvalidTimestamp: word {index} starts before its predecessor"
            ));
        }
        if u64::from(end_ms) > duration_ms {
            if index + 1 == native_words.len()
                && u64::from(end_ms) <= duration_ms + u64::from(TDT_FRAME_MS)
            {
                end_ms = duration_ms as u32;
            } else {
                return Err(format!(
                    "InvalidTimestamp: word {index} exceeds the audio duration"
                ));
            }
        }
        if start_ms >= end_ms || u64::from(end_ms) > duration_ms {
            return Err(format!(
                "InvalidTimestamp: word {index} is outside audio bounds"
            ));
        }
        previous_start = start_ms;
        words.push(Word {
            text: word.text.clone(),
            start_ms,
            end_ms,
        });
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_ffi::{NativeResult, NativeWord};

    #[test]
    fn final_word_can_be_bounded_by_one_tdt_frame() {
        let duration_samples = 16_000;
        let duration_ms = duration_samples as u64 * 1000 / 16_000;
        let result = NativeResult {
            text: "tail".into(),
            words: vec![NativeWord {
                text: "tail".into(),
                start_ms: 900,
                end_ms: 1_080,
            }],
        };
        let checked = validate_words(&result.words, duration_ms).unwrap();
        assert_eq!(checked[0].end_ms, 1_000);
        assert!(
            validate_words(
                &[NativeWord {
                    text: "x".into(),
                    start_ms: 900,
                    end_ms: 1_081
                }],
                duration_ms
            )
            .is_err()
        );
    }

    #[test]
    fn timestamps_reject_invalid_order_and_non_final_overrun() {
        let words = [
            NativeWord {
                text: "one".into(),
                start_ms: 300,
                end_ms: 400,
            },
            NativeWord {
                text: "two".into(),
                start_ms: 200,
                end_ms: 500,
            },
        ];
        assert!(validate_words(&words, 1_000).is_err());
        let words = [NativeWord {
            text: "one".into(),
            start_ms: 900,
            end_ms: 1_081,
        }];
        assert!(validate_words(&words, 1_000).is_err());
    }
}
