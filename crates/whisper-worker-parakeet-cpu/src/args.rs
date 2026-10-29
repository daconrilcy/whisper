use std::{env, path::PathBuf};
use whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;
use whisper_core::parakeet_windows::{PROFILE_IMPORT_VAD, PROFILE_QUALIFICATION_REFERENCE};

pub(crate) fn parse_args() -> Result<Args, String> {
    let mut model = None;
    let mut model_sha256 = None;
    let mut native_dir = None;
    #[allow(unused_mut)]
    let mut profile = PARAKEET_PROFILE_IMPORT_SINGLE_PASS.to_owned();
    #[allow(unused_mut)]
    let mut profile_was_set = false;
    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--qualification-reference" => {
                #[cfg(feature = "parakeet-qualification-reference")]
                {
                    if profile_was_set {
                        return Err("InvalidProfile: profile flags cannot be combined".into());
                    }
                    profile = PROFILE_QUALIFICATION_REFERENCE.to_owned();
                    profile_was_set = true;
                    continue;
                }
                #[cfg(not(feature = "parakeet-qualification-reference"))]
                return Err("QualificationReferenceUnavailable".into());
            }
            _ => {
                let value = args
                    .next()
                    .ok_or_else(|| format!("missing value for {}", arg.to_string_lossy()))?;
                match arg.to_string_lossy().as_ref() {
                    "--model" => model = Some(PathBuf::from(value)),
                    "--model-sha256" => {
                        model_sha256 = Some(value.to_string_lossy().to_ascii_lowercase())
                    }
                    "--native-dir" => native_dir = Some(PathBuf::from(value)),
                    "--profile" => {
                        if profile_was_set {
                            return Err("InvalidProfile: profile may be specified only once".into());
                        }
                        let requested = value.to_string_lossy().to_string();
                        if requested == PROFILE_IMPORT_VAD {
                            #[cfg(feature = "parakeet-qualification-reference")]
                            {
                                profile = requested;
                                profile_was_set = true;
                            }
                            #[cfg(not(feature = "parakeet-qualification-reference"))]
                            return Err("ProfileUnavailable: import-vad".into());
                        } else {
                            return Err(format!("InvalidProfile: {requested}"));
                        }
                    }
                    _ => return Err(format!("unknown argument {}", arg.to_string_lossy())),
                }
            }
        }
    }
    let model_sha256 = model_sha256.ok_or("missing --model-sha256")?;
    if model_sha256.len() != 64 || !model_sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("invalid --model-sha256".into());
    }
    Ok(Args {
        model: model.ok_or("missing --model")?,
        model_sha256,
        native_dir: native_dir.ok_or("missing --native-dir")?,
        profile,
    })
}

pub(crate) struct Args {
    pub(crate) model: PathBuf,
    pub(crate) model_sha256: String,
    pub(crate) native_dir: PathBuf,
    pub(crate) profile: String,
}
