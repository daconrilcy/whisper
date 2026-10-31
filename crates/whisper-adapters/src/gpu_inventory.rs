use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use whisper_core::gpu_inventory::{
    GpuAvailability, GpuCause, GpuIdentityMatch, GpuQualifiedObservation, GpuSupport,
};
use whisper_core::gpu_inventory::{GpuObservation, GpuPresence, GpuState};
use whisper_core::gpu_inventory_app::{GpuCollection, GpuInventoryPort, GpuQualifiedCollection};

#[cfg(windows)]
mod windows_native;

const DISPLAY_CLASS_GUID: &str = "{4d36e968-e325-11ce-bfc1-08002be10318}";

#[derive(Clone, Debug, Eq, PartialEq)]
struct DxgiLogEntry {
    identity: Option<String>,
    windows_rank: u32,
    luid: u64,
    identity_error: Option<String>,
}

fn gpu_query() -> String {
    format!(
        "SELECT DeviceID, Name, ConfigManagerErrorCode, Present FROM Win32_PnPEntity WHERE ClassGuid = '{DISPLAY_CLASS_GUID}'"
    )
}

pub struct WindowsGpuInventoryPort {
    receiver: Option<Receiver<(u64, Result<GpuCollection, String>)>>,
    generation: Option<u64>,
    collecting: Arc<AtomicBool>,
    worker: PathBuf,
    qualified_receiver: Option<Receiver<(u64, Result<GpuQualifiedCollection, String>)>>,
    runtime_receiver: Option<
        Receiver<(
            u64,
            Result<Vec<whisper_core::gpu_inventory::GpuRuntimeIdentity>, String>,
        )>,
    >,
}

impl WindowsGpuInventoryPort {
    pub fn new(worker: PathBuf) -> Self {
        Self {
            worker,
            ..Self::default()
        }
    }
}

pub fn map_device_fields(
    identity: Option<String>,
    name: Option<String>,
    config_manager_error_code: Option<u32>,
    present: Option<bool>,
) -> GpuObservation {
    let state = match config_manager_error_code {
        Some(0) => Some(GpuState::Ready),
        Some(22) => Some(GpuState::Disabled),
        Some(code) => Some(GpuState::Error(code)),
        None => None,
    };
    let presence = if identity.as_deref().is_none_or(str::is_empty) {
        GpuPresence::Unknown
    } else {
        match present {
            Some(true) => GpuPresence::Current,
            Some(false) => GpuPresence::Absent,
            None => GpuPresence::Unknown,
        }
    };
    GpuObservation {
        identity: identity.unwrap_or_default(),
        name,
        state,
        presence,
    }
}

impl Default for WindowsGpuInventoryPort {
    fn default() -> Self {
        Self {
            receiver: None,
            generation: None,
            collecting: Arc::new(AtomicBool::new(false)),
            worker: PathBuf::from("whisper-worker-gpu.exe"),
            qualified_receiver: None,
            runtime_receiver: None,
        }
    }
}

impl GpuInventoryPort for WindowsGpuInventoryPort {
    fn start(&mut self, generation: u64) -> Result<(), String> {
        if self.collecting.swap(true, Ordering::AcqRel) {
            return Err("Une collecte GPU est déjà en cours.".into());
        }
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        let (qualified_sender, qualified_receiver) = mpsc::channel();
        self.qualified_receiver = Some(qualified_receiver);
        let (runtime_sender, runtime_receiver) = mpsc::channel();
        self.runtime_receiver = Some(runtime_receiver);
        self.generation = Some(generation);
        let collecting = Arc::clone(&self.collecting);
        let worker = self.worker.clone();
        std::thread::Builder::new()
            .name("whisper-gpu-inventory".into())
            .spawn(move || {
                let result = collect_windows();
                let _ = sender.send((generation, result.clone()));
                let (qualified, runtime) = match result {
                    Err(error) => {
                        log_inventory(generation, &[], &[], &[], None, Some(&error), None);
                        (Err(error.clone()), Err(error))
                    }
                    Ok(collection) => {
                        #[cfg(windows)]
                        let (joined, dxgi_log) = match windows_native::enumerate_dxgi() {
                            Ok(adapters) => {
                                let mut joined = Vec::new();
                                let mut dxgi_log = Vec::with_capacity(adapters.len());
                                for adapter in adapters {
                                    let identity =
                                        windows_native::adapter_instance_id(adapter.luid);
                                    dxgi_log.push(DxgiLogEntry {
                                        identity: identity.as_ref().ok().cloned(),
                                        windows_rank: adapter.ordinal,
                                        luid: windows_native::luid_u64(adapter.luid),
                                        identity_error: identity.as_ref().err().cloned(),
                                    });
                                    if let Ok(identity) = identity {
                                        joined.push((identity.to_ascii_uppercase(), adapter));
                                    }
                                }
                                (joined, dxgi_log)
                            }
                            Err(error) => {
                                let message = error.clone();
                                log_inventory(
                                    generation,
                                    &collection.observations,
                                    &[],
                                    &[],
                                    None,
                                    Some(&message),
                                    None,
                                );
                                let qualified = unqualified_collection(collection, error);
                                let _ = qualified_sender.send((generation, Ok(qualified)));
                                let _ = runtime_sender.send((generation, Err(message)));
                                collecting.store(false, Ordering::Release);
                                return;
                            }
                        };
                        #[cfg(not(windows))]
                        let joined: Vec<(String, ())> = Vec::new();
                        #[cfg(not(windows))]
                        let dxgi_log: Vec<DxgiLogEntry> = Vec::new();
                        let targets = collection
                            .observations
                            .iter()
                            .map(|item| {
                                #[cfg(windows)]
                                let matches = joined
                                    .iter()
                                    .filter(|(id, _)| id == &item.identity.to_ascii_uppercase())
                                    .collect::<Vec<_>>();
                                #[cfg(not(windows))]
                                let matches: Vec<&(String, ())> = Vec::new();
                                let adapter = unique_adapter(&matches);
                                whisper_core::ipc::GpuQualificationTarget {
                                    generation,
                                    identity: item.identity.clone(),
                                    name: item.name.clone(),
                                    state: Some(format!("{:?}", item.state)),
                                    presence: format!("{:?}", item.presence),
                                    windows_rank: adapter.map(|device| device.ordinal),
                                    #[cfg(windows)]
                                    luid: adapter
                                        .map(|device| windows_native::luid_u64(device.luid)),
                                    #[cfg(not(windows))]
                                    luid: None,
                                    pci_id: None,
                                }
                            })
                            .collect::<Vec<_>>();
                        let response = crate::worker_ipc::qualify_gpu_worker(
                            &worker,
                            generation,
                            targets.clone(),
                        );
                        log_inventory(
                            generation,
                            &collection.observations,
                            &dxgi_log,
                            &targets,
                            response
                                .as_ref()
                                .ok()
                                .map(|(_, results)| results.as_slice()),
                            collection.error.as_deref(),
                            response.as_ref().err().map(String::as_str),
                        );
                        let runtime = response
                            .as_ref()
                            .map(|(_, results)| runtime_identities(&targets, results))
                            .map_err(Clone::clone);
                        (
                            Ok(qualified_collection(collection, &targets, response)),
                            runtime,
                        )
                    }
                };
                let _ = qualified_sender.send((generation, qualified));
                let _ = runtime_sender.send((generation, runtime));
                collecting.store(false, Ordering::Release);
            })
            .map_err(|error| {
                self.collecting.store(false, Ordering::Release);
                format!("Impossible de démarrer la collecte GPU : {error}")
            })?;
        Ok(())
    }

    fn uses_qualified_catalogue(&self) -> bool {
        true
    }

    fn poll_qualified(&mut self) -> Option<(u64, Result<GpuQualifiedCollection, String>)> {
        match self.qualified_receiver.as_ref()?.try_recv() {
            Ok(value) => {
                self.qualified_receiver = None;
                Some(value)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.qualified_receiver = None;
                Some((
                    self.generation.take()?,
                    Err("La qualification GPU s’est interrompue.".into()),
                ))
            }
        }
    }

    fn poll_runtime_identities(
        &mut self,
    ) -> Option<(
        u64,
        Result<Vec<whisper_core::gpu_inventory::GpuRuntimeIdentity>, String>,
    )> {
        match self.runtime_receiver.as_ref()?.try_recv() {
            Ok(value) => {
                self.runtime_receiver = None;
                Some(value)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.runtime_receiver = None;
                Some((
                    self.generation?,
                    Err("La preuve runtime GPU s’est interrompue.".into()),
                ))
            }
        }
    }

    fn poll(&mut self) -> Option<(u64, Result<GpuCollection, String>)> {
        match self.receiver.as_ref()?.try_recv() {
            Ok(result) => {
                self.receiver = None;
                self.generation = None;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                Some((
                    self.generation.take()?,
                    Err("La collecte GPU s’est interrompue.".into()),
                ))
            }
        }
    }
}

fn log_inventory(
    generation: u64,
    observations: &[GpuObservation],
    dxgi: &[DxgiLogEntry],
    targets: &[whisper_core::ipc::GpuQualificationTarget],
    results: Option<&[whisper_core::ipc::GpuQualificationResult]>,
    error: Option<&str>,
    qualification_error: Option<&str>,
) {
    let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
        let message = "GPU inventory log unavailable: LOCALAPPDATA is unset".to_owned();
        eprintln!("{message}");
        return;
    };
    let path = PathBuf::from(local_app_data)
        .join("Whisper")
        .join("gpu-inventory.log");
    if let Err(error) = append_inventory_log(
        &path,
        generation,
        observations,
        dxgi,
        targets,
        results,
        error,
        qualification_error,
    ) {
        let message = format!(
            "GPU inventory log append failed ({}): {error}",
            path.display()
        );
        eprintln!("{message}");
    }
}

fn append_inventory_log(
    path: &std::path::Path,
    generation: u64,
    observations: &[GpuObservation],
    dxgi: &[DxgiLogEntry],
    targets: &[whisper_core::ipc::GpuQualificationTarget],
    results: Option<&[whisper_core::ipc::GpuQualificationResult]>,
    error: Option<&str>,
    qualification_error: Option<&str>,
) -> Result<(), String> {
    let timestamp_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let value = serde_json::json!({
        "timestamp_unix_ms": timestamp_unix_ms,
        "generation": generation,
        "observations": observations.iter().map(|item| serde_json::json!({
            "name": item.name,
            "identity": item.identity,
            "state": format!("{:?}", item.state),
            "presence": format!("{:?}", item.presence),
        })).collect::<Vec<_>>(),
        "dxgi": dxgi.iter().map(|entry| serde_json::json!({
            "identity": entry.identity,
            "windows_rank": entry.windows_rank,
            "luid": entry.luid,
            "identity_error": entry.identity_error,
        })).collect::<Vec<_>>(),
        "qualification_targets": targets.iter().map(|target| serde_json::json!({
            "identity": target.identity,
            "name": target.name,
            "windows_rank": target.windows_rank,
            "luid": target.luid,
            "pci_id": target.pci_id,
        })).collect::<Vec<_>>(),
        "qualification": results.map(|items| items.iter().map(|item| serde_json::json!({
            "identity": item.identity,
            "windows_rank": item.windows_rank,
            "luid": item.luid,
            "pci_id": item.pci_id,
            "cuda_ordinal": item.cuda_ordinal,
            "ggml_ordinal": item.ggml_ordinal,
            "identity_match": item.identity_match,
            "support": item.support,
            "availability": item.availability,
            "cause": item.cause,
        })).collect::<Vec<_>>()),
        "error": error,
        "qualification_error": qualification_error,
    });
    let mut line = serde_json::to_vec(&value).map_err(|error| error.to_string())?;
    line.push(b'\n');
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(&line).map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())
}

fn qualified_collection(
    collection: GpuCollection,
    targets: &[whisper_core::ipc::GpuQualificationTarget],
    response: Result<(String, Vec<whisper_core::ipc::GpuQualificationResult>), String>,
) -> GpuQualifiedCollection {
    let (build, results, worker_error) = match response {
        Ok((build, results)) => (build, results, None),
        Err(error) => ("unknown build".into(), Vec::new(), Some(error)),
    };
    let mut error = collection.error;
    if let Some(worker_error) = worker_error.clone() {
        error = Some(match error {
            Some(collection_error) => format!("{collection_error}; {worker_error}"),
            None => worker_error,
        });
    }
    let observations = collection
        .observations
        .into_iter()
        .map(|observation| {
            let result = results
                .iter()
                .find(|result| result.identity == observation.identity);
            let target = targets
                .iter()
                .find(|target| target.identity == observation.identity);
            let identity_match = match result.map(|r| r.identity_match.as_str()) {
                Some("Unique") => GpuIdentityMatch::Unique,
                Some("Ambiguous") => GpuIdentityMatch::Ambiguous,
                Some("Conflicting") => GpuIdentityMatch::Conflicting,
                _ => GpuIdentityMatch::Missing,
            };
            let support = match result.map(|r| r.support.as_str()) {
                Some("ConfirmedSupported") => GpuSupport::ConfirmedSupported,
                Some("ConfirmedUnsupported") => GpuSupport::ConfirmedUnsupported,
                _ => GpuSupport::Undetermined,
            };
            let availability = if result.is_some_and(|r| r.availability == "Unavailable") {
                GpuAvailability::Unavailable
            } else if result.is_some_and(|r| r.availability == "Available") {
                GpuAvailability::Available
            } else {
                GpuAvailability::NotVerified
            };
            let mut causes = Vec::new();
            if let Some(cause) = result.and_then(|r| r.cause.clone()) {
                causes.push(GpuCause::Collection(cause));
            }
            if result.is_none() {
                causes.push(GpuCause::Collection(worker_error.clone().unwrap_or_else(
                    || "Réponse worker absente pour cette identité.".into(),
                )));
            }
            let windows_rank = target
                .and_then(|target| target.windows_rank)
                .map(|ordinal| whisper_core::gpu_inventory::GpuWindowsRank {
                    ordinal,
                    source: whisper_core::gpu_inventory::GpuRankSource::DxgiEnumeration,
                });
            if result.is_some_and(|r| r.support == "Undetermined") {
                causes.push(GpuCause::SupportUndetermined);
            }
            if result.is_some_and(|r| r.availability == "NotVerified") {
                causes.push(GpuCause::AvailabilityNotVerified);
            }
            if result.is_some_and(|r| r.availability == "Unavailable") {
                causes.push(GpuCause::TemporarilyUnavailable);
            }
            if worker_error.is_some() {
                causes.push(GpuCause::Collection(format!(
                    "Build CUDA {build} non qualifié."
                )));
            }
            GpuQualifiedObservation {
                observation,
                windows_rank,
                identity_match,
                support,
                availability,
                causes,
            }
        })
        .collect();
    GpuQualifiedCollection {
        observations,
        error,
    }
}

fn runtime_identities(
    targets: &[whisper_core::ipc::GpuQualificationTarget],
    results: &[whisper_core::ipc::GpuQualificationResult],
) -> Vec<whisper_core::gpu_inventory::GpuRuntimeIdentity> {
    results
        .iter()
        .filter_map(|result| {
            let target = targets
                .iter()
                .find(|target| target.identity == result.identity)?;
            Some(whisper_core::gpu_inventory::GpuRuntimeIdentity {
                generation: target.generation,
                pnp_instance_id: result.identity.clone(),
                dxgi_rank: result.windows_rank?,
                luid: result.luid?,
                pci_id: result.pci_id.clone()?,
                cuda_ordinal: result.cuda_ordinal?,
                ggml_ordinal: result.ggml_ordinal?,
            })
        })
        .collect()
}

fn unqualified_collection(collection: GpuCollection, error: String) -> GpuQualifiedCollection {
    let observations = collection
        .observations
        .into_iter()
        .map(|observation| GpuQualifiedObservation {
            observation,
            windows_rank: None,
            identity_match: GpuIdentityMatch::Missing,
            support: GpuSupport::Undetermined,
            availability: GpuAvailability::NotVerified,
            causes: vec![GpuCause::Collection(error.clone())],
        })
        .collect();
    GpuQualifiedCollection {
        observations,
        error: Some(error),
    }
}

fn unique_adapter<'a, T>(matches: &[&'a (String, T)]) -> Option<&'a T> {
    if matches.len() == 1 {
        matches.first().map(|entry| &entry.1)
    } else {
        None
    }
}

#[cfg(windows)]
fn collect_windows() -> Result<GpuCollection, String> {
    use wmi::{Variant, WMIConnection};

    let connection = WMIConnection::new().map_err(|error| format!("WMI indisponible : {error}"))?;
    let rows = connection
        .exec_query(gpu_query())
        .map_err(|error| format!("Énumération WMI des cartes graphiques impossible : {error}"))?;
    let mut observations = Vec::new();
    let mut error = None;
    for row in rows {
        let row = match row {
            Ok(row) => row,
            Err(query_error) => {
                error = Some(format!("Énumération WMI interrompue : {query_error}"));
                break;
            }
        };
        observations.push(map_device_fields(
            row.get_property("DeviceID")
                .ok()
                .and_then(|value| variant_string(&value)),
            row.get_property("Name")
                .ok()
                .and_then(|value| variant_string(&value)),
            row.get_property("ConfigManagerErrorCode")
                .ok()
                .and_then(|value| variant_u32(&value)),
            row.get_property("Present")
                .ok()
                .and_then(|value| match value {
                    Variant::Bool(value) => Some(value),
                    _ => None,
                }),
        ));
    }
    Ok(GpuCollection {
        observations,
        error,
    })
}

#[cfg(not(windows))]
fn collect_windows() -> Result<GpuCollection, String> {
    Err("L’inventaire WMI des GPU est disponible uniquement sous Windows.".into())
}

#[cfg(windows)]
fn variant_string(value: &wmi::Variant) -> Option<String> {
    match value {
        wmi::Variant::String(value) if !value.trim().is_empty() => Some(value.clone()),
        _ => None,
    }
}

#[cfg(windows)]
fn variant_u32(value: &wmi::Variant) -> Option<u32> {
    match value {
        wmi::Variant::UI1(value) => Some(u32::from(*value)),
        wmi::Variant::UI2(value) => Some(u32::from(*value)),
        wmi::Variant::UI4(value) => Some(*value),
        wmi::Variant::I1(value) => u32::try_from(*value).ok(),
        wmi::Variant::I2(value) => u32::try_from(*value).ok(),
        wmi::Variant::I4(value) => u32::try_from(*value).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DISPLAY_CLASS_GUID, DxgiLogEntry, GpuCollection, append_inventory_log, gpu_query,
        qualified_collection, runtime_identities, unique_adapter, unqualified_collection,
    };
    use whisper_core::gpu_inventory::{
        GpuAvailability, GpuCatalogueCoverage, GpuCause, GpuIdentityMatch, GpuObservation,
        GpuPresence, GpuState, GpuSupport, qualify,
    };

    #[test]
    fn query_uses_the_display_class_guid_without_filtering_current_presence() {
        let query = gpu_query();
        assert!(query.contains(&format!("ClassGuid = '{DISPLAY_CLASS_GUID}'")));
        assert!(!query.contains("PNPClass"));
        let where_clause = query.split_once("WHERE").unwrap().1;
        assert!(!where_clause.contains("Present"));
    }

    #[test]
    fn missing_dxgi_identity_match_is_safe_and_returns_no_adapter() {
        let candidates: Vec<(String, u8)> = Vec::new();
        let matches = candidates.iter().collect::<Vec<_>>();
        assert!(unique_adapter(&matches).is_none());

        let observation = GpuObservation {
            identity: "PCI\\VEN_10DE&DEV_1B06".into(),
            name: Some("NVIDIA GPU".into()),
            state: Some(GpuState::Ready),
            presence: GpuPresence::Current,
        };
        let collection = qualified_collection(
            GpuCollection {
                observations: vec![observation],
                error: Some("DXGI rank unavailable".into()),
            },
            &[whisper_core::ipc::GpuQualificationTarget {
                generation: 1,
                identity: "PCI\\VEN_10DE&DEV_1B06".into(),
                name: None,
                state: None,
                presence: "Current".into(),
                windows_rank: None,
                luid: None,
                pci_id: None,
            }],
            Ok((
                "61-real".into(),
                vec![whisper_core::ipc::GpuQualificationResult {
                    identity: "PCI\\VEN_10DE&DEV_1B06".into(),
                    windows_rank: None,
                    luid: None,
                    pci_id: None,
                    cuda_ordinal: None,
                    ggml_ordinal: None,
                    identity_match: "Missing".into(),
                    support: "Undetermined".into(),
                    availability: "NotVerified".into(),
                    cause: None,
                }],
            )),
        );
        let catalogue = qualify(collection.observations, collection.error);
        assert_eq!(catalogue.coverage, GpuCatalogueCoverage::Partial);
        assert_eq!(catalogue.devices.len(), 1);
        assert!(catalogue.devices[0].windows_rank.is_none());
    }

    #[test]
    fn dxgi_failure_keeps_wmi_observations_as_partial_catalogue_with_cause() {
        let collection = GpuCollection {
            observations: vec![GpuObservation {
                identity: "PCI\\VEN_10DE&DEV_1B06".into(),
                name: Some("NVIDIA GPU".into()),
                state: Some(GpuState::Ready),
                presence: GpuPresence::Current,
            }],
            error: None,
        };
        let partial = unqualified_collection(collection, "DXGI enumeration failed".into());
        assert_eq!(partial.observations.len(), 1);
        assert_eq!(partial.observations[0].support, GpuSupport::Undetermined);
        assert_eq!(
            partial.observations[0].availability,
            GpuAvailability::NotVerified
        );
        assert_eq!(
            partial.observations[0].identity_match,
            GpuIdentityMatch::Missing
        );
        let catalogue = qualify(partial.observations, partial.error);
        assert_eq!(catalogue.coverage, GpuCatalogueCoverage::Partial);
        assert_eq!(catalogue.devices.len(), 1);
        assert!(catalogue.causes.iter().any(|cause| matches!(cause, GpuCause::Collection(message) if message.contains("DXGI enumeration failed"))));
    }

    #[test]
    fn runtime_proof_keeps_windows_rank_distinct_from_cuda_and_ggml_ordinals() {
        let target = whisper_core::ipc::GpuQualificationTarget {
            generation: 9,
            identity: "PCI\\VEN_10DE&DEV_1B06".into(),
            name: None,
            state: None,
            presence: "Current".into(),
            windows_rank: Some(2),
            luid: Some(44),
            pci_id: None,
        };
        let result = whisper_core::ipc::GpuQualificationResult {
            identity: target.identity.clone(),
            windows_rank: Some(2),
            luid: Some(44),
            pci_id: Some("0000:03:00.0".into()),
            cuda_ordinal: Some(0),
            ggml_ordinal: Some(5),
            identity_match: "Unique".into(),
            support: "ConfirmedSupported".into(),
            availability: "Available".into(),
            cause: None,
        };
        let proof = runtime_identities(&[target], &[result]);
        assert_eq!(proof.len(), 1);
        assert_eq!(proof[0].generation, 9);
        assert_eq!(proof[0].dxgi_rank, 2);
        assert_eq!(proof[0].cuda_ordinal, 0);
        assert_eq!(proof[0].ggml_ordinal, 5);
    }

    #[test]
    fn inventory_log_appends_timestamped_partial_snapshots_and_escapes_names() {
        let path = inventory_test_path("append.jsonl");
        let observation = GpuObservation {
            identity: "PCI\\VEN_10DE&DEV_1B06".into(),
            name: Some("GPU \"quoted\"\\line".into()),
            state: Some(GpuState::Ready),
            presence: GpuPresence::Current,
        };
        append_inventory_log(
            &path,
            4,
            &[observation.clone()],
            &[],
            &[],
            None,
            Some("partial"),
            None,
        )
        .unwrap();
        append_inventory_log(
            &path,
            5,
            &[observation],
            &[],
            &[],
            None,
            Some("second partial"),
            None,
        )
        .unwrap();

        let text = std::fs::read_to_string(path).unwrap();
        let lines = text.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert!(first["timestamp_unix_ms"].as_u64().is_some());
        assert_eq!(first["generation"], 4);
        assert_eq!(first["error"], "partial");
        assert_eq!(first["observations"][0]["name"], "GPU \"quoted\"\\line");
        assert_eq!(second["generation"], 5);
    }

    #[test]
    fn inventory_log_write_failure_is_reportable_without_panicking() {
        let path = inventory_test_path("not-a-directory/entry.jsonl");
        std::fs::create_dir_all(&path).unwrap();
        assert!(
            append_inventory_log(
                &path,
                1,
                &[],
                &[],
                &[],
                None,
                Some("collection failed"),
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn inventory_log_keeps_dxgi_adapters_that_failed_identity_join() {
        let path = inventory_test_path("dxgi-unjoined.jsonl");
        let entry = DxgiLogEntry {
            identity: None,
            windows_rank: 2,
            luid: 0x1234,
            identity_error: Some("SetupAPI join failed".into()),
        };
        append_inventory_log(&path, 7, &[], &[entry], &[], None, None, None).unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        let value: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(value["dxgi"][0]["identity"], serde_json::Value::Null);
        assert_eq!(value["dxgi"][0]["windows_rank"], 2);
        assert_eq!(value["dxgi"][0]["luid"], 0x1234_u64);
        assert_eq!(value["dxgi"][0]["identity_error"], "SetupAPI join failed");
    }

    fn inventory_test_path(suffix: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join("gpu-selection-tests")
            .join(format!(
                "gpu-inventory-{}-{nonce}-{suffix}",
                std::process::id()
            ))
    }
}
