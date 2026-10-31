use super::*;

pub(super) fn fill_pending_from_wav(
    wav: &mut WavOutput,
    next_offset: u64,
    pending: &mut VecDeque<i16>,
) -> Result<(), String> {
    let loaded_until = next_offset
        .checked_add(pending.len() as u64)
        .ok_or_else(|| "Backlog WAV hors limites".to_owned())?;
    let available = wav.samples.saturating_sub(loaded_until);
    let free = MAX_BUFFERED_SAMPLES.saturating_sub(pending.len());
    let count = available.min(free as u64) as usize;
    if count > 0 {
        pending.extend(wav.read_samples(loaded_until, count)?);
    }
    Ok(())
}

pub(crate) struct WavOutput {
    file: fs::File,
    path: PathBuf,
    pub(super) samples: u64,
}

impl WavOutput {
    pub(crate) fn create(request: &LiveRequest) -> Result<Self, String> {
        let directory = PathBuf::from(&request.destination);
        fs::create_dir_all(&directory)
            .map_err(|error| format!("Dossier de sortie inaccessible : {error}"))?;
        let path = directory.join(format!("micro-{:032x}.wav", request.job_id.0));
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("WAV non créé : {error}"))?;
        write_wav_header(&mut file, 0)?;
        Ok(Self {
            file,
            path,
            samples: 0,
        })
    }

    pub(crate) fn append(&mut self, samples: &[i16]) -> Result<(), String> {
        let total = self
            .samples
            .checked_add(samples.len() as u64)
            .ok_or_else(|| "WAV trop long".to_owned())?;
        if total > (u32::MAX as u64 - 36) / 2 {
            return Err("WAV trop long pour le format PCM".into());
        }
        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        self.file
            .write_all(&bytes)
            .map_err(|error| format!("WAV non écrit : {error}"))?;
        self.samples = total;
        Ok(())
    }

    pub(crate) fn read_samples(&mut self, offset: u64, count: usize) -> Result<Vec<i16>, String> {
        if offset
            .checked_add(count as u64)
            .is_none_or(|end| end > self.samples)
        {
            return Err("Backlog WAV hors limites".into());
        }
        self.file
            .flush()
            .map_err(|error| format!("WAV non synchronisé : {error}"))?;
        self.file
            .seek(SeekFrom::Start(44 + offset * 2))
            .map_err(|error| format!("Backlog WAV inaccessible : {error}"))?;
        let mut bytes = vec![0; count * 2];
        self.file
            .read_exact(&mut bytes)
            .map_err(|error| format!("Backlog WAV tronqué : {error}"))?;
        self.file
            .seek(SeekFrom::End(0))
            .map_err(|error| format!("WAV non repositionné : {error}"))?;
        Ok(bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| i16::from_le_bytes(*pair))
            .collect())
    }

    pub(crate) fn sample_count(&self) -> u64 {
        self.samples
    }

    pub(crate) fn finish(mut self) -> Result<String, String> {
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|error| format!("WAV non finalisé : {error}"))?;
        write_wav_header(&mut self.file, self.samples as u32)?;
        self.file
            .flush()
            .map_err(|error| format!("WAV non finalisé : {error}"))?;
        Ok(self.path.to_string_lossy().into_owned())
    }
}

impl Drop for WavOutput {
    fn drop(&mut self) {
        if self.file.seek(SeekFrom::Start(0)).is_ok() {
            let _ = write_wav_header(&mut self.file, self.samples as u32);
            let _ = self.file.flush();
        }
    }
}

pub(super) fn write_wav_header(writer: &mut impl Write, samples: u32) -> Result<(), String> {
    let data_bytes = samples * 2;
    writer
        .write_all(b"RIFF")
        .and_then(|_| writer.write_all(&(36 + data_bytes).to_le_bytes()))
        .and_then(|_| writer.write_all(b"WAVEfmt "))
        .and_then(|_| writer.write_all(&16u32.to_le_bytes()))
        .and_then(|_| writer.write_all(&1u16.to_le_bytes()))
        .and_then(|_| writer.write_all(&1u16.to_le_bytes()))
        .and_then(|_| writer.write_all(&16_000u32.to_le_bytes()))
        .and_then(|_| writer.write_all(&32_000u32.to_le_bytes()))
        .and_then(|_| writer.write_all(&2u16.to_le_bytes()))
        .and_then(|_| writer.write_all(&16u16.to_le_bytes()))
        .and_then(|_| writer.write_all(b"data"))
        .and_then(|_| writer.write_all(&data_bytes.to_le_bytes()))
        .map_err(|error| format!("En-tête WAV non écrit : {error}"))
}
