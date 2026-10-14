use std::time::Instant;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub const MAX_LIVE_WINDOW_SAMPLES: usize = 128_000;

pub struct CpuEngine {
    context: WhisperContext,
}

pub struct TextSegment {
    pub start_centiseconds: i64,
    pub end_centiseconds: i64,
    pub text: String,
}

impl CpuEngine {
    pub fn load(path: &str) -> Result<Self, String> {
        let load_started = Instant::now();
        #[cfg(feature = "l01-memory-qualification")]
        let started = std::time::Instant::now();
        #[cfg(feature = "l01-memory-qualification")]
        crate::memory_qualification::record(
            "native.context.create.begin",
            serde_json::json!({
                "model_path_bytes": std::fs::metadata(path).map(|metadata| metadata.len()).ok(),
                "native_allocation_bytes": "opaque",
            }),
        );
        let mut params = WhisperContextParameters::default();
        params.use_gpu(false);
        let context = WhisperContext::new_with_params(path, params)
            .map_err(|e| format!("CPU model initialization failed: {e}"))?;
        // Creating state here is the CPU backend attestation point; inference cannot fall back.
        context
            .create_state()
            .map_err(|e| format!("CPU state initialization failed: {e}"))?;
        if std::env::var_os("WHISPER_CPU_TIMINGS").is_some() {
            eprintln!(
                "cpu_timing model_load_ms={:.1}",
                load_started.elapsed().as_secs_f64() * 1000.0
            );
        }
        #[cfg(feature = "l01-memory-qualification")]
        crate::memory_qualification::record(
            "native.context.create.complete",
            serde_json::json!({
                "elapsed_us": started.elapsed().as_micros(),
                "model_path_bytes": std::fs::metadata(path).map(|metadata| metadata.len()).ok(),
                "native_context_allocation_bytes": "opaque; see process memory samples",
                "initial_state_retained": false,
            }),
        );
        Ok(Self { context })
    }

    pub fn transcribe(
        &self,
        pcm: &[f32],
        language: Option<&str>,
    ) -> Result<Vec<TextSegment>, String> {
        if pcm.is_empty() || pcm.len() > MAX_LIVE_WINDOW_SAMPLES {
            return Err("invalid inference window size".into());
        }
        if let Some(language) = language
            && (language.contains('\0') || whisper_rs::get_lang_id(language).is_none())
        {
            return Err("UnsupportedLanguage: select a supported Whisper language code".into());
        }
        // Whisper can hallucinate on silence or a short quiet tail. Keep the
        // window's coverage, but return no text without invoking the model.
        if pcm.iter().all(|sample| *sample == 0.0) || whisper_core::audio::is_quiet_short_tail(pcm)
        {
            return Ok(Vec::new());
        }
        #[cfg(feature = "l01-memory-qualification")]
        crate::memory_qualification::record(
            "native.state.create.begin",
            serde_json::json!({
                "pcm_samples": pcm.len(),
                "native_state_allocation_bytes": "opaque; process counters sampled at stage",
            }),
        );
        let state_started = Instant::now();
        let mut state = self
            .context
            .create_state()
            .map_err(|e| format!("CPU state creation failed: {e}"))?;
        let state_ms = state_started.elapsed().as_secs_f64() * 1000.0;
        #[cfg(feature = "l01-memory-qualification")]
        let state_handle_bytes = std::mem::size_of_val(&state);
        #[cfg(feature = "l01-memory-qualification")]
        let started = std::time::Instant::now();
        #[cfg(feature = "l01-memory-qualification")]
        crate::memory_qualification::record(
            "native.state.create.complete",
            serde_json::json!({
                "pcm_samples": pcm.len(),
                "rust_state_handle_bytes": state_handle_bytes,
                "native_state_allocation_bytes": "opaque; process counters sampled at stage",
            }),
        );
        #[cfg(feature = "l01-memory-qualification")]
        crate::memory_qualification::record(
            "native.inference.begin",
            serde_json::json!({
                "pcm_samples": pcm.len(),
                "pcm_bytes": std::mem::size_of_val(pcm),
                "native_state_allocation_bytes": "opaque; process counters sampled at stage",
            }),
        );
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        // The validated Windows target (i9-14900K) has 24 physical cores.
        // Clamp for hosts with fewer logical processors.
        let threads = std::thread::available_parallelism()
            .map(|count| count.get().min(24) as i32)
            .unwrap_or(24);
        params.set_n_threads(threads);
        params.set_language(language);
        params.set_translate(false);
        params.set_no_context(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        let infer_started = Instant::now();
        state
            .full(params, pcm)
            .map_err(|e| format!("CPU inference failed: {e}"))?;
        let full_ms = infer_started.elapsed().as_secs_f64() * 1000.0;
        if std::env::var_os("WHISPER_CPU_TIMINGS").is_some() {
            // The wrapper exposes encoder calls but not the native state's internal counters.
            // Measure one separate encoder pass; the remaining full() time is an estimate.
            let diagnostic_started = Instant::now();
            let mut diagnostic = self
                .context
                .create_state()
                .map_err(|e| format!("CPU diagnostic state creation failed: {e}"))?;
            let diagnostic_state_ms = diagnostic_started.elapsed().as_secs_f64() * 1000.0;
            let mel_started = Instant::now();
            diagnostic
                .pcm_to_mel(pcm, threads as usize)
                .map_err(|e| format!("CPU diagnostic mel failed: {e}"))?;
            let mel_ms = mel_started.elapsed().as_secs_f64() * 1000.0;
            let encode_started = Instant::now();
            diagnostic
                .encode(0, threads as usize)
                .map_err(|e| format!("CPU diagnostic encode failed: {e}"))?;
            let encode_ms = encode_started.elapsed().as_secs_f64() * 1000.0;
            eprintln!(
                "cpu_timing pcm_samples={} threads={} state_ms={:.1} full_ms={:.1} diagnostic_state_ms={:.1} mel_ms={:.1} encode_ms={:.1} decode_and_overhead_estimate_ms={:.1}",
                pcm.len(),
                threads,
                state_ms,
                full_ms,
                diagnostic_state_ms,
                mel_ms,
                encode_ms,
                (full_ms - mel_ms - encode_ms).max(0.0)
            );
        }
        let segments = state
            .as_iter()
            .map(|segment| TextSegment {
                start_centiseconds: segment.start_timestamp(),
                end_centiseconds: segment.end_timestamp(),
                text: segment.to_string(),
            })
            .collect::<Vec<_>>();
        #[cfg(feature = "l01-memory-qualification")]
        crate::memory_qualification::record(
            "native.inference.complete",
            serde_json::json!({
                "elapsed_us": started.elapsed().as_micros(),
                "pcm_samples": pcm.len(),
                "segment_count": segments.len(),
                "segment_vec_capacity": segments.capacity(),
                "segment_object_bytes": segments.capacity() * std::mem::size_of::<TextSegment>(),
                "segment_text_capacity_bytes": segments.iter().map(|segment| segment.text.capacity()).sum::<usize>(),
                "rust_state_handle_bytes": state_handle_bytes,
                "native_state_allocation_bytes": "opaque; process counters sampled at stage",
                "state_dropped": false,
            }),
        );
        drop(state);
        #[cfg(feature = "l01-memory-qualification")]
        crate::memory_qualification::record(
            "native.state.destroyed",
            serde_json::json!({
                "pcm_samples": pcm.len(),
                "segment_count": segments.len(),
                "rust_state_handle_bytes_released": state_handle_bytes,
                "native_state_allocation_bytes": "opaque; process counters sampled after drop",
            }),
        );
        Ok(segments)
    }
}
