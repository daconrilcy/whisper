#[cfg(test)]
use super::*;

#[derive(Clone)]
#[cfg_attr(not(test), derive(Default))]
pub(super) struct D2WorkerDiagnostics {
    #[cfg(test)]
    path: Option<PathBuf>,
    #[cfg(test)]
    write_lock: Arc<std::sync::Mutex<()>>,
}

#[cfg(test)]
impl Default for D2WorkerDiagnostics {
    fn default() -> Self {
        Self {
            path: None,
            write_lock: Arc::new(std::sync::Mutex::new(())),
        }
    }
}

impl D2WorkerDiagnostics {
    #[cfg(test)]
    pub(super) fn for_d2_destination(destination: &Path) -> Result<Self, String> {
        let root = env::var_os("WHISPER_MIC_LEVEL_D2_ROOT")
            .map(PathBuf::from)
            .ok_or_else(|| "D2 diagnostics root is absent".to_owned())?
            .canonicalize()
            .map_err(|error| format!("D2 diagnostics root: {error}"))?;
        let destination = destination
            .canonicalize()
            .map_err(|error| format!("D2 case root: {error}"))?;
        if !destination.starts_with(&root) {
            return Err("D2 diagnostics destination is outside the guarded root".into());
        }
        Ok(Self {
            path: Some(destination.join("d2-worker-events.jsonl")),
            write_lock: Arc::new(std::sync::Mutex::new(())),
        })
    }

    pub(super) fn enabled(&self) -> bool {
        #[cfg(test)]
        {
            return self.path.is_some();
        }
        #[cfg(not(test))]
        false
    }

    pub(super) fn record(&self, value: serde_json::Value) {
        #[cfg(test)]
        if let Some(path) = &self.path {
            if let Ok(_guard) = self.write_lock.lock()
                && let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path)
            {
                let _ = serde_json::to_writer(&mut file, &value);
                let _ = file.write_all(b"\n");
            }
        }
        #[cfg(not(test))]
        let _ = value;
    }
}
