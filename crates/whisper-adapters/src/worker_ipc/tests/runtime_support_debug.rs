use super::*;

#[test]
fn native_qualification_debug_retains_spawn_and_result_fields() {
    let spawn = NativeQualificationEventKind::Spawned {
        incarnation: 2,
        pid: 42,
    };
    assert_eq!(format!("{spawn:?}"), "Spawned { incarnation: 2, pid: 42 }");

    let result = NativeQualificationEventKind::Result {
        incarnation: 2,
        pid: 42,
        session: whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: JobId(7),
            generation: Generation::first(),
        },
        generation: Generation::first(),
        attempt: 1,
        request_id: 3,
        work: NativeQualificationWork::Final(1),
        word_count: 4,
    };
    let trace = format!("{result:?}");
    assert!(trace.starts_with("Result { incarnation: 2, pid: 42, session:"));
    assert!(trace.ends_with("work: Final(1), word_count: 4 }"));
}
