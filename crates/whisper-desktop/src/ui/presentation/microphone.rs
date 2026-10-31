use super::*;

#[derive(Clone, Debug, PartialEq)]
pub enum MicrophoneMeterPresentation {
    Awaiting,
    Stale,
    WrongCapture,
    Absent,
    Silence,
    Signal { rms_dbfs: f64, peak_dbfs: f64 },
}

pub(in crate::ui) const MICROPHONE_SATURATION_LABEL: &str = "Écrêtage après gain détecté";
pub(in crate::ui) const MICROPHONE_SATURATION_COLOR: egui::Color32 = egui::Color32::RED;

pub fn microphone_source_change_notice(
    actual: Option<&str>,
    planned: Option<&str>,
    devices: &[MicrophoneDevice],
) -> Option<String> {
    let (Some(actual), Some(planned)) = (actual, planned) else {
        return None;
    };
    if actual == planned {
        return None;
    }
    let labels = MicrophoneLabelProjection::new(devices);
    Some(format!(
        "Le microphone prévu a changé ({}), mais le flux actif reste sur {}. Arrêtez-le puis démarrez une nouvelle action pour ouvrir la nouvelle source.",
        labels.device_label(planned),
        labels.device_label(actual)
    ))
}

pub fn microphone_meter_presentation(
    meter: Option<&whisper_core::microphone_level::MicrophoneMeter>,
    active: Option<(JobId, Generation)>,
    now_ms: u64,
) -> MicrophoneMeterPresentation {
    let Some(meter) = meter else {
        return MicrophoneMeterPresentation::Awaiting;
    };
    if active != Some((meter.job_id, meter.generation)) {
        return MicrophoneMeterPresentation::WrongCapture;
    }
    if now_ms.saturating_sub(meter.captured_at_monotonic_ms) > 500 {
        return MicrophoneMeterPresentation::Stale;
    }
    use whisper_core::microphone_level::MeasurementStatus;
    match meter.post_gain_limiter.status {
        MeasurementStatus::Absent => MicrophoneMeterPresentation::Absent,
        MeasurementStatus::Silence => MicrophoneMeterPresentation::Silence,
        MeasurementStatus::Signal => MicrophoneMeterPresentation::Signal {
            rms_dbfs: meter
                .post_gain_limiter
                .rms_dbfs()
                .unwrap_or(f64::NEG_INFINITY),
            peak_dbfs: if meter.post_gain_limiter.peak() > 0.0 {
                20.0 * meter.post_gain_limiter.peak().log10()
            } else {
                f64::NEG_INFINITY
            },
        },
    }
}

pub(in crate::ui) fn microphone_saturation_is_current(
    meter: Option<&whisper_core::microphone_level::MicrophoneMeter>,
    active: Option<(JobId, Generation)>,
    source_identity: Option<&str>,
    capture_active: bool,
    now_ms: u64,
) -> bool {
    let Some(meter) = meter else {
        return false;
    };
    capture_active
        && active == Some((meter.job_id, meter.generation))
        && source_identity == Some(meter.source_identity.as_str())
        && now_ms.saturating_sub(meter.captured_at_monotonic_ms) <= 500
        && meter.limiter_samples > 0
}

pub(in crate::ui) fn microphone_test_start_enabled(
    identified_target: bool,
    view: &AppView,
) -> bool {
    identified_target
        && !view.microphone_test_active
        && !view.direct_capture_phase.is_some_and(|phase| {
            matches!(
                phase,
                whisper_core::application::DirectCapturePhase::Initializing
                    | whisper_core::application::DirectCapturePhase::Active
            )
        })
        && !view
            .active_job
            .is_some_and(|(_, state)| window_must_remain_open(state))
}
pub(in crate::ui) struct MicrophoneLabelProjection<'a> {
    devices: &'a [MicrophoneDevice],
}

impl<'a> MicrophoneLabelProjection<'a> {
    pub(in crate::ui) fn new(devices: &'a [MicrophoneDevice]) -> Self {
        Self { devices }
    }

    pub(in crate::ui) fn device_label(&self, identity: &str) -> String {
        if identity.trim().is_empty() {
            return "Microphone indisponible".into();
        }
        let Some(device) = self.devices.iter().find(|device| device.id == identity) else {
            return "Microphone indisponible".into();
        };
        let name = if device.name.trim().is_empty() {
            "Microphone sans nom"
        } else {
            &device.name
        };
        let mut same_name_ids = self
            .devices
            .iter()
            .filter(|other| other.name == device.name)
            .map(|other| other.id.as_str())
            .collect::<Vec<_>>();
        if same_name_ids.len() <= 1 {
            return name.to_owned();
        }
        same_name_ids.sort_unstable();
        let Some(index) = same_name_ids
            .iter()
            .position(|candidate| *candidate == identity)
            .map(|index| index + 1)
        else {
            return "Microphone indisponible".into();
        };
        format!("{name} ({index})")
    }

    pub(in crate::ui) fn choice_label(&self, choice: &MicrophoneChoice) -> String {
        match choice {
            MicrophoneChoice::WindowsDefault => "Microphone par défaut de Windows".into(),
            MicrophoneChoice::Device(identity) => self.device_label(identity),
        }
    }

    pub(in crate::ui) fn actual_line(&self, identity: &str) -> String {
        format!("Microphone utilisé : {}", self.device_label(identity))
    }

    pub(in crate::ui) fn planned_line(&self, identity: &str) -> String {
        format!("Microphone prévu : {}", self.device_label(identity))
    }
}

pub(in crate::ui) fn microphone_device_label(
    device: &MicrophoneDevice,
    devices: &[MicrophoneDevice],
) -> String {
    MicrophoneLabelProjection::new(devices).device_label(&device.id)
}

pub(in crate::ui) fn microphone_choice_label(
    choice: &MicrophoneChoice,
    devices: &[MicrophoneDevice],
) -> String {
    MicrophoneLabelProjection::new(devices).choice_label(choice)
}
