#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MicrophoneChoice {
    #[default]
    WindowsDefault,
    Device(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MicrophoneDevice {
    pub id: String,
    pub name: String,
    pub unavailable_reason: Option<String>,
}

impl MicrophoneDevice {
    pub fn selectable(&self) -> bool {
        self.unavailable_reason.is_none()
    }
}

pub trait MicrophonePort {
    fn list_inputs(&self) -> Result<Vec<MicrophoneDevice>, String>;
}

pub struct NullMicrophonePort;

impl MicrophonePort for NullMicrophonePort {
    fn list_inputs(&self) -> Result<Vec<MicrophoneDevice>, String> {
        Ok(Vec::new())
    }
}

pub struct MicrophoneSettings {
    port: Box<dyn MicrophonePort>,
    preferred: MicrophoneChoice,
    effective: MicrophoneChoice,
    devices: Vec<MicrophoneDevice>,
    list_failed: bool,
    notice: Option<String>,
}

impl MicrophoneSettings {
    pub fn new(port: Box<dyn MicrophonePort>, preferred: MicrophoneChoice) -> Self {
        let mut settings = Self {
            port,
            effective: preferred.clone(),
            preferred,
            devices: Vec::new(),
            list_failed: false,
            notice: None,
        };
        let listed = settings.refresh().is_ok();
        if let MicrophoneChoice::Device(id) = &settings.preferred
            && listed
            && !settings
                .devices
                .iter()
                .any(|device| device.id == *id && device.selectable())
        {
            settings.effective = MicrophoneChoice::WindowsDefault;
            settings.notice = Some(
                "Microphone mémorisé indisponible au démarrage ; microphone par défaut de Windows utilisé. Votre choix est conservé.".into(),
            );
        }
        settings
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        match self.port.list_inputs() {
            Ok(devices) => {
                self.devices = devices;
                self.list_failed = false;
                self.notice = None;
                Ok(())
            }
            Err(error) => {
                self.devices.clear();
                self.list_failed = true;
                self.notice = Some(format!("Liste des microphones indisponible : {error}"));
                Err(error)
            }
        }
    }

    pub fn select(&mut self, choice: MicrophoneChoice) -> Result<(), String> {
        if let MicrophoneChoice::Device(id) = &choice {
            let Some(device) = self.devices.iter().find(|device| device.id == *id) else {
                return Err("Ce microphone n'est plus disponible. Actualisez la liste.".into());
            };
            if let Some(reason) = &device.unavailable_reason {
                return Err(format!("Ce microphone ne peut pas être utilisé : {reason}"));
            }
        }
        self.preferred = choice.clone();
        self.effective = choice;
        self.notice = None;
        Ok(())
    }

    pub fn preferred(&self) -> &MicrophoneChoice {
        &self.preferred
    }

    pub fn effective(&self) -> &MicrophoneChoice {
        &self.effective
    }

    pub fn devices(&self) -> &[MicrophoneDevice] {
        &self.devices
    }

    pub fn list_failed(&self) -> bool {
        self.list_failed
    }

    pub fn take_notice(&mut self) -> Option<String> {
        self.notice.take()
    }
}

#[cfg(test)]
mod tests {
    use super::{MicrophoneChoice, MicrophoneDevice, MicrophonePort, MicrophoneSettings};
    use std::{cell::RefCell, rc::Rc};

    struct ChangingPort(Rc<RefCell<Vec<MicrophoneDevice>>>);

    struct FailingPort;

    impl MicrophonePort for FailingPort {
        fn list_inputs(&self) -> Result<Vec<MicrophoneDevice>, String> {
            Err("WASAPI indisponible".into())
        }
    }

    impl MicrophonePort for ChangingPort {
        fn list_inputs(&self) -> Result<Vec<MicrophoneDevice>, String> {
            Ok(self.0.borrow().clone())
        }
    }

    fn device(id: &str) -> MicrophoneDevice {
        MicrophoneDevice {
            id: id.into(),
            name: "Microphone".into(),
            unavailable_reason: None,
        }
    }

    #[test]
    fn preferred_id_survives_temporary_default_and_recovers_on_next_launch() {
        let devices = Rc::new(RefCell::new(vec![device("wasapi:one")]));
        let port = || Box::new(ChangingPort(devices.clone())) as Box<dyn MicrophonePort>;
        let preferred = MicrophoneChoice::Device("wasapi:one".into());
        let first = MicrophoneSettings::new(port(), preferred.clone());
        assert_eq!(first.effective(), &preferred);

        devices.borrow_mut().clear();
        let mut missing = MicrophoneSettings::new(port(), preferred.clone());
        assert_eq!(missing.preferred(), &preferred);
        assert_eq!(missing.effective(), &MicrophoneChoice::WindowsDefault);
        assert!(missing.take_notice().unwrap().contains("défaut"));

        devices.borrow_mut().push(device("wasapi:one"));
        missing.refresh().unwrap();
        assert_eq!(missing.effective(), &MicrophoneChoice::WindowsDefault);
        let restored = MicrophoneSettings::new(port(), preferred.clone());
        assert_eq!(restored.effective(), &preferred);
    }

    #[test]
    fn changed_id_requires_explicit_selection() {
        let devices = Rc::new(RefCell::new(vec![device("wasapi:new")]));
        let mut settings = MicrophoneSettings::new(
            Box::new(ChangingPort(devices)),
            MicrophoneChoice::Device("wasapi:old".into()),
        );
        assert_eq!(settings.effective(), &MicrophoneChoice::WindowsDefault);
        assert!(
            settings
                .select(MicrophoneChoice::Device("wasapi:old".into()))
                .is_err()
        );
        settings
            .select(MicrophoneChoice::Device("wasapi:new".into()))
            .unwrap();
        assert_eq!(
            settings.effective(),
            &MicrophoneChoice::Device("wasapi:new".into())
        );
    }

    #[test]
    fn enumeration_failure_is_not_mistaken_for_missing_preferred_device() {
        let preferred = MicrophoneChoice::Device("wasapi:one".into());
        let mut settings = MicrophoneSettings::new(Box::new(FailingPort), preferred.clone());
        assert!(settings.list_failed());
        assert_eq!(settings.effective(), &preferred);
        assert!(
            settings
                .take_notice()
                .unwrap()
                .contains("Liste des microphones")
        );
    }
}
