use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct GpuEngine {
    context: WhisperContext,
}

pub struct TextSegment {
    pub start_centiseconds: i64,
    pub end_centiseconds: i64,
    pub text: String,
}

impl GpuEngine {
    pub fn load(path: &str) -> Result<Self, String> {
        let mut params = WhisperContextParameters::default();
        params.use_gpu(true);
        let context = WhisperContext::new_with_params(path, params)
            .map_err(|e| format!("CUDA model initialization failed: {e}"))?;
        context
            .create_state()
            .map_err(|e| format!("CUDA state initialization failed: {e}"))?;
        Ok(Self { context })
    }

    pub fn transcribe(
        &self,
        pcm: &[f32],
        language: Option<&str>,
    ) -> Result<Vec<TextSegment>, String> {
        if pcm.is_empty() || pcm.len() > 80_000 {
            return Err("invalid inference window size".into());
        }
        if let Some(language) = language
            && (language.contains('\0') || whisper_rs::get_lang_id(language).is_none())
        {
            return Err("UnsupportedLanguage: select a supported Whisper language code".into());
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
