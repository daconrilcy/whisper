use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};
use whisper_core::path_settings::{PathField, PathPort, StoredPaths};

pub struct WindowsPathPort {
    file: PathBuf,
    sender: Option<Sender<StoredPaths>>,
    results: Receiver<Result<(), String>>,
    worker: Option<JoinHandle<()>>,
    pending: usize,
}

impl WindowsPathPort {
    pub fn new() -> Self {
        let file = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join("Whisper")
            .join("settings.json");
        let (sender, receiver) = mpsc::channel();
        let (result_sender, results) = mpsc::channel();
        let path = file.clone();
        let worker = thread::spawn(move || {
            for (sequence, paths) in receiver.into_iter().enumerate() {
                let result = write_settings(&path, sequence, &paths);
                let _ = result_sender.send(result);
            }
        });
        Self {
            file,
            sender: Some(sender),
            results,
            worker: Some(worker),
            pending: 0,
        }
    }

    pub fn settings_path(&self) -> &Path {
        &self.file
    }
}

impl Default for WindowsPathPort {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WindowsPathPort {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl PathPort for WindowsPathPort {
    fn load(&self) -> Result<Option<StoredPaths>, String> {
        if !self.file.is_absolute() {
            return Err("LOCALAPPDATA indisponible".into());
        }
        let bytes = match fs::read(&self.file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| error.to_string())
    }

    fn save(&mut self, paths: StoredPaths) -> Result<(), String> {
        if !self.file.is_absolute() {
            return Err("LOCALAPPDATA indisponible".into());
        }
        self.sender
            .as_ref()
            .ok_or_else(|| "écriture arrêtée".to_owned())?
            .send(paths)
            .map_err(|error| error.to_string())?;
        self.pending += 1;
        Ok(())
    }

    fn pick(&mut self, field: PathField, directory: &Path) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let mut dialog = rfd::FileDialog::new();
            if !directory.as_os_str().is_empty() {
                dialog = dialog.set_directory(directory);
            }
            match field {
                PathField::Audio => dialog
                    .add_filter("Audio WAV/MP3", &["wav", "mp3"])
                    .pick_file(),
                PathField::Model => dialog.add_filter("Modèle BIN", &["bin"]).pick_file(),
                PathField::Destination => dialog.pick_folder(),
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (field, directory);
            None
        }
    }

    fn exists(&self, path: &Path, field: PathField) -> bool {
        match field {
            PathField::Destination => path.is_dir(),
            PathField::Audio | PathField::Model => {
                let extension = path.extension().and_then(|value| value.to_str());
                path.is_file()
                    && match field {
                        PathField::Audio => extension.is_some_and(|value| {
                            value.eq_ignore_ascii_case("wav") || value.eq_ignore_ascii_case("mp3")
                        }),
                        PathField::Model => {
                            extension.is_some_and(|value| value.eq_ignore_ascii_case("bin"))
                        }
                        PathField::Destination => false,
                    }
            }
        }
    }

    fn take_save_error(&mut self) -> Option<String> {
        let mut error = None;
        while let Ok(result) = self.results.try_recv() {
            self.pending = self.pending.saturating_sub(1);
            if let Err(message) = result {
                error = Some(format!("Chemins utilisés, mais non mémorisés : {message}"));
            }
        }
        error
    }

    fn has_pending_save(&self) -> bool {
        self.pending > 0
    }
}

fn write_settings(path: &Path, sequence: usize, paths: &StoredPaths) -> Result<(), String> {
    let parent = path.parent().ok_or("profil utilisateur introuvable")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = parent.join(format!("settings-{}-{sequence}.tmp", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        serde_json::to_writer_pretty(&mut file, paths).map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        fs::rename(&temporary, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
