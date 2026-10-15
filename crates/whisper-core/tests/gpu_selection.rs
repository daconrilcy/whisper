use whisper_core::{
    domain::ComputeChoice,
    gpu_inventory::{
        GpuAvailability, GpuCatalogueCoverage, GpuIdentityMatch, GpuObservation, GpuPresence,
        GpuQualifiedObservation, GpuRankSource, GpuState, GpuSupport, GpuWindowsRank, qualify,
    },
    gpu_inventory_app::GpuQualifiedInventoryState,
    gpu_selection::{
        AttestedBackend, EffectiveEngine, GpuSelectionEvent, SelectionReason, SelectionResolution,
        SessionCandidate, attest, attestation_event, candidate_event, resolve,
    },
};

fn row(
    identity: &str,
    name: &str,
    rank: Option<u32>,
    support: GpuSupport,
) -> GpuQualifiedObservation {
    GpuQualifiedObservation {
        observation: GpuObservation {
            identity: identity.into(),
            name: Some(name.into()),
            state: Some(GpuState::Ready),
            presence: GpuPresence::Current,
        },
        windows_rank: rank.map(|ordinal| GpuWindowsRank {
            ordinal,
            source: GpuRankSource::DxgiEnumeration,
        }),
        identity_match: GpuIdentityMatch::Unique,
        support,
        availability: GpuAvailability::Available,
        causes: vec![],
    }
}

fn complete(rows: Vec<GpuQualifiedObservation>) -> GpuQualifiedInventoryState {
    let catalogue = qualify(rows, None);
    assert_eq!(catalogue.coverage, GpuCatalogueCoverage::Complete);
    GpuQualifiedInventoryState::Complete(catalogue)
}

#[test]
fn default_uses_windows_rank_even_when_motor_order_and_names_are_reversed() {
    let catalogue = complete(vec![
        row(
            "windows-b",
            "Same name",
            Some(1),
            GpuSupport::ConfirmedSupported,
        ),
        row(
            "windows-c",
            "Same name",
            Some(2),
            GpuSupport::ConfirmedSupported,
        ),
        row(
            "windows-a",
            "Unsupported",
            Some(0),
            GpuSupport::ConfirmedUnsupported,
        ),
    ]);
    assert_eq!(
        resolve(ComputeChoice::Gpu, None, &catalogue),
        SelectionResolution::Candidate(SessionCandidate::Gpu(
            whisper_core::gpu_selection::GpuCandidate {
                identity: "windows-b".into(),
                name: "Same name".into(),
                preference_substituted: false,
            }
        ))
    );
}

#[test]
fn preferred_compatible_device_wins_without_rank_and_missing_rank_never_becomes_default() {
    let preferred = complete(vec![row(
        "preferred",
        "Preferred",
        None,
        GpuSupport::ConfirmedSupported,
    )]);
    assert!(
        matches!(resolve(ComputeChoice::Gpu, Some("preferred"), &preferred), SelectionResolution::Candidate(SessionCandidate::Gpu(candidate)) if candidate.identity == "preferred")
    );
    assert_eq!(
        resolve(ComputeChoice::Gpu, None, &preferred),
        SelectionResolution::Waiting(SelectionReason::WindowsOrderUnproven)
    );
}

#[test]
fn unavailable_preference_is_not_overwritten_and_substitution_uses_first_ranked_device() {
    let mut preferred = row(
        "preferred",
        "Preferred",
        Some(0),
        GpuSupport::ConfirmedSupported,
    );
    preferred.availability = GpuAvailability::Unavailable;
    let catalogue = complete(vec![
        row("second", "Second", Some(2), GpuSupport::ConfirmedSupported),
        preferred,
        row("first", "First", Some(1), GpuSupport::ConfirmedSupported),
    ]);
    assert!(
        matches!(resolve(ComputeChoice::Gpu, Some("preferred"), &catalogue), SelectionResolution::Candidate(SessionCandidate::Gpu(candidate)) if candidate.identity == "first" && candidate.preference_substituted)
    );
}

#[test]
fn incomplete_or_ambiguous_catalogue_waits_instead_of_claiming_no_gpu() {
    assert_eq!(
        resolve(
            ComputeChoice::Auto,
            None,
            &GpuQualifiedInventoryState::Loading
        ),
        SelectionResolution::Waiting(SelectionReason::InventoryNotReady)
    );
    assert_eq!(
        resolve(
            ComputeChoice::Auto,
            None,
            &GpuQualifiedInventoryState::Failed("scan failed".into())
        ),
        SelectionResolution::Waiting(SelectionReason::InventoryIncomplete)
    );
    let mut uncertain = row("maybe", "Maybe", Some(0), GpuSupport::Undetermined);
    uncertain.identity_match = GpuIdentityMatch::Ambiguous;
    let catalogue = complete(vec![uncertain]);
    assert_eq!(
        resolve(ComputeChoice::Auto, None, &catalogue),
        SelectionResolution::Waiting(SelectionReason::QualificationUncertain)
    );
    let partial_catalogue = qualify(
        vec![row(
            "known",
            "Known",
            Some(0),
            GpuSupport::ConfirmedSupported,
        )],
        Some("more devices may exist".into()),
    );
    assert_eq!(partial_catalogue.coverage, GpuCatalogueCoverage::Partial);
    assert_eq!(
        resolve(
            ComputeChoice::Gpu,
            None,
            &GpuQualifiedInventoryState::Partial(partial_catalogue)
        ),
        SelectionResolution::Waiting(SelectionReason::WindowsOrderUnproven)
    );
}

#[test]
fn only_confirmed_identity_support_and_availability_make_a_gpu_selectable() {
    let mut unmatched = row("c", "C", Some(2), GpuSupport::ConfirmedSupported);
    unmatched.identity_match = GpuIdentityMatch::Missing;
    let mut disabled = row("d", "D", Some(3), GpuSupport::Undetermined);
    disabled.observation.state = Some(GpuState::Disabled);
    disabled.availability = GpuAvailability::NotVerified;
    let mut failed_temporarily = row("e", "E", Some(4), GpuSupport::ConfirmedSupported);
    failed_temporarily.availability = GpuAvailability::Unavailable;
    let catalogue = complete(vec![
        row("a", "A", Some(0), GpuSupport::ConfirmedSupported),
        row("b", "B", Some(1), GpuSupport::ConfirmedUnsupported),
        unmatched,
        disabled,
        failed_temporarily,
    ]);
    assert!(matches!(
        resolve(ComputeChoice::Gpu, None, &catalogue),
        SelectionResolution::Candidate(SessionCandidate::Gpu(candidate)) if candidate.identity == "a"
    ));
    let GpuQualifiedInventoryState::Complete(catalogue) = catalogue else {
        unreachable!()
    };
    assert_eq!(
        catalogue.devices[1].support,
        GpuSupport::ConfirmedUnsupported
    );
    assert_eq!(
        catalogue.devices[2].identity_match,
        GpuIdentityMatch::Missing
    );
    assert_eq!(catalogue.devices[3].device.state, GpuState::Disabled);
    assert_eq!(catalogue.devices[3].support, GpuSupport::Undetermined);
    assert_eq!(
        catalogue.devices[4].availability,
        GpuAvailability::Unavailable
    );
}

#[test]
fn mode_and_preference_matrix_keeps_candidate_separate_from_effective_engine() {
    let supported = complete(vec![row(
        "gpu-1",
        "GPU",
        Some(0),
        GpuSupport::ConfirmedSupported,
    )]);
    let mut unsupported_row = row(
        "gpu-x",
        "No CUDA",
        Some(0),
        GpuSupport::ConfirmedUnsupported,
    );
    unsupported_row.availability = GpuAvailability::NotVerified;
    let unsupported = complete(vec![unsupported_row]);
    assert_eq!(
        resolve(ComputeChoice::Cpu, Some("gpu-1"), &supported),
        SelectionResolution::Candidate(SessionCandidate::Cpu)
    );
    assert!(matches!(
        resolve(ComputeChoice::Gpu, None, &unsupported),
        SelectionResolution::Refused(SelectionReason::NoCompatibleGpu)
    ));
    assert_eq!(
        resolve(ComputeChoice::Auto, None, &unsupported),
        SelectionResolution::CpuFallback {
            reason: SelectionReason::NoCompatibleGpu
        }
    );
    assert_eq!(
        resolve(ComputeChoice::Auto, Some("gpu-x"), &unsupported),
        SelectionResolution::CpuFallback {
            reason: SelectionReason::PreferredGpuUnavailable
        }
    );
    assert_eq!(
        resolve(ComputeChoice::Cpu, Some("missing"), &unsupported),
        SelectionResolution::Candidate(SessionCandidate::Cpu)
    );
    let candidate = SessionCandidate::Gpu(whisper_core::gpu_selection::GpuCandidate {
        identity: "gpu-1".into(),
        name: "GPU".into(),
        preference_substituted: false,
    });
    assert_eq!(
        attest(42, &candidate, AttestedBackend::Gpu("gpu-1")),
        Ok(EffectiveEngine::GpuAttested {
            session_id: 42,
            identity: "gpu-1".into()
        })
    );
    assert_eq!(
        attest(42, &candidate, AttestedBackend::Gpu("gpu-other")),
        Err(SelectionReason::AttestationMismatch)
    );
    assert!(matches!(
        candidate_event(42, candidate.clone()),
        GpuSelectionEvent::CandidateResolved { session_id: 42, .. }
    ));
    assert_eq!(
        attestation_event(42, &candidate, AttestedBackend::Gpu("gpu-1")),
        Ok(GpuSelectionEvent::EffectiveEngineAttested {
            engine: EffectiveEngine::GpuAttested {
                session_id: 42,
                identity: "gpu-1".into()
            }
        })
    );
}

#[test]
fn disabled_and_empty_identity_devices_are_never_gpu_candidates() {
    let mut disabled = row(
        "disabled-id",
        "Disabled",
        Some(0),
        GpuSupport::ConfirmedSupported,
    );
    disabled.observation.state = Some(GpuState::Disabled);
    let empty_identity = row("", "No identity", Some(1), GpuSupport::ConfirmedSupported);
    for catalogue in [complete(vec![disabled]), complete(vec![empty_identity])] {
        assert_eq!(
            resolve(ComputeChoice::Gpu, None, &catalogue),
            SelectionResolution::Refused(SelectionReason::NoCompatibleGpu)
        );
    }
}

#[test]
fn duplicate_windows_rank_cannot_choose_an_arbitrary_first_device() {
    let catalogue = complete(vec![
        row("a", "A", Some(0), GpuSupport::ConfirmedSupported),
        row("b", "B", Some(0), GpuSupport::ConfirmedSupported),
    ]);
    assert_eq!(
        resolve(ComputeChoice::Gpu, None, &catalogue),
        SelectionResolution::Waiting(SelectionReason::WindowsOrderUnproven)
    );
}
