use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub const MAX_LIVE_WINDOW_SAMPLES: usize = 128_000;

pub struct GpuEngine {
    context: WhisperContext,
}

pub struct TextSegment {
    pub start_centiseconds: i64,
    pub end_centiseconds: i64,
    pub text: String,
}

impl GpuEngine {
    pub fn attest(&self) -> Result<(), String> {
        let started = std::time::Instant::now();
        let result = self
            .context
            .create_state()
            .map(|_| ())
            .map_err(|error| format!("CUDA state initialization failed: {error}"));
        eprintln!(
            "startup_perf phase=gpu_state_attest elapsed_ms={}",
            started.elapsed().as_millis()
        );
        result
    }

    pub fn load(path: &str) -> Result<Self, String> {
        let started = std::time::Instant::now();
        let mut params = WhisperContextParameters::default();
        params.use_gpu(true);
        let context = WhisperContext::new_with_params(path, params)
            .map_err(|e| format!("CUDA model initialization failed: {e}"))?;
        eprintln!(
            "startup_perf phase=gpu_context_load elapsed_ms={}",
            started.elapsed().as_millis()
        );
        let state_started = std::time::Instant::now();
        context
            .create_state()
            .map_err(|e| format!("CUDA state initialization failed: {e}"))?;
        eprintln!(
            "startup_perf phase=gpu_initial_state elapsed_ms={}",
            state_started.elapsed().as_millis()
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
        let mut state = self
            .context
            .create_state()
            .map_err(|e| format!("CUDA state creation failed: {e}"))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(language);
        params.set_translate(false);
        params.set_no_context(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        state
            .full(params, pcm)
            .map_err(|e| format!("CUDA inference failed: {e}"))?;
        Ok(state
            .as_iter()
            .map(|segment| TextSegment {
                start_centiseconds: segment.start_timestamp(),
                end_centiseconds: segment.end_timestamp(),
                text: segment.to_string(),
            })
            .collect())
    }
}
