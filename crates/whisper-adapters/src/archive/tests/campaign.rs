    #[cfg(feature = "recette-hooks")]
    #[test]
    fn l04_campaign_gate_rejects_escape_and_duplicate_trigger() {
        if option_env!("WHISPER_L04_CAMPAIGN_BUILD").is_none() {
            assert!(campaign_gate("coverage.after_sync", 1, 1, 0).is_ok());
            return;
        }
        if let Ok(mode) = std::env::var("WHISPER_L04_GATE_TEST_CHILD") {
            if mode == "outside" {
                assert!(
                    campaign_gate("coverage.after_sync", 1, 1, 0)
                        .unwrap_err()
                        .contains("direct child")
                );
            } else if mode == "zero" {
                assert!(campaign_gate("recovery.before_scan_live", 1, 0, 0).is_ok());
            } else {
                let control_path = std::env::var_os("WHISPER_L04_CAMPAIGN_RUN")
                    .map(PathBuf::from)
                    .unwrap()
                    .join("run.json");
                let original_control = fs::read(&control_path).unwrap();
                let archive = control_path.parent().unwrap().join("archive");
                assert!(assert_campaign_archive(&archive).is_ok());
                assert!(assert_campaign_archive(control_path.parent().unwrap()).is_err());
                fs::write(
                    &control_path,
                    br#"{"schema":"whisper-l04-fault/1","hook":"coverage.after_sync","job_id":"1","generation":1,"instance_id":0,"binary":{"sha256":"invalid"}}"#,
                )
                .unwrap();
                assert!(
                    campaign_gate("coverage.before_record", 1, 1, 0)
                        .unwrap_err()
                        .contains("missing trigger count")
                );
                fs::write(
                    &control_path,
                    br#"{"schema":"whisper-l04-fault/1","hook":"coverage.after_sync","job_id":"1","generation":1,"instance_id":0,"trigger_count":1,"binary":{"sha256":"invalid"}}"#,
                )
                .unwrap();
                assert!(
                    campaign_gate("coverage.before_record", 1, 1, 0)
                        .unwrap_err()
                        .contains("binary hash mismatch")
                );
                fs::write(&control_path, original_control).unwrap();
                assert!(campaign_gate("coverage.before_record", 1, 1, 0).is_ok());
                assert!(campaign_gate("coverage.after_sync", 1, 1, 0).is_ok());
                assert!(
                    campaign_gate("coverage.after_sync", 1, 1, 0)
                        .unwrap_err()
                        .contains("duplicate trigger")
                );
                assert!(
                    campaign_gate("coverage.after_sync", 2, 1, 0)
                        .unwrap_err()
                        .contains("identity mismatch")
                );
                fs::write(
                    std::env::var_os("WHISPER_L04_CAMPAIGN_RUN")
                        .map(PathBuf::from)
                        .unwrap()
                        .join("run.json"),
                    br#"{"schema":"whisper-l04-fault/1","hook":"unknown","job_id":"1","generation":1,"instance_id":0}"#,
                )
                .unwrap();
                assert!(
                    campaign_gate("coverage.after_sync", 1, 1, 0)
                        .unwrap_err()
                        .contains("unknown hook")
                );
            }
            return;
        }
        let runs = Path::new(r"C:\WhisperLive\L04-campaign\runs");
        fs::create_dir_all(runs).unwrap();
        let binary = std::env::current_exe().unwrap();
        let binary_sha256 = format!("{:x}", Sha256::digest(fs::read(&binary).unwrap()));
        let test_name = "archive::tests::l04_campaign_gate_rejects_escape_and_duplicate_trigger";
        let outside = std::process::Command::new(&binary)
            .args(["--exact", test_name])
            .env("WHISPER_L04_GATE_TEST_CHILD", "outside")
            .env("WHISPER_L04_CAMPAIGN_RUN", std::env::temp_dir())
            .status()
            .unwrap();
        assert!(outside.success());
        let run = runs.join(format!(
            "gate-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&run).unwrap();
        fs::create_dir(run.join("archive")).unwrap();
        fs::write(
            run.join("run.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema":"whisper-l04-fault/1", "hook":"coverage.after_sync",
                "job_id":"1", "generation":1, "instance_id":0, "trigger_count":1,
                "binary":{"sha256":binary_sha256},
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(run.join("decision.txt"), b"release\n").unwrap();
        let inside = std::process::Command::new(binary)
            .args(["--exact", test_name])
            .env("WHISPER_L04_GATE_TEST_CHILD", "inside")
            .env("WHISPER_L04_CAMPAIGN_RUN", &run)
            .status()
            .unwrap();
        assert!(inside.success());
        let marker: serde_json::Value =
            serde_json::from_slice(&fs::read(run.join("entered.json")).unwrap()).unwrap();
        assert_eq!(marker["binary_sha256"], binary_sha256);
        assert_eq!(marker["preceding_operation_result"], "returned_ok");
        assert!(marker["entered_unix_ms"].as_u64().is_some());
        let receipt: serde_json::Value =
            serde_json::from_slice(&fs::read(run.join("decision-observed.json")).unwrap()).unwrap();
        assert_eq!(receipt["action"], "release");
        assert!(receipt["observed_unix_ms"].as_u64().is_some());
        fs::remove_dir_all(run).unwrap();
        let zero = runs.join(format!("gate-zero-{}", std::process::id()));
        fs::create_dir(&zero).unwrap();
        fs::write(
            zero.join("run.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema":"whisper-l04-fault/1", "hook":"recovery.before_scan_live",
                "job_id":"1", "generation":0, "instance_id":0, "trigger_count":1,
                "binary":{"sha256":binary_sha256},
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(zero.join("decision.txt"), b"release\n").unwrap();
        let zero_result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test_name])
            .env("WHISPER_L04_GATE_TEST_CHILD", "zero")
            .env("WHISPER_L04_CAMPAIGN_RUN", &zero)
            .status()
            .unwrap();
        assert!(zero_result.success());
        fs::remove_dir_all(zero).unwrap();
    }
