//! Conservative reconciliation of timestamped Parakeet microphone hypotheses.
//!
//! A word is publishable only after two distinct windows agree on its spelling
//! and native time. Unmatched words remain in the ledger, so a later match
//! cannot leap over an unresolved earlier word.

use whisper_core::SourceRange;

const SAMPLE_RATE: u32 = 16_000;
const TIME_TOLERANCE_SAMPLES: u64 = 2_560; // 160 ms
const UNCONFIRMED_TAIL_SAMPLES: u64 = 16_000;
const MAX_PENDING_WORDS: usize = 1_024;
const MAX_EVIDENCE_WINDOWS: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TimedWord {
    pub text: String,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub(super) struct Hypothesis {
    pub sequence: u64,
    pub range: SourceRange,
    pub words: Vec<TimedWord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ConfirmedBlock {
    pub segment_id: u64,
    pub text: String,
    pub range: SourceRange,
}

#[derive(Clone, Debug)]
pub(super) struct ConfirmationDecision {
    pub cursor_before: u64,
    pub cursor_after: u64,
    pub published: Vec<ConfirmedBlock>,
    pub first_refusal: Option<ConfirmationRefusal>,
    pub unresolved_before: Vec<SourceRange>,
    pub unresolved_after: Vec<SourceRange>,
}

#[derive(Clone, Debug)]
pub(super) struct ConfirmationRefusal {
    pub code: &'static str,
    pub range: Option<SourceRange>,
    pub gaps: Vec<SourceRange>,
    pub alignment_candidate: Option<AlignmentCandidate>,
}

#[derive(Clone, Debug)]
pub(super) struct AlignmentCandidate {
    pub sequence: u64,
    pub range: SourceRange,
    pub start_delta_samples: u64,
    pub end_delta_samples: u64,
    pub overlap_samples: u64,
    pub within_time_tolerance: bool,
    pub same_span: bool,
}

#[derive(Clone, Debug)]
struct TrackedWord {
    word: TimedWord,
    first_sequence: u64,
    first_window: SourceRange,
    last_sequence: u64,
    witnessed_twice: bool,
    conflict: bool,
}

#[derive(Default)]
pub(super) struct Confirmation {
    pending: Vec<TrackedWord>,
    confirmed: Vec<ConfirmedBlock>,
    unresolved: Vec<SourceRange>,
    confirmed_end: u64,
    next_segment_id: u64,
    latest_sequence: u64,
    latest_window_end: u64,
    covered_end: u64,
    evidence: Vec<Hypothesis>,
}

impl Confirmation {
    pub(super) fn confirmed(&self) -> &[ConfirmedBlock] {
        &self.confirmed
    }

    pub(super) fn confirmed_end(&self) -> u64 {
        self.confirmed_end
    }

    pub(super) fn unresolved(&self) -> &[SourceRange] {
        &self.unresolved
    }

    pub(super) fn recovery_ranges(&self) -> Vec<SourceRange> {
        let mut ranges = self.unresolved.clone();
        for tracked in &self.pending {
            if !tracked.witnessed_twice && !ranges.contains(&tracked.word.range) {
                ranges.push(tracked.word.range);
            }
        }
        ranges.sort_by_key(|range| range.start_sample);
        ranges
    }

    /// Portions for which ordinary future windows can no longer provide a
    /// second witness. These remain candidates only; a reread is still fed
    /// through `ingest_observed` and cannot bypass the normal proof rules.
    pub(super) fn recovery_candidates(&self, next_window_start: u64) -> Vec<SourceRange> {
        let mut ranges = self
            .unresolved
            .iter()
            .copied()
            .filter(|range| {
                range.end_sample.saturating_add(TIME_TOLERANCE_SAMPLES) < next_window_start
                    && !self
                        .confirmed
                        .iter()
                        .any(|block| ranges_overlap(block.range, *range))
            })
            .collect::<Vec<_>>();
        for tracked in &self.pending {
            let range = tracked.word.range;
            if !tracked.witnessed_twice
                && range.end_sample.saturating_add(TIME_TOLERANCE_SAMPLES) < next_window_start
                && !self
                    .confirmed
                    .iter()
                    .any(|block| ranges_overlap(block.range, range))
                && !ranges.contains(&range)
            {
                ranges.push(range);
            }
        }
        ranges.sort_by_key(|range| range.start_sample);
        ranges
    }

    pub(super) fn remaining_ranges(&self, durable_end: u64) -> Vec<SourceRange> {
        let mut ranges = self.unresolved.clone();
        for tracked in &self.pending {
            if !ranges.contains(&tracked.word.range) {
                ranges.push(tracked.word.range);
            }
        }
        if self.covered_end < durable_end {
            ranges.push(SourceRange {
                start_sample: self.covered_end,
                end_sample: durable_end,
                sample_rate_hz: SAMPLE_RATE,
            });
        }
        ranges.sort_by_key(|range| range.start_sample);
        ranges
    }

    pub(super) fn has_remainder(&self, durable_end: u64) -> bool {
        !self.pending.is_empty() || !self.unresolved.is_empty() || self.covered_end < durable_end
    }

    #[cfg(test)]
    pub(super) fn finalize(&mut self) -> Vec<ConfirmedBlock> {
        self.finalize_observed(false).published
    }

    pub(super) fn finalize_observed(
        &mut self,
        include_alignment_diagnostic: bool,
    ) -> ConfirmationDecision {
        let cursor_before = self.confirmed_end;
        let unresolved_before = self.unresolved.clone();
        // The capture has ended, so the last second is no longer a moving edge.
        // This only releases words already witnessed in two windows.
        self.latest_window_end = self
            .latest_window_end
            .saturating_add(UNCONFIRMED_TAIL_SAMPLES);
        let (published, first_refusal) = self.publish_ready(include_alignment_diagnostic);
        ConfirmationDecision {
            cursor_before,
            cursor_after: self.confirmed_end,
            published,
            first_refusal,
            unresolved_before,
            unresolved_after: self.unresolved.clone(),
        }
    }

    pub(super) fn mark_unresolved(&mut self, range: SourceRange) {
        if range.sample_rate_hz != SAMPLE_RATE || range.start_sample >= range.end_sample {
            return;
        }
        if !self.unresolved.contains(&range) {
            self.unresolved.push(range);
            self.unresolved.sort_by_key(|range| range.start_sample);
        }
    }

    #[cfg(test)]
    pub(super) fn ingest(&mut self, hypothesis: Hypothesis) -> Result<Vec<ConfirmedBlock>, String> {
        self.ingest_observed(hypothesis, false)
            .map(|decision| decision.published)
    }

    pub(super) fn ingest_observed(
        &mut self,
        hypothesis: Hypothesis,
        include_alignment_diagnostic: bool,
    ) -> Result<ConfirmationDecision, String> {
        let cursor_before = self.confirmed_end;
        let unresolved_before = self.unresolved.clone();
        if hypothesis.sequence <= self.latest_sequence
            || hypothesis.range.sample_rate_hz != SAMPLE_RATE
            || hypothesis.range.start_sample >= hypothesis.range.end_sample
        {
            return Err("Hypothèse Parakeet hors session ou chronologie".into());
        }
        let mut last_end = hypothesis.range.start_sample;
        for word in &hypothesis.words {
            if word.text.trim().is_empty()
                || word.range.sample_rate_hz != SAMPLE_RATE
                || word.range.start_sample < hypothesis.range.start_sample
                || word.range.end_sample > hypothesis.range.end_sample
                || word.range.start_sample >= word.range.end_sample
                || word.range.start_sample < last_end
            {
                return Err("Mot Parakeet sans horodatage natif valide".into());
            }
            last_end = word.range.end_sample;
        }
        let evidence = hypothesis.clone();
        if hypothesis.range.start_sample > self.covered_end && self.latest_sequence != 0 {
            self.mark_unresolved(SourceRange {
                start_sample: self.covered_end,
                end_sample: hypothesis.range.start_sample,
                sample_rate_hz: SAMPLE_RATE,
            });
        }
        self.covered_end = self.covered_end.max(hypothesis.range.end_sample);
        self.latest_sequence = hypothesis.sequence;
        // A targeted reread can be older than the latest regular window. Keep
        // the live confirmation frontier monotonic when accepting its proof.
        self.latest_window_end = self.latest_window_end.max(hypothesis.range.end_sample);

        let mut matched = vec![false; self.pending.len()];
        for word in hypothesis.words {
            if self
                .confirmed
                .iter()
                .any(|block| ranges_overlap(block.range, word.range))
            {
                continue;
            }
            if let Some((index, tracked)) =
                self.pending
                    .iter_mut()
                    .enumerate()
                    .find(|(index, tracked)| {
                        !matched[*index]
                            && tracked.last_sequence != hypothesis.sequence
                            && same_word(&tracked.word, &word)
                    })
            {
                matched[index] = true;
                tracked.last_sequence = hypothesis.sequence;
                tracked.witnessed_twice |= tracked.first_sequence != hypothesis.sequence
                    && tracked.first_window != hypothesis.range;
                // Keep the newest punctuation while preserving the established word time.
                tracked.word.text = word.text;
            } else {
                self.pending.push(TrackedWord {
                    word,
                    first_sequence: hypothesis.sequence,
                    first_window: hypothesis.range,
                    last_sequence: hypothesis.sequence,
                    witnessed_twice: false,
                    conflict: false,
                });
                matched.push(true);
            }
        }
        let superseded: Vec<_> = self
            .pending
            .iter()
            .filter(|old| {
                !old.witnessed_twice
                    && self.pending.iter().any(|replacement| {
                        replacement.witnessed_twice
                            && old.word.text != replacement.word.text
                            && old.last_sequence < replacement.first_sequence
                            && revision_has_context(old, replacement, &self.evidence)
                    })
            })
            .map(|old| old.word.range)
            .collect();
        self.pending
            .retain(|tracked| !superseded.contains(&tracked.word.range) || tracked.witnessed_twice);
        self.pending.sort_by_key(|tracked| {
            (
                tracked.word.range.start_sample,
                tracked.word.range.end_sample,
            )
        });
        for tracked in &mut self.pending {
            tracked.conflict = false;
        }
        for index in 1..self.pending.len() {
            let (before, after) = self.pending.split_at_mut(index);
            let prior = &mut before[index - 1];
            let current = &mut after[0];
            if current.word.range.start_sample < prior.word.range.end_sample {
                prior.conflict = true;
                current.conflict = true;
            }
        }
        // Once an unmatched word has left the newest window, future ordinary
        // windows cannot attest it. Retain it and its audio range for recovery.
        let newly_unresolved: Vec<_> = self
            .pending
            .iter()
            .filter(|tracked| {
                !tracked.witnessed_twice
                    && tracked
                        .word
                        .range
                        .end_sample
                        .saturating_add(TIME_TOLERANCE_SAMPLES)
                        < hypothesis.range.start_sample
            })
            .map(|tracked| tracked.word.range)
            .collect();
        for range in newly_unresolved {
            self.mark_unresolved(range);
        }
        if self.pending.len() > MAX_PENDING_WORDS {
            let excess = self.pending.len() - MAX_PENDING_WORDS;
            let dropped: Vec<_> = self
                .pending
                .drain(..excess)
                .map(|tracked| tracked.word.range)
                .collect();
            for range in dropped {
                self.mark_unresolved(range);
            }
        }
        self.evidence.push(evidence);
        if self.evidence.len() > MAX_EVIDENCE_WINDOWS {
            self.evidence.remove(0);
        }
        let evidence = &self.evidence;
        let pending = &self.pending;
        self.unresolved
            .retain(|range| !gap_fully_covered(*range, evidence, pending));
        let (published, first_refusal) = self.publish_ready(include_alignment_diagnostic);
        Ok(ConfirmationDecision {
            cursor_before,
            cursor_after: self.confirmed_end,
            published,
            first_refusal,
            unresolved_before,
            unresolved_after: self.unresolved.clone(),
        })
    }

    fn publish_ready(
        &mut self,
        include_alignment_diagnostic: bool,
    ) -> (Vec<ConfirmedBlock>, Option<ConfirmationRefusal>) {
        let cutoff = self
            .latest_window_end
            .saturating_sub(UNCONFIRMED_TAIL_SAMPLES);
        let mut published = Vec::new();
        let mut retained = Vec::with_capacity(self.pending.len());
        let mut run = Vec::new();
        let mut first_refusal = None;
        for tracked in std::mem::take(&mut self.pending) {
            let refusal = if !tracked.witnessed_twice {
                Some(ConfirmationRefusal {
                    code: "insufficient_evidence",
                    range: Some(tracked.word.range),
                    gaps: Vec::new(),
                    alignment_candidate: include_alignment_diagnostic
                        .then(|| self.first_unaligned_lexical_candidate(&tracked))
                        .flatten(),
                })
            } else if tracked.conflict {
                let lexical_conflict = self.confirmed.iter().any(|other| {
                    other.range != tracked.word.range
                        && ranges_overlap(other.range, tracked.word.range)
                        && normalized_word(&other.text) != normalized_word(&tracked.word.text)
                });
                Some(ConfirmationRefusal {
                    code: if lexical_conflict {
                        "lexical_conflict"
                    } else {
                        "temporal_disagreement"
                    },
                    range: Some(tracked.word.range),
                    gaps: Vec::new(),
                    alignment_candidate: None,
                })
            } else if tracked.word.range.end_sample > cutoff {
                Some(ConfirmationRefusal {
                    code: "reserved_tail",
                    range: Some(tracked.word.range),
                    gaps: Vec::new(),
                    alignment_candidate: None,
                })
            } else if self
                .confirmed
                .iter()
                .any(|block| ranges_overlap(block.range, tracked.word.range))
            {
                Some(ConfirmationRefusal {
                    code: "continuity_break",
                    range: Some(tracked.word.range),
                    gaps: Vec::new(),
                    alignment_candidate: None,
                })
            } else {
                let gaps: Vec<_> = self
                    .unresolved
                    .iter()
                    .copied()
                    .filter(|range| {
                        range.start_sample < tracked.word.range.end_sample
                            && range.end_sample > tracked.word.range.start_sample
                    })
                    .collect();
                (!gaps.is_empty()).then_some(ConfirmationRefusal {
                    code: "unresolved_gap",
                    range: Some(tracked.word.range),
                    gaps,
                    alignment_candidate: None,
                })
            };
            if let Some(refusal) = refusal {
                if first_refusal.is_none() {
                    first_refusal = Some(refusal);
                }
                if !run.is_empty() {
                    published.push(self.block_from_words(std::mem::take(&mut run)));
                }
                retained.push(tracked);
            } else {
                let word = tracked.word;
                if run.last().is_some_and(|last: &TimedWord| {
                    self.unresolved.iter().any(|range| {
                        range.start_sample < word.range.start_sample
                            && range.end_sample > last.range.end_sample
                    }) || self.confirmed.iter().any(|block| {
                        block.range.start_sample < word.range.start_sample
                            && block.range.end_sample > last.range.end_sample
                    })
                }) {
                    published.push(self.block_from_words(std::mem::take(&mut run)));
                }
                run.push(word);
            }
        }
        if !run.is_empty() {
            published.push(self.block_from_words(run));
        }
        self.pending = retained;
        (published, first_refusal)
    }

    fn first_unaligned_lexical_candidate(
        &self,
        tracked: &TrackedWord,
    ) -> Option<AlignmentCandidate> {
        let expected = normalized_word(&tracked.word.text);
        if expected.is_empty() {
            return None;
        }
        for hypothesis in self
            .evidence
            .iter()
            .filter(|hypothesis| hypothesis.sequence > tracked.first_sequence)
        {
            for candidate in &hypothesis.words {
                if normalized_word(&candidate.text) != expected
                    || same_word(&tracked.word, candidate)
                {
                    continue;
                }
                let overlap_start = tracked
                    .word
                    .range
                    .start_sample
                    .max(candidate.range.start_sample);
                let overlap_end = tracked
                    .word
                    .range
                    .end_sample
                    .min(candidate.range.end_sample);
                return Some(AlignmentCandidate {
                    sequence: hypothesis.sequence,
                    range: candidate.range,
                    start_delta_samples: tracked
                        .word
                        .range
                        .start_sample
                        .abs_diff(candidate.range.start_sample),
                    end_delta_samples: tracked
                        .word
                        .range
                        .end_sample
                        .abs_diff(candidate.range.end_sample),
                    overlap_samples: overlap_end.saturating_sub(overlap_start),
                    within_time_tolerance: tracked
                        .word
                        .range
                        .start_sample
                        .abs_diff(candidate.range.start_sample)
                        <= TIME_TOLERANCE_SAMPLES
                        && tracked
                            .word
                            .range
                            .end_sample
                            .abs_diff(candidate.range.end_sample)
                            <= TIME_TOLERANCE_SAMPLES,
                    same_span: same_span(&tracked.word, candidate),
                });
            }
        }
        None
    }

    fn block_from_words(&mut self, words: Vec<TimedWord>) -> ConfirmedBlock {
        let first = words.first().expect("nonempty");
        let last = words.last().expect("nonempty");
        let block = ConfirmedBlock {
            segment_id: self.next_segment_id.saturating_add(1),
            text: words
                .iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            range: SourceRange {
                start_sample: first.range.start_sample,
                end_sample: last.range.end_sample,
                sample_rate_hz: SAMPLE_RATE,
            },
        };
        self.next_segment_id = block.segment_id;
        self.confirmed.push(block.clone());
        self.confirmed.sort_by_key(|item| {
            (
                item.range.start_sample,
                item.range.end_sample,
                item.segment_id,
            )
        });
        self.confirmed_end = self
            .confirmed
            .iter()
            .map(|item| item.range.end_sample)
            .max()
            .unwrap_or(0);
        block
    }
}

fn normalized_word(text: &str) -> String {
    text.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn same_word(a: &TimedWord, b: &TimedWord) -> bool {
    let left = normalized_word(&a.text);
    !left.is_empty() && left == normalized_word(&b.text) && same_span(a, b)
}

fn same_span(a: &TimedWord, b: &TimedWord) -> bool {
    let overlap_start = a.range.start_sample.max(b.range.start_sample);
    let overlap_end = a.range.end_sample.min(b.range.end_sample);
    let overlap = overlap_end.saturating_sub(overlap_start);
    a.range.start_sample.abs_diff(b.range.start_sample) <= TIME_TOLERANCE_SAMPLES
        && a.range.end_sample.abs_diff(b.range.end_sample) <= TIME_TOLERANCE_SAMPLES
        && overlap > 0
        && overlap.saturating_mul(2) >= a.range.end_sample - a.range.start_sample
        && overlap.saturating_mul(2) >= b.range.end_sample - b.range.start_sample
}

fn revision_has_context(
    old: &TrackedWord,
    replacement: &TrackedWord,
    evidence: &[Hypothesis],
) -> bool {
    if old.first_window == replacement.first_window || !same_span(&old.word, &replacement.word) {
        return false;
    }
    let overlap = old
        .word
        .range
        .end_sample
        .min(replacement.word.range.end_sample)
        .saturating_sub(
            old.word
                .range
                .start_sample
                .max(replacement.word.range.start_sample),
        );
    if overlap.saturating_mul(5)
        < (old.word.range.end_sample - old.word.range.start_sample).saturating_mul(4)
        || overlap.saturating_mul(5)
            < (replacement.word.range.end_sample - replacement.word.range.start_sample)
                .saturating_mul(4)
    {
        return false;
    }
    let Some(old_hypothesis) = evidence
        .iter()
        .find(|hypothesis| hypothesis.sequence == old.first_sequence)
    else {
        return false;
    };
    let Some(replacement_hypothesis) = evidence
        .iter()
        .find(|hypothesis| hypothesis.sequence == replacement.first_sequence)
    else {
        return false;
    };
    old_hypothesis.words.iter().any(|anchor| {
        !ranges_overlap(anchor.range, old.word.range)
            && !ranges_overlap(anchor.range, replacement.word.range)
            && replacement_hypothesis.words.iter().any(|other| {
                !ranges_overlap(other.range, old.word.range)
                    && !ranges_overlap(other.range, replacement.word.range)
                    && same_word(anchor, other)
            })
    })
}

fn gap_fully_covered(gap: SourceRange, evidence: &[Hypothesis], pending: &[TrackedWord]) -> bool {
    if pending.iter().any(|tracked| {
        ranges_overlap(tracked.word.range, gap) && (!tracked.witnessed_twice || tracked.conflict)
    }) {
        return false;
    }
    // Only matching, valid native word spans attest the part of the gap they
    // actually cover. Agreement on an empty interval is not evidence of silence.
    let mut certified = Vec::new();
    for (index, left) in evidence.iter().enumerate() {
        for right in evidence.iter().skip(index + 1) {
            if left.range == right.range {
                continue;
            }
            let start = gap
                .start_sample
                .max(left.range.start_sample)
                .max(right.range.start_sample);
            let end = gap
                .end_sample
                .min(left.range.end_sample)
                .min(right.range.end_sample);
            if start < end && concordant_on(left, right, start, end) {
                for (left_word, right_word) in words_overlapping(left, start, end)
                    .into_iter()
                    .zip(words_overlapping(right, start, end))
                {
                    let word_start = start
                        .max(left_word.range.start_sample)
                        .max(right_word.range.start_sample);
                    let word_end = end
                        .min(left_word.range.end_sample)
                        .min(right_word.range.end_sample);
                    if word_start < word_end {
                        certified.push((word_start, word_end));
                    }
                }
            }
        }
    }
    certified.sort_unstable();
    let mut covered_end = gap.start_sample;
    for (start, end) in certified {
        if start > covered_end {
            return false;
        }
        covered_end = covered_end.max(end);
        if covered_end >= gap.end_sample {
            return true;
        }
    }
    false
}

fn concordant_on(left: &Hypothesis, right: &Hypothesis, start: u64, end: u64) -> bool {
    let left_words = words_overlapping(left, start, end);
    let right_words = words_overlapping(right, start, end);
    left_words.len() == right_words.len()
        && left_words
            .iter()
            .zip(right_words)
            .all(|(left, right)| same_word(left, right))
}

fn words_overlapping(hypothesis: &Hypothesis, start: u64, end: u64) -> Vec<&TimedWord> {
    hypothesis
        .words
        .iter()
        .filter(|word| word.range.start_sample < end && word.range.end_sample > start)
        .collect()
}

fn ranges_overlap(left: SourceRange, right: SourceRange) -> bool {
    left.start_sample < right.end_sample && left.end_sample > right.start_sample
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(start: u64, end: u64) -> SourceRange {
        SourceRange::new(start, end, SAMPLE_RATE).unwrap()
    }

    fn word(text: &str, start: u64, end: u64) -> TimedWord {
        TimedWord {
            text: text.into(),
            range: range(start, end),
        }
    }

    #[test]
    fn stable_prefix_progresses_and_is_immutable() {
        let mut state = Confirmation::default();
        assert!(
            state
                .ingest(Hypothesis {
                    sequence: 1,
                    range: range(0, 32_000),
                    words: vec![word("bonjour", 2_000, 8_000)]
                })
                .unwrap()
                .is_empty()
        );
        let first = state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 64_000),
                words: vec![word("Bonjour,", 2_000, 8_000), word("toi", 20_000, 25_000)],
            })
            .unwrap();
        assert_eq!(first[0].text, "Bonjour,");
        assert_eq!(state.confirmed_end(), 8_000);
        state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(32_000, 96_000),
                words: vec![word("autre", 35_000, 40_000)],
            })
            .unwrap();
        assert_eq!(state.confirmed()[0], first[0]);
    }

    #[test]
    fn invalid_time_cannot_enter_ledger() {
        let mut state = Confirmation::default();
        assert!(
            state
                .ingest(Hypothesis {
                    sequence: 1,
                    range: range(0, 32_000),
                    words: vec![word("faux", 32_001, 33_000)]
                })
                .is_err()
        );
        assert!(state.confirmed().is_empty());
    }

    #[test]
    fn unmatched_word_blocks_later_confirmation() {
        let mut state = Confirmation::default();
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: vec![
                    word("avant", 1_000, 4_000),
                    word("incertain", 6_000, 10_000),
                ],
            })
            .unwrap();
        state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 64_000),
                words: vec![
                    word("avant", 1_000, 4_000),
                    word("autre", 6_000, 10_000),
                    word("après", 35_000, 40_000),
                ],
            })
            .unwrap();
        assert_eq!(state.confirmed()[0].text, "avant");
        assert!(
            state
                .pending
                .iter()
                .any(|tracked| tracked.word.text == "incertain")
        );
        assert_eq!(state.confirmed_end(), 4_000);
    }

    #[test]
    fn provisional_revision_needs_two_later_witnesses() {
        let mut state = Confirmation::default();
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: vec![word("faux", 2_000, 8_000), word("contexte", 10_000, 14_000)],
            })
            .unwrap();
        let context = state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 64_000),
                words: vec![
                    word("juste", 2_500, 8_500),
                    word("contexte", 10_000, 14_000),
                ],
            })
            .unwrap();
        assert_eq!(context.len(), 1);
        assert_eq!(context[0].text, "contexte");
        assert_eq!(context[0].range, range(10_000, 14_000));
        let immutable_context = context[0].clone();
        assert_eq!(state.pending.len(), 2);
        assert_eq!(state.pending[0].word.text, "faux");
        assert_eq!(state.pending[0].word.range, range(2_000, 8_000));
        assert!(!state.pending[0].witnessed_twice);
        assert_eq!(state.pending[1].word.text, "juste");
        assert_eq!(state.pending[1].word.range, range(2_500, 8_500));
        assert!(!state.pending[1].witnessed_twice);
        assert_eq!(state.confirmed(), &[immutable_context.clone()]);
        assert_eq!(
            state
                .pending
                .iter()
                .map(|tracked| tracked.word.text.as_str())
                .collect::<Vec<_>>(),
            ["faux", "juste"]
        );
        let blocks = state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(0, 96_000),
                words: vec![
                    word("juste", 2_500, 8_500),
                    word("contexte", 10_000, 14_000),
                ],
            })
            .unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "juste");
        assert_eq!(blocks[0].range, range(2_500, 8_500));
        assert!(state.pending.is_empty());
        assert_eq!(state.confirmed().len(), 2);
        assert_eq!(state.confirmed()[0].text, "juste");
        assert_eq!(state.confirmed()[1], immutable_context);
        assert_eq!(
            state
                .confirmed()
                .iter()
                .map(|block| block.text.as_str())
                .collect::<Vec<_>>(),
            ["juste", "contexte"]
        );
        assert!(!state.confirmed().iter().any(|block| block.text == "faux"));
        for pair in state.confirmed().windows(2) {
            assert!(pair[0].range.start_sample <= pair[1].range.start_sample);
            assert!(pair[0].range.end_sample <= pair[1].range.start_sample);
        }
        assert_eq!(
            state
                .confirmed()
                .iter()
                .filter(|block| block.range == immutable_context.range)
                .count(),
            1
        );
    }

    #[test]
    fn adjacent_words_inside_time_tolerance_are_not_the_same_acoustic_word() {
        let mut state = Confirmation::default();
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: vec![word("le", 2_000, 4_000)],
            })
            .unwrap();
        state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 64_000),
                words: vec![word("chat", 4_000, 6_000)],
            })
            .unwrap();
        let blocks = state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(0, 96_000),
                words: vec![word("chat", 4_000, 6_000)],
            })
            .unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "chat");
        assert_eq!(blocks[0].range, range(4_000, 6_000));
        assert_eq!(state.pending.len(), 1);
        assert_eq!(state.pending[0].word.text, "le");
        assert!(!state.pending[0].witnessed_twice);
        assert_eq!(state.confirmed(), blocks.as_slice());
        assert!(state.confirmed()[0].range.start_sample >= state.pending[0].word.range.end_sample);
    }

    #[test]
    fn adjacent_repeated_word_requires_its_own_temporal_witness() {
        let mut state = Confirmation::default();
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: vec![word("oui", 2_000, 4_000), word("oui", 4_000, 6_000)],
            })
            .unwrap();
        let blocks = state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 64_000),
                words: vec![word("oui", 4_000, 6_000)],
            })
            .unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "oui");
        assert_eq!(blocks[0].range, range(4_000, 6_000));
        assert_eq!(state.pending.len(), 1);
        assert_eq!(state.pending[0].word.text, "oui");
        assert_eq!(state.pending[0].word.range, range(2_000, 4_000));
        assert!(!state.pending[0].witnessed_twice);
        assert_eq!(state.confirmed(), blocks.as_slice());
        assert!(state.confirmed()[0].range.start_sample >= state.pending[0].word.range.end_sample);
    }

    #[test]
    fn ambiguous_overlap_does_not_supersede_an_earlier_word() {
        let mut state = Confirmation::default();
        for (sequence, text, start, end, window_end) in [
            (1, "ancien", 2_000, 4_000, 32_000),
            (2, "nouveau", 3_500, 5_500, 64_000),
            (3, "nouveau", 3_500, 5_500, 96_000),
        ] {
            state
                .ingest(Hypothesis {
                    sequence,
                    range: range(0, window_end),
                    words: vec![word(text, start, end)],
                })
                .unwrap();
        }
        assert_eq!(state.pending.len(), 2);
        assert_eq!(state.pending[0].word.text, "ancien");
        assert!(state.confirmed().is_empty());
    }

    #[test]
    fn shifted_distinct_word_without_shared_context_cannot_erase_predecessor() {
        let mut state = Confirmation::default();
        for (sequence, text, start, end, window_end) in [
            (1, "le", 2_000, 4_000, 32_000),
            (2, "chat", 3_000, 5_000, 64_000),
            (3, "chat", 3_000, 5_000, 96_000),
        ] {
            state
                .ingest(Hypothesis {
                    sequence,
                    range: range(0, window_end),
                    words: vec![word(text, start, end)],
                })
                .unwrap();
        }
        assert_eq!(state.pending.len(), 2);
        assert_eq!(state.pending[0].word.text, "le");
        assert!(state.confirmed().is_empty());
    }

    #[test]
    fn complete_concordant_coverage_resolves_a_full_window_gap() {
        let mut state = Confirmation::default();
        state.mark_unresolved(range(0, 32_000));
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 64_000),
                words: vec![word("retrouvé", 0, 32_000)],
            })
            .unwrap();
        assert_eq!(state.unresolved(), &[range(0, 32_000)]);
        let blocks = state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 96_000),
                words: vec![word("retrouvé", 0, 32_000)],
            })
            .unwrap();
        assert!(state.unresolved().is_empty());
        assert_eq!(blocks[0].text, "retrouvé");
    }

    #[test]
    fn partial_coverage_keeps_gap_until_concordant_pieces_join() {
        let mut state = Confirmation::default();
        state.mark_unresolved(range(0, 64_000));
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: vec![word("début", 0, 32_000)],
            })
            .unwrap();
        assert!(
            state
                .ingest(Hypothesis {
                    sequence: 2,
                    range: range(0, 64_000),
                    words: vec![word("début", 0, 32_000), word("suite", 32_000, 64_000)],
                })
                .unwrap()
                .is_empty()
        );
        assert_eq!(state.unresolved(), &[range(0, 64_000)]);
        let blocks = state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(32_000, 96_000),
                words: vec![word("suite", 32_000, 64_000)],
            })
            .unwrap();
        assert!(state.unresolved().is_empty());
        assert_eq!(blocks[0].text, "début suite");
    }

    #[test]
    fn empty_first_piece_cannot_borrow_a_word_from_the_second_piece() {
        let mut state = Confirmation::default();
        state.mark_unresolved(range(0, 64_000));
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: Vec::new(),
            })
            .unwrap();
        state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 64_000),
                words: vec![word("fin", 40_000, 44_000)],
            })
            .unwrap();
        let blocks = state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(32_000, 80_000),
                words: vec![word("fin", 40_000, 44_000)],
            })
            .unwrap();
        assert!(blocks.is_empty());
        assert_eq!(state.unresolved(), &[range(0, 64_000)]);
        assert_eq!(state.pending[0].word.text, "fin");
    }

    #[test]
    fn a_word_in_only_part_of_the_gap_cannot_release_later_words() {
        let mut state = Confirmation::default();
        state.mark_unresolved(range(0, 32_000));
        let mut published = Vec::new();
        for (sequence, end) in [(1, 48_000), (2, 64_000)] {
            published.extend(
                state
                    .ingest(Hypothesis {
                        sequence,
                        range: range(0, end),
                        words: vec![word("début", 0, 16_000), word("après", 36_000, 40_000)],
                    })
                    .unwrap(),
            );
        }
        assert_eq!(state.unresolved(), &[range(0, 32_000)]);
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].text, "après");
        assert_eq!(published[0].range, range(36_000, 40_000));
        assert_eq!(state.confirmed(), published.as_slice());
        assert_eq!(state.pending.len(), 1);
        assert_eq!(state.pending[0].word.text, "début");
        assert!(state.unresolved()[0].end_sample <= state.confirmed()[0].range.start_sample);
    }

    #[test]
    fn an_ambiguous_interval_between_agreed_words_stays_unresolved() {
        let mut state = Confirmation::default();
        state.mark_unresolved(range(0, 48_000));
        for (sequence, end) in [(1, 64_000), (2, 80_000)] {
            state
                .ingest(Hypothesis {
                    sequence,
                    range: range(0, end),
                    words: vec![word("avant", 0, 16_000), word("après", 32_000, 48_000)],
                })
                .unwrap();
        }
        assert_eq!(state.unresolved(), &[range(0, 48_000)]);
        assert!(state.confirmed().is_empty());
    }

    #[test]
    fn invalid_native_time_never_supplies_gap_evidence() {
        let mut state = Confirmation::default();
        state.mark_unresolved(range(0, 16_000));
        assert!(
            state
                .ingest(Hypothesis {
                    sequence: 1,
                    range: range(0, 32_000),
                    words: vec![TimedWord {
                        text: "invalide".into(),
                        range: SourceRange {
                            start_sample: 0,
                            end_sample: 0,
                            sample_rate_hz: SAMPLE_RATE,
                        },
                    }],
                })
                .is_err()
        );
        assert_eq!(state.unresolved(), &[range(0, 16_000)]);
        assert!(state.confirmed().is_empty());
    }

    #[test]
    fn unresolved_gap_preserves_prefix_and_allows_independent_later_confirmation() {
        let mut state = Confirmation::default();
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: vec![word("avant", 0, 8_000)],
            })
            .unwrap();
        let prefix = state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 64_000),
                words: vec![word("avant", 0, 8_000)],
            })
            .unwrap();
        assert_eq!(prefix[0].text, "avant");
        state.mark_unresolved(range(16_000, 32_000));
        state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(32_000, 96_000),
                words: vec![word("après", 36_000, 40_000)],
            })
            .unwrap();
        let later = state
            .ingest(Hypothesis {
                sequence: 4,
                range: range(32_000, 112_000),
                words: vec![word("après", 36_000, 40_000)],
            })
            .unwrap();
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].text, "après");
        assert_eq!(state.confirmed()[0], prefix[0]);
        assert_eq!(state.confirmed()[1].text, "après");
        assert_eq!(state.unresolved(), &[range(16_000, 32_000)]);
        assert!(state.pending.is_empty());
    }

    #[test]
    fn later_agreed_words_publish_independently_across_an_uncovered_prefix() {
        let mut state = Confirmation::default();
        state.mark_unresolved(range(0, 16_000));
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: vec![word("après", 20_000, 24_000)],
            })
            .unwrap();
        let blocks = state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(16_000, 64_000),
                words: vec![word("après", 20_000, 24_000)],
            })
            .unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "après");
        assert_eq!(state.unresolved(), &[range(0, 16_000)]);
        assert!(state.pending.is_empty());
    }

    #[test]
    fn two_empty_hypotheses_cannot_resolve_a_gap_before_later_text() {
        let mut state = Confirmation::default();
        state.mark_unresolved(range(0, 16_000));
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: Vec::new(),
            })
            .unwrap();
        state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 64_000),
                words: Vec::new(),
            })
            .unwrap();
        assert_eq!(state.unresolved(), &[range(0, 16_000)]);
        state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(16_000, 80_000),
                words: vec![word("après", 20_000, 24_000)],
            })
            .unwrap();
        assert_eq!(state.unresolved(), &[range(0, 16_000)]);
        assert!(state.confirmed().is_empty());
    }

    #[test]
    fn repeated_word_requires_two_distinct_temporal_matches() {
        let mut state = Confirmation::default();
        let words = vec![word("oui", 2_000, 5_000), word("oui", 12_000, 15_000)];
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: words.clone(),
            })
            .unwrap();
        let blocks = state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 64_000),
                words,
            })
            .unwrap();
        assert_eq!(blocks[0].text, "oui oui");
        assert_eq!(state.confirmed().len(), 1);
    }

    #[test]
    fn expired_gap_does_not_block_a_stable_later_word() {
        let mut state = Confirmation::default();
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: vec![word("perdu", 2_000, 6_000)],
            })
            .unwrap();
        state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 64_000),
                words: vec![word("après", 36_000, 42_000)],
            })
            .unwrap();
        let blocks = state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(32_000, 96_000),
                words: vec![word("après", 36_000, 42_000)],
            })
            .unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "après");
        assert_eq!(state.unresolved(), &[range(2_000, 6_000)]);
    }

    #[test]
    fn confirmation_progresses_when_old_anchor_leaves_the_window() {
        let mut state = Confirmation::default();
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: vec![word("premier", 2_000, 8_000)],
            })
            .unwrap();
        assert_eq!(
            state
                .ingest(Hypothesis {
                    sequence: 2,
                    range: range(0, 64_000),
                    words: vec![
                        word("premier", 2_000, 8_000),
                        word("suivant", 36_000, 42_000)
                    ]
                })
                .unwrap()[0]
                .text,
            "premier"
        );
        let next = state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(32_000, 96_000),
                words: vec![word("suivant", 36_000, 42_000)],
            })
            .unwrap();
        assert_eq!(next[0].text, "suivant");
    }

    #[test]
    fn finalization_never_promotes_a_single_hypothesis() {
        let mut state = Confirmation::default();
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: vec![word("seul", 2_000, 8_000)],
            })
            .unwrap();
        assert!(state.finalize().is_empty());
        assert!(state.has_remainder(32_000));
    }

    #[test]
    fn identical_audio_context_is_not_a_second_proof() {
        let mut state = Confirmation::default();
        let first = Hypothesis {
            sequence: 1,
            range: range(0, 32_000),
            words: vec![word("bonjour", 2_000, 8_000)],
        };
        state.ingest(first.clone()).unwrap();
        assert!(
            state
                .ingest(Hypothesis {
                    sequence: 2,
                    ..first
                })
                .unwrap()
                .is_empty()
        );
        assert!(state.finalize().is_empty());
    }

    #[test]
    fn pending_word_is_marked_for_recovery_before_the_next_window_passes_it() {
        let mut state = Confirmation::default();
        let ah = word("Ah", 116_480, 119_040);
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 96_000),
                words: Vec::new(),
            })
            .unwrap();
        state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(96_000, 160_000),
                words: vec![ah.clone()],
            })
            .unwrap();

        assert!(state.recovery_candidates(120_000).is_empty());
        assert_eq!(state.recovery_candidates(128_000), vec![ah.range]);
    }

    #[test]
    fn contextual_recovery_witness_confirms_without_duplication_and_keeps_prefix() {
        let mut state = Confirmation::default();
        let prefix = state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 96_000),
                words: vec![word("avant", 40_000, 48_000)],
            })
            .unwrap();
        assert!(prefix.is_empty());
        let prefix = state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(0, 128_000),
                words: vec![word("avant", 40_000, 48_000)],
            })
            .unwrap();
        assert_eq!(prefix[0].text, "avant");
        let confirmed_prefix = state.confirmed().to_vec();

        let ah = word("Ah", 116_480, 119_040);
        state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(96_000, 160_000),
                words: vec![ah.clone()],
            })
            .unwrap();
        assert!(state.recovery_candidates(128_000).contains(&ah.range));

        let recovered = state
            .ingest(Hypothesis {
                sequence: 4,
                range: range(85_760, 149_760),
                words: vec![word("Ah", 116_480, 119_040)],
            })
            .unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].text, "Ah");
        let confirmed = state.confirmed().to_vec();
        assert_eq!(&confirmed[..1], confirmed_prefix.as_slice());

        let later = state
            .ingest(Hypothesis {
                sequence: 5,
                range: range(160_000, 224_000),
                words: Vec::new(),
            })
            .unwrap();
        assert!(later.is_empty());
        assert_eq!(confirmed[1], state.confirmed()[1]);
        assert_eq!(&state.confirmed()[..1], confirmed_prefix.as_slice());
        assert_eq!(state.confirmed().len(), 2);
        assert_eq!(state.confirmed()[1].text, "Ah");
    }

    #[test]
    fn failed_contextual_recovery_keeps_word_and_gap_unresolved() {
        let mut state = Confirmation::default();
        let ah = word("Ah", 116_480, 119_040);
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 96_000),
                words: Vec::new(),
            })
            .unwrap();
        state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(96_000, 160_000),
                words: vec![ah.clone()],
            })
            .unwrap();
        state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(85_760, 149_760),
                words: Vec::new(),
            })
            .unwrap();
        assert!(
            state
                .ingest(Hypothesis {
                    sequence: 4,
                    range: range(128_000, 192_000),
                    words: Vec::new(),
                })
                .unwrap()
                .is_empty()
        );
        assert!(state.confirmed().is_empty());
        assert_eq!(state.pending.len(), 1);
        assert_eq!(state.pending[0].word, ah);
        assert!(!state.pending[0].witnessed_twice);
        assert!(state.unresolved().contains(&ah.range));
    }

    #[test]
    fn partial_contextual_recovery_does_not_resolve_the_rest_of_a_gap() {
        let mut state = Confirmation::default();
        let gap = range(121_600, 124_160);
        state.mark_unresolved(gap);
        for sequence in [1, 2] {
            state
                .ingest(Hypothesis {
                    sequence,
                    range: if sequence == 1 {
                        range(80_000, 144_000)
                    } else {
                        range(88_000, 152_000)
                    },
                    words: vec![word("ça", 122_000, 123_000)],
                })
                .unwrap();
        }
        assert_eq!(state.unresolved(), &[gap]);
        assert!(state.confirmed().is_empty());
    }

    #[test]
    fn later_segment_publishes_across_an_unresolved_prefix_and_ids_survive_recovery() {
        let mut state = Confirmation::default();
        let earlier = word("test", 2_000, 6_000);
        let later = word("fonctionne", 36_000, 42_000);
        state
            .ingest(Hypothesis {
                sequence: 1,
                range: range(0, 32_000),
                words: vec![earlier.clone()],
            })
            .unwrap();
        state
            .ingest(Hypothesis {
                sequence: 2,
                range: range(32_000, 64_000),
                words: vec![later.clone()],
            })
            .unwrap();
        let later_block = state
            .ingest(Hypothesis {
                sequence: 3,
                range: range(24_000, 64_000),
                words: vec![later],
            })
            .unwrap();
        assert_eq!(later_block.len(), 1);
        assert_eq!(later_block[0].segment_id, 1);
        assert_eq!(state.unresolved(), &[earlier.range]);

        let recovered = state
            .ingest(Hypothesis {
                sequence: 4,
                range: range(0, 30_000),
                words: vec![earlier],
            })
            .unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].segment_id, 2);
        assert_eq!(state.confirmed()[0].text, "test");
        assert_eq!(state.confirmed()[0].segment_id, 2);
        assert_eq!(state.confirmed()[1].text, "fonctionne");
        assert_eq!(state.confirmed()[1].segment_id, 1);
    }
}
