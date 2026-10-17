// SAFETY/ABI CONTRACT (NeMo-Speech.cpp v0.2.0 `nemo_speech/asr.h`): the repr(C)
// layouts and function signatures below mirror that header. Every pointer passed
// into a call remains valid for the synchronous call; recognizer/result handles
// are owned by this wrapper and destroyed exactly once. Native result strings
// are copied into Rust-owned Strings before ResultGuard destroys their owner.
// This recognizer is confined to the worker's single inference thread; &mut self
// serializes calls. The DLL is intentionally kept mapped for process lifetime,
// so function pointers and native destructors remain callable until process exit.

use std::{
    ffi::{CStr, CString, c_char, c_void},
    mem::{ManuallyDrop, size_of},
    path::Path,
};

use libloading::os::windows::{
    LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, Library,
};

#[repr(C)]
struct BackendConfig {
    size: usize,
    gpu: i32,
}
#[repr(C)]
struct ModelConfig {
    size: usize,
    path: *const c_char,
    name: *const c_char,
}
#[repr(C)]
struct RecognizerConfig {
    size: usize,
    backend: *const BackendConfig,
    model: *const ModelConfig,
    streaming: *const c_void,
    decoder: *const c_void,
    vad: *const c_void,
    endpointing: *const c_void,
    postproc: *const c_void,
    diar: *const c_void,
    batching: *const c_void,
}
#[repr(C)]
struct RecognitionOptions {
    size: usize,
    request_id: *const c_char,
    language_code: *const c_char,
    interim_results: bool,
    enable_word_time_offsets: bool,
    enable_automatic_punctuation: bool,
    verbatim_transcripts: bool,
    profanity_filter: bool,
    stop_history_eou_ms: i32,
    speech_contexts: *const c_void,
    speech_context_count: usize,
    max_alternatives: i32,
    enable_speaker_diarization: bool,
    max_speaker_count: i32,
}

type Create = unsafe extern "C" fn(*const RecognizerConfig, *mut *mut c_void) -> i32;
type Destroy = unsafe extern "C" fn(*mut c_void);
type Recognize = unsafe extern "C" fn(
    *mut c_void,
    *const RecognitionOptions,
    *const f32,
    usize,
    i32,
    *mut *mut c_void,
) -> i32;
type LastError = unsafe extern "C" fn() -> *const c_char;
type Version = unsafe extern "C" fn() -> *const c_char;
type ResultWordCount = unsafe extern "C" fn(*const c_void, usize) -> usize;
type ResultWordText = unsafe extern "C" fn(*const c_void, usize, usize) -> *const c_char;
type ResultWordTime = unsafe extern "C" fn(*const c_void, usize, usize) -> i32;
type ResultDestroy = unsafe extern "C" fn(*mut c_void);

struct Api {
    _library: ManuallyDrop<Library>,
    create: Create,
    destroy: Destroy,
    recognize: Recognize,
    last_error: LastError,
    version: Version,
    result_word_count: ResultWordCount,
    result_word_text: ResultWordText,
    result_word_start: ResultWordTime,
    result_word_end: ResultWordTime,
    result_destroy: ResultDestroy,
}

pub struct NativeRecognizer {
    api: Api,
    recognizer: *mut c_void,
}

#[derive(Clone, Debug)]
pub struct NativeWord {
    pub text: String,
    pub start_ms: i32,
    pub end_ms: i32,
}

#[derive(Clone, Debug)]
pub struct NativeResult {
    pub words: Vec<NativeWord>,
}

impl NativeRecognizer {
    pub fn load(native_dir: &Path, model: &Path) -> Result<Self, String> {
        let dll = native_dir.join("nemo_speech_asr_c.dll");
        let dll = dll
            .canonicalize()
            .map_err(|error| format!("NativeLibrary: {error}"))?;
        // SAFETY: native_dir is a canonical directory of the authorized v0.2.0
        // CPU DLL set. The loaded Library is immediately wrapped in ManuallyDrop
        // below and therefore outlives every symbol/function pointer in Api.
        let library = unsafe {
            Library::load_with_flags(
                &dll,
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
            )
        }
        .map_err(|error| format!("NativeLibrary: {error}"))?;
        // ManuallyDrop keeps the DLL mapped until this worker process exits. NeMo's
        // CPU build crashes when FreeLibrary follows completed inference.
        let library = ManuallyDrop::new(library);
        // SAFETY: each symbol name and function signature is declared by the
        // matching v0.2.0 header. `library` remains mapped in Api for process life.
        let api = unsafe {
            Api {
                create: *library
                    .get(b"nemo_speech_asr_create\0")
                    .map_err(|e| e.to_string())?,
                destroy: *library
                    .get(b"nemo_speech_asr_destroy\0")
                    .map_err(|e| e.to_string())?,
                recognize: *library
                    .get(b"nemo_speech_asr_recognize_f32\0")
                    .map_err(|e| e.to_string())?,
                last_error: *library
                    .get(b"nemo_speech_asr_last_error\0")
                    .map_err(|e| e.to_string())?,
                version: *library
                    .get(b"nemo_speech_asr_version\0")
                    .map_err(|e| e.to_string())?,
                result_word_count: *library
                    .get(b"nemo_speech_asr_result_word_count\0")
                    .map_err(|e| e.to_string())?,
                result_word_text: *library
                    .get(b"nemo_speech_asr_result_word_text\0")
                    .map_err(|e| e.to_string())?,
                result_word_start: *library
                    .get(b"nemo_speech_asr_result_word_start_time\0")
                    .map_err(|e| e.to_string())?,
                result_word_end: *library
                    .get(b"nemo_speech_asr_result_word_end_time\0")
                    .map_err(|e| e.to_string())?,
                result_destroy: *library
                    .get(b"nemo_speech_asr_result_destroy\0")
                    .map_err(|e| e.to_string())?,
                _library: library,
            }
        };

        let model = model
            .canonicalize()
            .map_err(|error| format!("Model: {error}"))?;
        let model = CString::new(model.to_string_lossy().as_bytes())
            .map_err(|_| "Model: path contains an interior NUL".to_owned())?;
        let backend = BackendConfig {
            size: size_of::<BackendConfig>(),
            gpu: -1,
        };
        let model_config = ModelConfig {
            size: size_of::<ModelConfig>(),
            path: model.as_ptr(),
            name: std::ptr::null(),
        };
        let config = RecognizerConfig {
            size: size_of::<RecognizerConfig>(),
            backend: &backend,
            model: &model_config,
            streaming: std::ptr::null(),
            decoder: std::ptr::null(),
            vad: std::ptr::null(),
            endpointing: std::ptr::null(),
            postproc: std::ptr::null(),
            diar: std::ptr::null(),
            batching: std::ptr::null(),
        };
        let mut recognizer = std::ptr::null_mut();
        // SAFETY: config and all pointers it references point to stack/CString
        // values alive for this synchronous call; NeMo copies configuration it
        // needs and returns an owned recognizer through the out pointer.
        let status = unsafe { (api.create)(&config, &mut recognizer) };
        if status != 0 {
            return Err(native_error(&api, status));
        }
        if recognizer.is_null() {
            return Err("NativeCreate: returned a null recognizer".into());
        }
        Ok(Self { api, recognizer })
    }

    pub fn version(&self) -> Result<String, String> {
        // SAFETY: the API returns its version string as a NUL-terminated pointer
        // valid while the DLL remains loaded; copy_c_string takes an owned copy.
        let ptr = unsafe { (self.api.version)() };
        copy_c_string(ptr, "native version")
    }

    pub fn transcribe(&mut self, samples: &[f32]) -> Result<NativeResult, String> {
        let options = RecognitionOptions {
            size: size_of::<RecognitionOptions>(),
            request_id: std::ptr::null(),
            language_code: std::ptr::null(),
            interim_results: false,
            enable_word_time_offsets: true,
            enable_automatic_punctuation: true,
            verbatim_transcripts: false,
            profanity_filter: false,
            stop_history_eou_ms: 0,
            speech_contexts: std::ptr::null(),
            speech_context_count: 0,
            max_alternatives: 1,
            enable_speaker_diarization: false,
            max_speaker_count: 0,
        };
        let mut result = std::ptr::null_mut();
        // SAFETY: &mut self enforces single-threaded access; samples and options
        // remain alive for the synchronous call. On success, NeMo returns an
        // owned result handle, which is released by ResultGuard below.
        let status = unsafe {
            (self.api.recognize)(
                self.recognizer,
                &options,
                samples.as_ptr(),
                samples.len(),
                16_000,
                &mut result,
            )
        };
        if status != 0 {
            return Err(native_error(&self.api, status));
        }
        if result.is_null() {
            return Err("NativeRecognize: returned a null result".into());
        }
        let guard = ResultGuard {
            api: &self.api,
            result,
        };
        // SAFETY: the live result handle and alternative index 0 are valid for
        // the duration of this call; count is bounded before allocating.
        let count = unsafe { (self.api.result_word_count)(guard.result, 0) };
        if count > 1_000_000 {
            return Err("NativeResult: word count exceeds bound".into());
        }
        let mut words = Vec::with_capacity(count);
        for index in 0..count {
            // SAFETY: index is below the count returned for this live result;
            // native text is copied before ResultGuard releases the handle.
            let text = copy_word_text(
                unsafe { (self.api.result_word_text)(guard.result, 0, index) },
                index,
            )?;
            // SAFETY: both accessors use the same live result and checked index.
            let start_ms = unsafe { (self.api.result_word_start)(guard.result, 0, index) };
            let end_ms = unsafe { (self.api.result_word_end)(guard.result, 0, index) };
            words.push(NativeWord {
                text,
                start_ms,
                end_ms,
            });
        }
        Ok(NativeResult { words })
    }
}

impl Drop for NativeRecognizer {
    fn drop(&mut self) {
        if !self.recognizer.is_null() {
            // SAFETY: this is the unique owned recognizer handle, destroyed once
            // while Api still keeps the NeMo DLL mapped; the handle is nulled.
            unsafe { (self.api.destroy)(self.recognizer) };
            self.recognizer = std::ptr::null_mut();
        }
    }
}

struct ResultGuard<'a> {
    api: &'a Api,
    result: *mut c_void,
}
impl Drop for ResultGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: ResultGuard uniquely owns a successful recognize result and
        // drops it before the recognizer/API or process-lifetime DLL can end.
        unsafe { (self.api.result_destroy)(self.result) }
    }
}

fn native_error(api: &Api, status: i32) -> String {
    // SAFETY: NeMo exposes its last-error string as NUL-terminated storage valid
    // until the next API call; this function copies it immediately.
    let message = copy_c_string(unsafe { (api.last_error)() }, "native error")
        .unwrap_or_else(|_| "unknown native error".into());
    format!("NativeStatus: {status}: {message}")
}

fn copy_c_string(ptr: *const c_char, label: &str) -> Result<String, String> {
    if ptr.is_null() {
        return Err(format!("NativeResult: null {label}"));
    }
    // SAFETY: all callers pass pointers documented by the NeMo ABI as valid
    // NUL-terminated strings for the owning result/DLL lifetime. The ABI is the
    // source of the termination guarantee; this copies the bytes immediately.
    let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes();
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(format!("NativeResult: {label} exceeds bound"));
    }
    decode_utf8_fragments(&[bytes], label)
}

fn copy_word_text(ptr: *const c_char, index: usize) -> Result<String, String> {
    if ptr.is_null() {
        return Err(format!("NativeResult: null word {index}"));
    }
    // SAFETY: the ABI returns a NUL-terminated word string borrowed from the
    // live result; it is copied before ResultGuard releases that result.
    let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes();
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(format!("NativeResult: word {index} exceeds bound"));
    }
    decode_utf8_fragments(&[bytes], &format!("word {index}"))
}

fn decode_utf8_fragments(fragments: &[&[u8]], label: &str) -> Result<String, String> {
    let byte_count = fragments
        .iter()
        .try_fold(0_usize, |total, fragment| total.checked_add(fragment.len()));
    let Some(byte_count) = byte_count.filter(|count| *count <= 16 * 1024 * 1024) else {
        return Err(format!("NativeResult: {label} exceeds bound"));
    };
    let mut bytes = Vec::with_capacity(byte_count);
    for fragment in fragments {
        bytes.extend_from_slice(fragment);
    }
    let decoded = String::from_utf8(bytes).map_err(|error| {
        format!(
            "InvalidUtf8: {label}: invalid byte sequence at {}",
            error.utf8_error().valid_up_to()
        )
    })?;
    if decoded.contains('\u{fffd}') {
        return Err(format!(
            "InvalidUtf8: {label}: replacement character is forbidden"
        ));
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::decode_utf8_fragments;

    #[test]
    fn utf8_character_split_between_tokens_is_decoded_after_word_assembly() {
        let word = decode_utf8_fragments(&[b"caf\xc3", b"\xa9"], "word 4").unwrap();
        assert_eq!(word, "café");
    }

    #[test]
    fn invalid_word_utf8_is_attributed_without_replacement() {
        let error = decode_utf8_fragments(&[b"bad\xff"], "word 7").unwrap_err();
        assert!(error.starts_with("InvalidUtf8: word 7:"));
        assert!(!error.contains('\u{fffd}'));
    }

    #[test]
    fn literal_replacement_character_is_rejected() {
        let error = decode_utf8_fragments(&["bad\u{fffd}".as_bytes()], "word 8").unwrap_err();
        assert!(error.starts_with("InvalidUtf8: word 8:"));
    }
}
