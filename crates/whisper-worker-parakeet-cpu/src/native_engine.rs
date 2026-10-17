use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

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
        let started = Instant::now();
        let result = self.recognizer.transcribe(&audio.samples);
        if let Some(log) = std::env::var_os("PARAKEET_R6_TIMING_LOG") {
            log_native_timing(PathBuf::from(log), audio.samples.len(), started.elapsed())?;
        }
        let result = result?;
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
    let mut lexical: Vec<LexicalWord> = Vec::with_capacity(native_words.len());
    for (index, word) in native_words.iter().enumerate() {
        let text = word.text.trim();
        if text.is_empty() {
            return Err(format!("InvalidWord: word {index} is empty"));
        }
        if is_punctuation_only(text) {
            let Some(previous) = lexical.last_mut() else {
                return Err(format!(
                    "InvalidWord: punctuation word {index} has no preceding word"
                ));
            };
            previous.text.push_str(text);
            continue;
        }
        if word.start_ms < 0 || word.end_ms <= word.start_ms {
            return Err(format!(
                "InvalidTimestamp: word {index} text={text:?} has non-positive duration; raw={}..{}",
                word.start_ms, word.end_ms
            ));
        }
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
                    "InvalidTimestamp: word {index} text={text:?} raw={}..{} exceeds audio duration {duration_ms} ms",
                    word.start_ms, word.end_ms
                ));
            }
        }
        let start_ms = word.start_ms as u32;
        if start_ms >= end_ms || u64::from(end_ms) > duration_ms {
            return Err(format!(
                "InvalidTimestamp: word {index} text={text:?} raw={}..{} is outside audio bounds {duration_ms} ms",
                word.start_ms, word.end_ms
            ));
        }
        lexical.push(LexicalWord {
            native_index: index,
            text: text.to_owned(),
            raw_start_ms: word.start_ms,
            raw_end_ms: word.end_ms,
            start_ms,
            end_ms,
        });
    }

    let mut words: Vec<Word> = Vec::with_capacity(lexical.len());
    let mut index = 0;
    while index < lexical.len() {
        let current = lexical[index].clone();
        let mut start_ms = current.start_ms;
        let end_ms = current.end_ms;
        if let Some(previous) = words.last_mut() {
            if start_ms < previous.start_ms {
                let overlap_ms = previous.end_ms.saturating_sub(start_ms);
                if overlap_ms > TDT_FRAME_MS {
                    return Err(raw_overlap_error(&lexical[index - 1], &current, overlap_ms));
                }
            }
            if start_ms < previous.end_ms {
                let overlap_ms = previous.end_ms - start_ms;
                if overlap_ms > TDT_FRAME_MS {
                    return Err(raw_overlap_error(&lexical[index - 1], &current, overlap_ms));
                }
                if previous.end_ms < end_ms {
                    start_ms = previous.end_ms;
                    // The next lexical overlap is evaluated against the interval
                    // that survived normalization, while raw_* stays diagnostic.
                    lexical[index].start_ms = start_ms;
                } else if previous.start_ms < start_ms {
                    previous.end_ms = start_ms;
                    lexical[index - 1].end_ms = start_ms;
                } else {
                    let run_start = overlapping_run_start(&lexical, index);
                    let share_start = (run_start..index).find(|candidate| {
                        shareable_interval(&lexical[*candidate..=index]).is_some()
                    });
                    let Some(share_start) = share_start else {
                        let shared = &lexical[run_start..=index];
                        let (common_start, common_end) = common_interval(shared);
                        return Err(format!(
                            "InvalidTimestamp: common interval {common_start}..{common_end} ms cannot give {} words positive integer-millisecond durations; raw words: {}",
                            shared.len(),
                            raw_word_list(shared)
                        ));
                    };
                    let shared = &lexical[share_start..=index];
                    let (common_start, common_end) =
                        shareable_interval(shared).expect("selected interval must be shareable");
                    let duration = common_end - common_start;
                    words.truncate(share_start);
                    let base = duration / shared.len() as u32;
                    let remainder = duration % shared.len() as u32;
                    let mut cursor = common_start;
                    for (part, native) in shared.iter().enumerate() {
                        let part_duration = base + u32::from((part as u32) < remainder);
                        let part_end = cursor + part_duration;
                        words.push(Word {
                            text: native.text.clone(),
                            start_ms: cursor,
                            end_ms: part_end,
                        });
                        cursor = part_end;
                    }
                    index += 1;
                    continue;
                }
            }
            if previous.start_ms >= previous.end_ms {
                return Err(format!(
                    "InvalidTimestamp: normalization removed word duration; raw words: {}",
                    raw_word_list(&lexical[index - 1..=index])
                ));
            }
        }
        if start_ms >= end_ms {
            return Err(format!(
                "InvalidTimestamp: word {} text={:?} normalization removed duration; raw={}..{}",
                current.native_index, current.text, current.raw_start_ms, current.raw_end_ms
            ));
        }
        words.push(Word {
            text: current.text.clone(),
            start_ms,
            end_ms,
        });
        index += 1;
    }
    Ok(words)
}

#[derive(Clone)]
struct LexicalWord {
    native_index: usize,
    text: String,
    raw_start_ms: i32,
    raw_end_ms: i32,
    start_ms: u32,
    end_ms: u32,
}

fn overlapping_run_start(words: &[LexicalWord], end: usize) -> usize {
    let mut start = end;
    while start > 0 {
        let previous = &words[start - 1];
        let current = &words[start];
        let common_start = previous.start_ms.max(current.start_ms);
        let common_end = previous.end_ms.min(current.end_ms);
        let overlap = common_end.saturating_sub(common_start);
        if overlap == 0 || overlap > TDT_FRAME_MS {
            break;
        }
        start -= 1;
    }
    start
}

fn common_interval(words: &[LexicalWord]) -> (u32, u32) {
    (
        words.iter().map(|word| word.start_ms).max().unwrap_or(0),
        words.iter().map(|word| word.end_ms).min().unwrap_or(0),
    )
}

fn shareable_interval(words: &[LexicalWord]) -> Option<(u32, u32)> {
    let (start, end) = common_interval(words);
    (end.saturating_sub(start) as usize >= words.len()).then_some((start, end))
}

fn raw_overlap_error(previous: &LexicalWord, current: &LexicalWord, overlap_ms: u32) -> String {
    format!(
        "InvalidTimestamp: overlap {overlap_ms} ms exceeds {TDT_FRAME_MS} ms; raw words: {}",
        raw_word_list(&[previous.clone(), current.clone()])
    )
}

fn raw_word_list(words: &[LexicalWord]) -> String {
    words
        .iter()
        .map(|word| {
            format!(
                "index={} text={:?} start={} end={}",
                word.native_index, word.text, word.raw_start_ms, word.raw_end_ms
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn log_native_timing(
    path: PathBuf,
    sample_count: usize,
    elapsed: std::time::Duration,
) -> Result<(), String> {
    let runtime = std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
        .map(PathBuf::from)
        .ok_or_else(|| "R6 timing log requires PARAKEET_L2_RUNTIME_ROOT".to_owned())?
        .canonicalize()
        .map_err(|error| format!("R6 timing runtime root: {error}"))?;
    let r6_root = runtime
        .join("r6")
        .canonicalize()
        .map_err(|error| format!("R6 timing root: {error}"))?;
    let parent = path
        .parent()
        .ok_or_else(|| "R6 timing log path has no parent".to_owned())?
        .canonicalize()
        .map_err(|error| format!("R6 timing log parent: {error}"))?;
    if !parent.starts_with(&r6_root) {
        return Err("R6 timing log path is outside the R6 execution root".into());
    }
    let canonical_file = parent.join(
        path.file_name()
            .ok_or_else(|| "R6 timing log path has no file name".to_owned())?,
    );
    if canonical_file.exists()
        && !canonical_file
            .canonicalize()
            .map_err(|error| format!("R6 timing log file: {error}"))?
            .starts_with(&r6_root)
    {
        return Err("R6 timing log file resolves outside the R6 execution root".into());
    }
    let case = std::env::var("PARAKEET_R6_CASE").unwrap_or_else(|_| "unspecified".into());
    let record = serde_json::json!({
        "case": case,
        "sample_count": sample_count,
        "inference_elapsed_ms": elapsed.as_millis(),
    });
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(canonical_file)
        .map_err(|error| format!("R6 timing log write: {error}"))?;
    serde_json::to_writer(&mut file, &record)
        .map_err(|error| format!("R6 timing log serialize: {error}"))?;
    file.write_all(b"\n")
        .map_err(|error| format!("R6 timing log write: {error}"))
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
    fn non_positive_raw_timestamps_are_reported_with_text_and_bounds() {
        let zero_duration = normalize_words(&[word("zero", 150, 150)], 1_000).unwrap_err();
        assert!(zero_duration.contains("text=\"zero\""), "{zero_duration}");
        assert!(zero_duration.contains("raw=150..150"), "{zero_duration}");

        let negative_start = normalize_words(&[word("negative", -10, 10)], 1_000).unwrap_err();
        assert!(
            negative_start.contains("text=\"negative\""),
            "{negative_start}"
        );
        assert!(negative_start.contains("raw=-10..10"), "{negative_start}");
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
        assert!(error.contains("overlap 81 ms exceeds 80 ms"));
        assert!(error.contains("index=0 text=\"a\" start=0 end=100"));
        assert!(error.contains("index=1 text=\"b\" start=19 end=120"));
    }

    #[test]
    fn same_tdt_frame_is_shared_as_equal_integer_millisecond_parts() {
        let checked = normalize_words(&[word("a", 0, 80), word("b", 0, 80)], 1_000).unwrap();
        assert_eq!(
            checked
                .iter()
                .map(|word| (word.start_ms, word.end_ms))
                .collect::<Vec<_>>(),
            vec![(0, 40), (40, 80)]
        );
    }

    #[test]
    fn three_words_in_one_tdt_frame_share_remainder_in_order() {
        let checked = normalize_words(
            &[
                word("a", 100, 180),
                word("b", 100, 180),
                word("c", 100, 180),
            ],
            1_000,
        )
        .unwrap();
        assert_eq!(
            checked
                .iter()
                .map(|word| (word.start_ms, word.end_ms))
                .collect::<Vec<_>>(),
            vec![(100, 127), (127, 154), (154, 180)]
        );
    }

    #[test]
    fn shareable_suffix_is_split_without_changing_the_preceding_word() {
        let checked = normalize_words(
            &[
                word("you'll", 437_200, 437_440),
                word("give", 437_360, 437_520),
                word("them", 437_440, 437_520),
            ],
            600_000,
        )
        .unwrap();
        assert_eq!(
            checked
                .iter()
                .map(|word| (word.start_ms, word.end_ms))
                .collect::<Vec<_>>(),
            vec![(437_200, 437_440), (437_440, 437_480), (437_480, 437_520)]
        );
        assert_eq!(checked[0].text, "you'll");
    }

    #[test]
    fn nested_words_with_an_unseparable_suffix_share_the_common_interval() {
        let checked = normalize_words(
            &[
                word("outer", 100, 180),
                word("inner", 100, 180),
                word("nested", 110, 170),
            ],
            1_000,
        )
        .unwrap();
        assert_eq!(
            checked
                .iter()
                .map(|word| (word.start_ms, word.end_ms))
                .collect::<Vec<_>>(),
            vec![(110, 130), (130, 150), (150, 170)]
        );
    }

    #[test]
    fn real_en60_the_eye_same_frame_is_split_at_618880_ms() {
        let checked = normalize_words(
            &[
                word("the", 618_880, 618_960),
                word("eye.", 618_880, 618_960),
            ],
            3_600_000,
        )
        .unwrap();
        assert_eq!(checked[0].text, "the");
        assert_eq!(checked[1].text, "eye.");
        assert_eq!((checked[0].start_ms, checked[0].end_ms), (618_880, 618_920));
        assert_eq!((checked[1].start_ms, checked[1].end_ms), (618_920, 618_960));
    }

    #[test]
    fn interval_shorter_than_word_count_is_rejected_with_raw_bounds() {
        let error = normalize_words(&[word("a", 10, 11), word("b", 10, 11)], 1_000).unwrap_err();
        assert!(error.contains("cannot give 2 words positive integer-millisecond durations"));
        assert!(error.contains("index=0 text=\"a\" start=10 end=11"));
        assert!(error.contains("index=1 text=\"b\" start=10 end=11"));
    }

    #[test]
    fn overlap_over_eighty_ms_reports_both_raw_intervals() {
        let error = normalize_words(&[word("a", 0, 100), word("b", 19, 120)], 1_000).unwrap_err();
        assert!(error.contains("overlap 81 ms exceeds 80 ms"));
        assert!(error.contains("index=0 text=\"a\" start=0 end=100"));
        assert!(error.contains("index=1 text=\"b\" start=19 end=120"));
    }
}
