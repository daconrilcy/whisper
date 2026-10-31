use super::*;

pub(super) fn finalize_incomplete_outputs(
    request: &LiveRequest,
    wav: WavOutput,
    segments: &[WorkerSegment],
    error: String,
) -> String {
    let wav_path = match wav.finish() {
        Ok(path) => path,
        Err(save_error) => {
            return format!("{error}. WAV incomplet non finalisé : {save_error}");
        }
    };
    let gpu = request.config.compute == ComputeChoice::Gpu;
    match export_segments(request, segments, true) {
        Ok((txt, srt)) if gpu => format!(
            "{error}. Session GPU arrêtée ; résultats incomplets : {wav_path} | {txt} | {srt}. Réimportez le WAV en CPU pour obtenir le texte complet."
        ),
        Ok((txt, srt)) => format!(
            "{error}. Session micro arrêtée ; résultats incomplets : {wav_path} | {txt} | {srt}."
        ),
        Err(save_error) if gpu => format!(
            "{error}. Session GPU arrêtée ; WAV conservé : {wav_path}. TXT/SRT incomplets non enregistrés : {save_error}. Réimportez le WAV en CPU."
        ),
        Err(save_error) => format!(
            "{error}. Session micro arrêtée ; WAV conservé : {wav_path}. TXT/SRT incomplets non enregistrés : {save_error}."
        ),
    }
}

pub(crate) fn export_segments(
    request: &LiveRequest,
    segments: &[WorkerSegment],
    incomplete: bool,
) -> Result<(String, String), String> {
    let directory = PathBuf::from(&request.destination);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Dossier de sortie inaccessible : {error}"))?;
    let stem = format!("micro-{:032x}", request.job_id.0);
    let suffix = if incomplete { "-incomplet" } else { "" };
    let txt = directory.join(format!("{stem}{suffix}.txt"));
    let srt = directory.join(format!("{stem}{suffix}.srt"));
    let mut ordered = segments.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|segment| {
        (
            segment.range.start_sample,
            segment.range.end_sample,
            segment.segment_id.0,
        )
    });
    let text = ordered
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let subtitles = ordered
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            format!(
                "{}\n{} --> {}\n{}\n\n",
                index + 1,
                timestamp(segment.range.start_sample),
                timestamp(segment.range.end_sample),
                segment.text
            )
        })
        .collect::<String>();
    let mut txt_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&txt)
        .map_err(|error| format!("TXT non enregistré. Vérifiez le dossier de sortie : {error}"))?;
    if let Err(error) = txt_file
        .write_all(text.as_bytes())
        .and_then(|()| txt_file.flush())
    {
        drop(txt_file);
        let _ = fs::remove_file(&txt);
        return Err(format!(
            "TXT non enregistré. Vérifiez le dossier de sortie : {error}"
        ));
    }
    drop(txt_file);
    let mut srt_file = match OpenOptions::new().write(true).create_new(true).open(&srt) {
        Ok(file) => file,
        Err(error) => {
            let cleanup = fs::remove_file(&txt).err();
            let suffix = cleanup.map_or_else(String::new, |cleanup| {
                format!("; TXT temporaire créé par cette exportation non retiré : {cleanup}")
            });
            return Err(format!(
                "SRT non enregistré. Vérifiez le dossier de sortie : {error}{suffix}"
            ));
        }
    };
    if let Err(error) = srt_file
        .write_all(subtitles.as_bytes())
        .and_then(|()| srt_file.flush())
    {
        drop(srt_file);
        let srt_cleanup = fs::remove_file(&srt).err();
        let txt_cleanup = fs::remove_file(&txt).err();
        let suffix = [srt_cleanup, txt_cleanup]
            .into_iter()
            .flatten()
            .map(|cleanup| format!("; sortie créée par cette exportation non retirée : {cleanup}"))
            .collect::<String>();
        return Err(format!(
            "SRT non enregistré. Vérifiez le dossier de sortie : {error}{suffix}"
        ));
    }
    Ok((
        txt.to_string_lossy().into_owned(),
        srt.to_string_lossy().into_owned(),
    ))
}

pub(crate) fn automatic_final_output_paths(
    wav_path: &str,
) -> Result<whisper_core::ports::AutomaticFinalOutputPaths, String> {
    let wav = Path::new(wav_path);
    let parent = wav
        .parent()
        .ok_or_else(|| "WAV sans dossier parent".to_owned())?;
    let stem = wav
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Nom de capture WAV invalide".to_owned())?;
    Ok(whisper_core::ports::AutomaticFinalOutputPaths {
        txt_path: parent
            .join(format!("{stem}-auto-non-verifie.txt"))
            .to_string_lossy()
            .into_owned(),
        srt_path: parent
            .join(format!("{stem}-auto-non-verifie.srt"))
            .to_string_lossy()
            .into_owned(),
        companion_json_path: wav.with_extension("json").to_string_lossy().into_owned(),
    })
}

pub(crate) fn export_parakeet_manifest(
    request: &LiveRequest,
    wav_path: &str,
    audio_samples: u64,
    segments: &[WorkerSegment],
    unresolved: &[SourceRange],
    complete: bool,
) -> Result<String, String> {
    let wav = PathBuf::from(wav_path);
    let mut json = wav.clone();
    json.set_extension("json");
    let mut ordered = segments.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|segment| {
        (
            segment.range.start_sample,
            segment.range.end_sample,
            segment.segment_id.0,
        )
    });
    let automatic_outputs = automatic_final_output_paths(wav_path)?;
    let document = serde_json::json!({
        "version": 1,
        "capture_id": format!("{:032x}", request.job_id.0),
        "capture_name": wav.file_name().and_then(|name| name.to_str()).unwrap_or_default(),
        "audio_samples": audio_samples,
        "sample_rate_hz": 16_000,
        "complete": complete,
        "confirmed_intervals": ordered.iter().map(|segment| serde_json::json!({
            "segment_id": segment.segment_id.0,
            "start_sample": segment.range.start_sample.saturating_sub(request.group_offset_samples),
            "end_sample": segment.range.end_sample.saturating_sub(request.group_offset_samples),
            "sample_rate_hz": segment.range.sample_rate_hz,
            "text": segment.text.as_str(),
            "status": "confirmed",
        })).collect::<Vec<_>>(),
        "unresolved_intervals": unresolved.iter().map(|range| serde_json::json!({
            "start_sample": range.start_sample.saturating_sub(request.group_offset_samples),
            "end_sample": range.end_sample.saturating_sub(request.group_offset_samples),
            "sample_rate_hz": range.sample_rate_hz,
            "status": "unresolved",
        })).collect::<Vec<_>>(),
        "automatic_final_transcription": {
            "status": "pending",
            "txt_path": automatic_outputs.txt_path,
            "srt_path": automatic_outputs.srt_path,
            "model_sha256": request.model_sha256.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            "requested_compute": format!("{:?}", request.config.compute).to_ascii_lowercase(),
            "effective_backend": serde_json::Value::Null,
            "diagnostic": serde_json::Value::Null,
        },
    });
    let bytes =
        serde_json::to_vec_pretty(&document).map_err(|error| format!("JSON invalide : {error}"))?;
    fs::write(&json, bytes).map_err(|error| format!("écriture JSON impossible : {error}"))?;
    Ok(json.to_string_lossy().into_owned())
}

pub(crate) struct ParakeetExportOutcome {
    pub txt_path: Option<String>,
    pub srt_path: Option<String>,
    pub complete: bool,
    pub diagnostics: Vec<String>,
}

pub(crate) fn export_parakeet_outputs(
    request: &LiveRequest,
    wav_path: &str,
    audio_samples: u64,
    segments: &[WorkerSegment],
    unresolved: &[SourceRange],
    mut complete: bool,
) -> ParakeetExportOutcome {
    let mut diagnostics = Vec::new();
    let (txt_path, srt_path) = match export_segments(request, segments, !complete) {
        Ok(paths) => (Some(paths.0), Some(paths.1)),
        Err(error) => {
            diagnostics.push(format!("TXT/SRT non enregistrés : {error}"));
            complete = false;
            match export_segments(request, segments, true) {
                Ok(paths) => (Some(paths.0), Some(paths.1)),
                Err(retry_error) => {
                    diagnostics.push(format!(
                        "TXT/SRT incomplets non enregistrés : {retry_error}"
                    ));
                    (None, None)
                }
            }
        }
    };
    if wav_path.is_empty() {
        diagnostics.push("JSON de capture non enregistré : chemin WAV indisponible".into());
        complete = false;
    } else if let Err(error) = export_parakeet_manifest(
        request,
        wav_path,
        audio_samples,
        segments,
        unresolved,
        complete,
    ) {
        diagnostics.push(format!("JSON de capture non enregistré : {error}"));
        complete = false;
    }
    ParakeetExportOutcome {
        txt_path,
        srt_path,
        complete,
        diagnostics,
    }
}

fn timestamp(samples: u64) -> String {
    let millis = samples.saturating_mul(1_000) / 16_000;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        millis / 3_600_000,
        millis / 60_000 % 60,
        millis / 1_000 % 60,
        millis % 1_000
    )
}
