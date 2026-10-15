use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub const MAX_LIVE_WINDOW_SAMPLES: usize = 128_000;

pub struct GpuEngine {
    context: WhisperContext,
    attested_state: std::sync::Mutex<Option<whisper_rs::WhisperState>>,
    expected_backend: String,
    whisper_gpu_index: u32,
}

pub struct TextSegment {
    pub start_centiseconds: i64,
    pub end_centiseconds: i64,
    pub text: String,
}

impl GpuEngine {
    pub fn attest(&self) -> Result<(), String> {
        let started = std::time::Instant::now();
        crate::native_ffi::begin_native_gpu_phase()?;
        let state_result = self
            .context
            .create_state()
            .map_err(|error| format!("CUDA state initialization failed: {error}"));
        crate::native_ffi::finish_native_gpu_phase(&self.expected_backend)?;
        let state = state_result?;
        *self
            .attested_state
            .lock()
            .map_err(|_| "CUDA state lock is poisoned".to_owned())? = Some(state);
        eprintln!(
            "startup_perf phase=gpu_state_attest elapsed_ms={}",
            started.elapsed().as_millis()
        );
        Ok(())
    }

    pub fn load(
        path: &str,
        whisper_gpu_index: u32,
        expected_backend: String,
    ) -> Result<Self, String> {
        let started = std::time::Instant::now();
        let mut params = WhisperContextParameters::default();
        params.use_gpu(true);
        params.gpu_device(
            i32::try_from(whisper_gpu_index)
                .map_err(|_| "Whisper GPU index exceeds whisper-rs range".to_owned())?,
        );
        let context = WhisperContext::new_with_params(path, params)
            .map_err(|e| format!("CUDA model initialization failed: {e}"))?;
        eprintln!(
            "startup_perf phase=gpu_context_load elapsed_ms={}",
            started.elapsed().as_millis()
        );
        Ok(Self {
            context,
            attested_state: std::sync::Mutex::new(None),
            expected_backend,
            whisper_gpu_index,
        })
    }

    pub fn matches_device(&self, whisper_gpu_index: u32, expected_backend: &str) -> bool {
        self.whisper_gpu_index == whisper_gpu_index && self.expected_backend == expected_backend
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
            .attested_state
            .lock()
            .map_err(|_| "CUDA state lock is poisoned".to_owned())?
            .take()
            .ok_or_else(|| "CUDA state has not been attested".to_owned())?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(language);
        params.set_translate(false);
        params.set_no_context(true);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        let result = state
            .full(params, pcm)
            .map_err(|e| format!("CUDA inference failed: {e}"))
            .map(|()| {
                state
                    .as_iter()
                    .map(|segment| TextSegment {
                        start_centiseconds: segment.start_timestamp(),
                        end_centiseconds: segment.end_timestamp(),
                        text: segment.to_string(),
                    })
                    .collect::<Vec<_>>()
            });
        *self
            .attested_state
            .lock()
            .map_err(|_| "CUDA state lock is poisoned".to_owned())? = Some(state);
        result
    }
}
