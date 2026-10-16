use std::path::Path;

use whisper_core::parakeet_ipc::Word;

use crate::{decoder::DecodedAudio, native_ffi::NativeRecognizer};

pub const MAX_DURATION_MS: u64 = 1_800_000;
#[cfg(feature = "parakeet-qualification-reference")]
pub const MAX_REFERENCE_DURATION_MS: u64 = 3_600_000;
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
        self.transcribe_bounded(audio, MAX_DURATION_MS)
    }

    #[cfg(feature = "parakeet-qualification-reference")]
    pub fn transcribe_reference(
        &mut self,
        audio: &DecodedAudio,
    ) -> Result<(String, Vec<Word>), String> {
        self.transcribe_bounded(audio, MAX_REFERENCE_DURATION_MS)
    }

    fn transcribe_bounded(
        &mut self,
        audio: &DecodedAudio,
        duration_limit_ms: u64,
    ) -> Result<(String, Vec<Word>), String> {
        let duration_ms = (audio.samples.len() as u64 * 1000) / 16_000;
        if duration_ms == 0 || duration_ms > duration_limit_ms {
            return Err(format!(
                "AudioTooLong: input must be at most {} seconds",
                duration_limit_ms / 1_000
            ));
        }
        let result = self.recognizer.transcribe(&audio.samples)?;
        let words = normalize_words(&result.words, duration_ms)?;
        let text = words
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        Ok((text, words))
    }
}

fn normalize_words(
    native_words: &[crate::native_ffi::NativeWord],
    duration_ms: u64,
) -> Result<Vec<Word>, String> {
    let mut words: Vec<Word> = Vec::with_capacity(native_words.len());
    for (index, word) in native_words.iter().enumerate() {
        let text = word.text.trim();
        if text.is_empty() {
            return Err(format!("InvalidWord: word {index} is empty"));
        }
        if is_punctuation_only(text) {
            let Some(previous) = words.last_mut() else {
                return Err(format!(
                    "InvalidWord: punctuation word {index} has no preceding word"
                ));
            };
            previous.text.push_str(text);
            continue;
        }
        if word.start_ms < 0 || word.end_ms <= word.start_ms {
            return Err(format!(
                "InvalidTimestamp: word {index} has non-positive duration"
            ));
        }
        let mut start_ms = word.start_ms as u32;
        let mut end_ms = word.end_ms as u32;
        if u64::from(end_ms) > duration_ms {
            let has_later_lexical_word = native_words[index + 1..]
                .iter()
                .any(|later| !is_punctuation_only(later.text.trim()));
            if !has_later_lexical_word && u64::from(end_ms) <= duration_ms + u64::from(TDT_FRAME_MS)
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
        if let Some(previous) = words.last_mut() {
            if start_ms < previous.start_ms {
                let overlap_ms = previous.end_ms.saturating_sub(start_ms);
                if overlap_ms > TDT_FRAME_MS {
                    return Err(format!(
                        "InvalidTimestamp: word {index} overlaps its predecessor by more than {TDT_FRAME_MS} ms"
                    ));
                }
            }
            if start_ms < previous.end_ms {
                let overlap_ms = previous.end_ms - start_ms;
                if overlap_ms > TDT_FRAME_MS {
                    return Err(format!(
                        "InvalidTimestamp: word {index} overlaps its predecessor by more than {TDT_FRAME_MS} ms"
                    ));
                }
                if previous.end_ms < end_ms {
                    start_ms = previous.end_ms;
                } else if previous.start_ms < start_ms {
                    previous.end_ms = start_ms;
                } else {
                    return Err(format!(
                        "InvalidTimestamp: word {index} overlap cannot be normalized with positive durations"
                    ));
                }
            }
            if previous.start_ms >= previous.end_ms {
                return Err(format!(
                    "InvalidTimestamp: normalization removed word {} duration",
                    words.len() - 1
                ));
            }
        }
        if start_ms >= end_ms {
            return Err(format!(
                "InvalidTimestamp: word {index} normalization removed its duration"
            ));
        }
        words.push(Word {
            text: text.to_owned(),
            start_ms,
            end_ms,
        });
    }
    Ok(words)
}

fn is_punctuation_only(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|character| !character.is_alphanumeric() && !character.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_ffi::NativeWord;

    fn word(text: &str, start_ms: i32, end_ms: i32) -> NativeWord {
        NativeWord {
            text: text.into(),
            start_ms,
            end_ms,
        }
    }

    #[test]
    fn final_word_can_be_bounded_by_one_tdt_frame() {
        let duration_samples = 16_000;
        let duration_ms = duration_samples as u64 * 1000 / 16_000;
        let words = [NativeWord {
            text: "tail".into(),
            start_ms: 900,
            end_ms: 1_080,
        }];
        let checked = normalize_words(&words, duration_ms).unwrap();
        assert_eq!(checked[0].end_ms, 1_000);
        assert!(
            normalize_words(
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
        assert!(normalize_words(&words, 1_000).is_err());
        let words = [NativeWord {
            text: "one".into(),
            start_ms: 900,
            end_ms: 1_081,
        }];
        assert!(normalize_words(&words, 1_000).is_err());
    }

    #[test]
    fn punctuation_is_attached_without_changing_its_predecessor_interval() {
        let checked = normalize_words(
            &[
                word("faire", 17_760, 17_920),
                word("?", 17_920, 18_080),
                word("Je", 18_000, 18_080),
            ],
            27_000,
        )
        .unwrap();
        assert_eq!(
            checked,
            vec![
                Word {
                    text: "faire?".into(),
                    start_ms: 17_760,
                    end_ms: 17_920
                },
                Word {
                    text: "Je".into(),
                    start_ms: 18_000,
                    end_ms: 18_080
                },
            ]
        );
    }

    #[test]
    fn zero_overlap_preserves_both_intervals() {
        let checked = normalize_words(&[word("a", 0, 100), word("b", 100, 200)], 1_000).unwrap();
        assert_eq!((checked[1].start_ms, checked[1].end_ms), (100, 200));
        assert_eq!((checked[0].start_ms, checked[0].end_ms), (0, 100));
    }

    #[test]
    fn eighty_ms_overlap_moves_the_next_start_when_duration_remains_positive() {
        let checked = normalize_words(&[word("a", 0, 100), word("b", 20, 140)], 1_000).unwrap();
        assert_eq!((checked[1].start_ms, checked[1].end_ms), (100, 140));
        assert_eq!((checked[0].start_ms, checked[0].end_ms), (0, 100));
    }

    #[test]
    fn eighty_ms_overlap_shrinks_previous_when_next_would_become_empty() {
        let checked = normalize_words(&[word("a", 0, 100), word("b", 20, 100)], 1_000).unwrap();
        assert_eq!((checked[0].start_ms, checked[0].end_ms), (0, 20));
        assert_eq!((checked[1].start_ms, checked[1].end_ms), (20, 100));
    }

    #[test]
    fn eighty_one_ms_overlap_is_rejected() {
        let error = normalize_words(&[word("a", 0, 100), word("b", 19, 120)], 1_000).unwrap_err();
        assert!(error.contains("overlaps its predecessor by more than 80 ms"));
    }

    #[test]
    fn overlap_that_cannot_preserve_two_positive_durations_is_rejected() {
        let error = normalize_words(&[word("a", 0, 50), word("b", 0, 50)], 1_000).unwrap_err();
        assert!(error.contains("overlap cannot be normalized"));
    }
}
