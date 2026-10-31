use super::*;

fn paint_microphone_spectrum(
    ui: &mut egui::Ui,
    title: &str,
    bins: &[whisper_core::microphone_level::SpectrumBin],
    sample_rate_hz: u32,
) {
    use whisper_core::microphone_level::SpectrumBin;
    ui.label(title);
    if bins.iter().all(|bin| matches!(bin, SpectrumBin::Silence)) {
        ui.label("Silence détecté");
    }
    let width = ui.available_width().max(180.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 116.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, egui::Color32::from_gray(18));
    let left = rect.left() + 34.0;
    let right = rect.right() - 8.0;
    let top = rect.top() + 8.0;
    let bottom = rect.bottom() - 18.0;
    for db in [-120.0, -60.0, 0.0] {
        let y = bottom - (db + 120.0) / 120.0 * (bottom - top);
        painter.line_segment(
            [egui::pos2(left, y), egui::pos2(right, y)],
            egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
        );
        painter.text(
            egui::pos2(rect.left() + 1.0, y),
            egui::Align2::LEFT_CENTER,
            format!("{db:.0}"),
            egui::FontId::monospace(9.0),
            egui::Color32::GRAY,
        );
    }
    let mut prior = None;
    let mut out_of_scale = false;
    for (index, bin) in bins.iter().enumerate() {
        let SpectrumBin::MilliDb(milli_db) = bin else {
            prior = None;
            continue;
        };
        let db = f64::from(*milli_db) / 1000.0;
        if !(-120.0..=0.0).contains(&db) {
            out_of_scale = true;
        }
        let x = left + (index as f32 / bins.len().saturating_sub(1).max(1) as f32) * (right - left);
        let y = bottom - (((db + 120.0) / 120.0).clamp(0.0, 1.0) as f32) * (bottom - top);
        let point = egui::pos2(x, y);
        if let Some(previous) = prior {
            painter.line_segment(
                [previous, point],
                egui::Stroke::new(1.25, egui::Color32::LIGHT_GREEN),
            );
        }
        prior = Some(point);
    }
    let nyquist = sample_rate_hz / 2;
    painter.text(
        egui::pos2(left, rect.bottom() - 3.0),
        egui::Align2::LEFT_BOTTOM,
        "0 Hz",
        egui::FontId::monospace(9.0),
        egui::Color32::GRAY,
    );
    painter.text(
        egui::pos2(right, rect.bottom() - 3.0),
        egui::Align2::RIGHT_BOTTOM,
        format!("{nyquist} Hz"),
        egui::FontId::monospace(9.0),
        egui::Color32::GRAY,
    );
    if out_of_scale {
        ui.small("Valeurs hors de l’échelle verticale affichées au bord du graphe");
    }
}

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn microphone_settings_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        ui.group(|ui| {
            self.microphone_source_ui(ui, busy);
            self.microphone_test_ui(ui);
            self.microphone_meter_ui(ui);
            self.microphone_gain_ui(ui);
        });
    }

    fn microphone_source_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        ui.heading("Microphone");
        ui.add_enabled_ui(!busy, |ui| {
                if ui.button("Actualiser les microphones").clicked() {
                    self.refresh_microphones();
                }
                ui.horizontal(|ui| {
                    ui.label("Source microphone");
                    let mut choice = self.microphones.preferred().clone();
                    let mut choice_requested = false;
                    let selected_text = microphone_choice_label(
                        self.microphones.effective(),
                        self.microphones.devices(),
                    );
                    egui::ComboBox::from_id_salt("microphone_choice")
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            choice_requested |= ui
                                .selectable_value(
                                    &mut choice,
                                    MicrophoneChoice::WindowsDefault,
                                    "Microphone par défaut de Windows",
                                )
                                .clicked();
                            for device in self.microphones.devices() {
                                ui.add_enabled_ui(device.selectable(), |ui| {
                                    choice_requested |= ui
                                        .selectable_value(
                                            &mut choice,
                                            MicrophoneChoice::Device(device.id.clone()),
                                            microphone_device_label(
                                                device,
                                                self.microphones.devices(),
                                            ),
                                        )
                                        .clicked();
                                });
                            }
                        });
                    if choice_requested {
                        match self.microphones.select(choice) {
                            Ok(()) => {
                                self.paths
                                    .set_microphone_preference(self.microphones.preferred());
                                self.microphone_notice = None;
                            }
                            Err(error) => self.microphone_notice = Some(error),
                        }
                    }
                });
                if let MicrophoneChoice::Device(_) = self.microphones.preferred()
                    && self.microphones.effective() == &MicrophoneChoice::WindowsDefault
                {
                    ui.label(
                        "Source préférée indisponible. Le microphone par défaut de Windows est utilisé.",
                    );
                }
                if self.microphones.devices().is_empty() && !self.microphones.list_failed() {
                    ui.label("Aucun microphone d'entrée disponible.");
                }
                for device in self.microphones.devices() {
                    if let Some(reason) = &device.unavailable_reason {
                        ui.label(format!("{} : {reason}", device.name));
                    }
                }
            });
    }

    fn microphone_test_ui(&mut self, ui: &mut egui::Ui) {
        let identified_target = self.microphones.target_identity().is_some();
        let button = if self.view.microphone_test_active {
            let response = ui.button("Arrêter le test microphone");
            if response.clicked() {
                self.stop_microphone_test();
            }
            response
        } else {
            let response = ui.add_enabled(
                microphone_test_start_enabled(identified_target, &self.view),
                egui::Button::new("Tester le microphone"),
            );
            if response.clicked() {
                self.start_microphone_test();
            }
            response
        };
        #[cfg(test)]
        {
            self.microphone_test_button_rect = Some(button.rect);
        }
        #[cfg(not(test))]
        let _ = button;

        if let Some(actual) = self.view.microphone_source_identity.as_deref() {
            let line =
                MicrophoneLabelProjection::new(self.microphones.devices()).actual_line(actual);
            ui.label(line);
        } else if let Some(expected) = self.microphones.target_identity() {
            let line =
                MicrophoneLabelProjection::new(self.microphones.devices()).planned_line(expected);
            ui.label(line);
        }
    }

    fn microphone_meter_ui(&mut self, ui: &mut egui::Ui) {
        let meter_state = microphone_meter_presentation(
            self.view.microphone_meter.as_ref(),
            self.active_identity,
            whisper_core::microphone_level::monotonic_millis(),
        );
        let saturated = microphone_saturation_is_current(
            self.view.microphone_meter.as_ref(),
            self.active_identity,
            self.view.microphone_source_identity.as_deref(),
            self.view.microphone_test_active
                || self.view.direct_capture_phase.is_some_and(|phase| {
                    matches!(
                        phase,
                        whisper_core::application::DirectCapturePhase::Initializing
                            | whisper_core::application::DirectCapturePhase::Active
                    )
                }),
            whisper_core::microphone_level::monotonic_millis(),
        );
        #[cfg(test)]
        {
            self.microphone_saturation_badge_rect = None;
        }
        match meter_state {
            MicrophoneMeterPresentation::Awaiting if self.view.microphone_test_active => {
                ui.label("En attente de mesures microphone…");
            }
            MicrophoneMeterPresentation::Awaiting => {}
            MicrophoneMeterPresentation::Stale => {
                ui.colored_label(egui::Color32::YELLOW, "Jauge périmée");
            }
            MicrophoneMeterPresentation::WrongCapture => {
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "Mesure d'une ancienne capture ignorée",
                );
            }
            MicrophoneMeterPresentation::Absent => {
                ui.label("Mesure microphone indisponible");
            }
            MicrophoneMeterPresentation::Silence => {
                ui.label("Silence détecté");
            }
            MicrophoneMeterPresentation::Signal {
                rms_dbfs,
                peak_dbfs,
            } => {
                ui.label(format!(
                    "Niveau micro — RMS {rms_dbfs:.1} dBFS, pic {peak_dbfs:.1} dBFS"
                ));
            }
        }
        if saturated {
            let _response =
                ui.colored_label(MICROPHONE_SATURATION_COLOR, MICROPHONE_SATURATION_LABEL);
            #[cfg(test)]
            {
                self.microphone_saturation_badge_rect = Some(_response.rect);
            }
        }
        if self.view.microphone_acquisition.is_some() {
            ui.separator();
            ui.label("Spectre en direct — échelle commune de −120 à 0 dBFS/Hz");
            if let Some(spectrum) = self.view.microphone_spectrum.as_ref() {
                let age = whisper_core::microphone_level::monotonic_millis()
                    .saturating_sub(spectrum.captured_at_monotonic_ms);
                if age <= 500 {
                    paint_microphone_spectrum(
                        ui,
                        "Avant gain",
                        &spectrum.pre_gain,
                        spectrum.sample_rate_hz,
                    );
                    paint_microphone_spectrum(
                        ui,
                        "Après gain et limiteur",
                        &spectrum.post_gain_limiter,
                        spectrum.sample_rate_hz,
                    );
                    if spectrum.mixed_gain {
                        ui.label(format!(
                            "Fenêtre FFT avec gain variable : {} → {} dB",
                            spectrum.gain_first_db, spectrum.gain_last_db
                        ));
                    }
                } else {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        "Spectre périmé ; attente d’une nouvelle fenêtre audio",
                    );
                }
            } else {
                ui.label("En attente de 2048 trames audio pour calculer le spectre…");
            }
        }
    }

    fn microphone_gain_ui(&mut self, ui: &mut egui::Ui) {
        let unified_capture_active = self.active_unified_capture_identity().is_some();
        let gain_identity = if unified_capture_active {
            self.view.microphone_source_identity.as_deref()
        } else {
            self.view
                .microphone_source_identity
                .as_deref()
                .or_else(|| self.microphones.target_identity())
        };
        if let Some(gain_identity) = gain_identity
            && let Ok(identity) =
                whisper_core::microphone_level::MicrophoneIdentity::new(gain_identity)
        {
            let enabled = (self.view.microphone_test_active
                || self.view.direct_capture_phase.is_some()
                || unified_capture_active
                || self.view.active_job.is_none())
                && (self.view.microphone_source_identity.is_some()
                    || (!unified_capture_active && self.microphones.target_identity().is_some()));
            let mut gain = if self.view.microphone_test_active
                || self.view.direct_capture_phase.is_some()
                || unified_capture_active
            {
                self.view.microphone_gain_requested_db
            } else {
                self.paths.microphone_gain(&identity).value()
            };
            let response = ui.add_enabled(
                enabled,
                egui::Slider::new(&mut gain, -24..=24).text("Gain microphone (dB)"),
            );
            #[cfg(test)]
            {
                self.microphone_gain_slider_enabled = Some(response.enabled());
            }
            if response.changed() {
                self.set_microphone_gain(gain);
            }
        } else {
            ui.label("Gain microphone indisponible : cible non identifiée.");
        }
    }
}
