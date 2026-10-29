use super::*;

#[test]
fn failed_commit_remains_committing_even_after_stop_rollback() {
    let gate = PublicationGate::default();
    gate.activate();
    let result: Result<(), String> = gate.commit(|| Err("storage failure".into()));
    assert_eq!(result, Err("storage failure".into()));
    assert_eq!(gate.0.load(Ordering::Acquire), GATE_COMMITTING);
    assert_eq!(gate.request_stop(), Err(PortError::Committing));
    gate.rollback_stop();
    assert_eq!(gate.0.load(Ordering::Acquire), GATE_COMMITTING);
    assert_eq!(
        gate.commit(|| Ok(())),
        Err("Publication: commit is already in progress".into())
    );
}

#[test]
fn rollback_only_restores_a_cancelled_gate() {
    let gate = PublicationGate::default();
    gate.activate();
    assert_eq!(gate.request_stop(), Ok(true));
    assert!(gate.is_cancelled());
    gate.rollback_stop();
    assert!(!gate.is_cancelled());
    assert_eq!(gate.commit(|| Ok(())), Ok(()));
    assert_eq!(gate.0.load(Ordering::Acquire), GATE_COMMITTED);
}
