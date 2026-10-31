use super::*;

pub(in crate::ui) fn capture_state_label(
    state: whisper_core::ports::ParakeetCaptureViewState,
) -> &'static str {
    match state {
        whisper_core::ports::ParakeetCaptureViewState::Starting => "Démarrage du microphone",
        whisper_core::ports::ParakeetCaptureViewState::Capturing => "Capture en cours",
        whisper_core::ports::ParakeetCaptureViewState::Finalizing => "Finalisation du WAV",
        whisper_core::ports::ParakeetCaptureViewState::Finalized => "Capture terminée",
        whisper_core::ports::ParakeetCaptureViewState::Failed => "Capture interrompue",
    }
}

pub(in crate::ui) fn unified_session_phase_label(
    state: whisper_core::ports::UnifiedSessionPhase,
) -> &'static str {
    match state {
        whisper_core::ports::UnifiedSessionPhase::Starting => "Démarrage",
        whisper_core::ports::UnifiedSessionPhase::Capturing => "Capture ou transcription en cours",
        whisper_core::ports::UnifiedSessionPhase::Finalizing => "Finalisation",
        whisper_core::ports::UnifiedSessionPhase::Publishing => "Publication",
        whisper_core::ports::UnifiedSessionPhase::Complete => "Terminé",
        whisper_core::ports::UnifiedSessionPhase::Recoverable => "Récupérable",
    }
}

pub(in crate::ui) fn compute_choice_label(choice: ComputeChoice) -> &'static str {
    match choice {
        ComputeChoice::Cpu => "CPU",
        ComputeChoice::Gpu => "GPU",
        ComputeChoice::Auto => "Auto",
    }
}

pub(in crate::ui) fn provisional_without_confirmed_prefix(
    capture: &whisper_core::ports::ParakeetCaptureView,
    provisional: &str,
) -> Option<String> {
    let range = capture.provisional_range?;
    let mut overlaps = capture
        .confirmed_segments
        .iter()
        .filter(|segment| {
            segment.range.start_sample < range.end_sample
                && range.start_sample < segment.range.end_sample
        })
        .collect::<Vec<_>>();
    if overlaps.is_empty() {
        return Some(provisional.to_owned());
    }
    overlaps.sort_by_key(|segment| (segment.range.start_sample, segment.segment_id.0));
    let confirmed_words = overlaps
        .iter()
        .flat_map(|segment| segment.text.split_whitespace())
        .map(normalized_word)
        .collect::<Vec<_>>();
    let provisional_words = provisional.split_whitespace().collect::<Vec<_>>();
    if confirmed_words.is_empty() || provisional_words.len() < confirmed_words.len() {
        return None;
    }
    let prefix_matches = provisional_words
        .iter()
        .take(confirmed_words.len())
        .map(|word| normalized_word(word))
        .eq(confirmed_words.iter().cloned());
    if !prefix_matches {
        return None;
    }
    let suffix = provisional_words[confirmed_words.len()..].join(" ");
    (!suffix.trim().is_empty()).then_some(suffix)
}

pub(in crate::ui) fn normalized_word(word: &str) -> String {
    word.chars()
        .filter(|character| character.is_alphanumeric() || *character == '\'')
        .flat_map(char::to_lowercase)
        .collect()
}
