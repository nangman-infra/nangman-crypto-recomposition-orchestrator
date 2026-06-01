use std::path::Path;

use crate::types::OrchestratorReport;

use super::json_payload::{jsonl_bytes, read_json_array_or_jsonl_bytes};
use super::validation::{safe_prefixed_key, validate_output_key};
use super::write::write_outputs_to_dir;

fn report(report_id: &str) -> OrchestratorReport {
    OrchestratorReport {
        orchestrator_report_id: report_id.to_owned(),
        schema_version: crate::REPORT_SCHEMA_VERSION.to_owned(),
        producer_app: crate::PRODUCER_APP.to_owned(),
        producer_version: "0.1.0".to_owned(),
        created_at_ms: 7_200_000,
        input_hypothesis_count: 0,
        input_s3_keys_read: 0,
        selected_job_count: 0,
        skipped_count: 0,
        changed_triggers: vec!["market_feature_delta_updated".to_owned()],
        output_job_key: String::new(),
        checksum: String::new(),
    }
}

#[test]
fn local_output_rejects_traversal_report_id() {
    let output_dir = std::env::temp_dir().join(format!(
        "recomposition-output-traversal-test-{}",
        std::process::id()
    ));
    let error = write_outputs_to_dir(&output_dir, 7_200_000, &[], &report("../escape"))
        .unwrap_err()
        .to_string();

    assert!(error.contains("orchestrator report id"));
    assert!(!output_dir.join("escape").exists());
}

#[test]
fn local_output_requires_absolute_output_dir() {
    let error = write_outputs_to_dir(
        Path::new("relative-output"),
        7_200_000,
        &[],
        &report("recomp_report_001"),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("absolute path"));
}

#[test]
fn local_output_rejects_ambiguous_absolute_output_dir() {
    let error = write_outputs_to_dir(
        Path::new("/tmp/../recomposition-output"),
        7_200_000,
        &[],
        &report("recomp_report_001"),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("relative path components"));
}

#[cfg(unix)]
#[test]
fn local_output_rejects_symlink_destination() {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let output_dir = std::env::temp_dir().join(format!(
        "recomposition-output-symlink-test-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&output_dir).expect("create temp output dir");
    let target_path = output_dir.join("outside-target.json");
    fs::write(&target_path, "do-not-overwrite").expect("write symlink target");
    let report_path = output_dir
        .join("recomposition-orchestrator-report/schema=recomposition_orchestrator_report_v1/dt=1970-01-01/hour=02/orchestrator_report_id=recomp_report_001/report.json");
    fs::create_dir_all(report_path.parent().expect("report path parent exists"))
        .expect("create report parent");
    symlink(&target_path, &report_path).expect("create report output symlink");

    let error = write_outputs_to_dir(&output_dir, 7_200_000, &[], &report("recomp_report_001"))
        .expect_err("symlink output destination must be rejected")
        .to_string();

    assert!(error.contains("symlink"));
    assert_eq!(
        fs::read_to_string(&target_path).expect("read symlink target"),
        "do-not-overwrite"
    );
    fs::remove_dir_all(&output_dir).ok();
}

#[test]
fn output_key_validation_rejects_escape_shapes() {
    for key in [
        "/tmp/out.json",
        "hypothesis-harness-job/../report.json",
        "hypothesis-harness-job/./report.json",
        "hypothesis-harness-job\\report.json",
        "hypothesis-harness-job/\n/report.json",
    ] {
        let error = validate_output_key(key).unwrap_err().to_string();
        assert!(
            error.contains("output key"),
            "expected output key error for {key:?}, got {error}"
        );
    }
}

#[test]
fn s3_prefix_rejects_traversal() {
    let error = safe_prefixed_key("../escape", "schema=v1/report.json")
        .unwrap_err()
        .to_string();

    assert!(error.contains("output key"));
}

#[test]
fn s3_prefix_preserves_normal_contract() {
    assert_eq!(
        safe_prefixed_key(
            "/recomposition/",
            "hypothesis-harness-job/schema=hypothesis_harness_job_v1/part-000001.jsonl",
        )
        .unwrap(),
        "recomposition/hypothesis-harness-job/schema=hypothesis_harness_job_v1/part-000001.jsonl"
    );
}

#[test]
fn json_payload_reads_array_single_object_and_jsonl_inputs() {
    let array = br#"[{"value":1},{"value":2}]"#;
    let object = br#"{"value":3}"#;
    let jsonl = b"{\"value\":4}\n\n{\"value\":5}\n";

    assert_eq!(
        read_json_array_or_jsonl_bytes::<serde_json::Value>("array", array).unwrap(),
        vec![
            serde_json::json!({"value": 1}),
            serde_json::json!({"value": 2})
        ]
    );
    assert_eq!(
        read_json_array_or_jsonl_bytes::<serde_json::Value>("object", object).unwrap(),
        vec![serde_json::json!({"value": 3})]
    );
    assert_eq!(
        read_json_array_or_jsonl_bytes::<serde_json::Value>("jsonl", jsonl).unwrap(),
        vec![
            serde_json::json!({"value": 4}),
            serde_json::json!({"value": 5})
        ]
    );
}

#[test]
fn json_payload_reports_empty_utf8_and_line_errors() {
    let empty_error = read_json_array_or_jsonl_bytes::<serde_json::Value>("empty", b"  ")
        .unwrap_err()
        .to_string();
    assert!(empty_error.contains("must not be empty"));

    let utf8_error = read_json_array_or_jsonl_bytes::<serde_json::Value>("bad-utf8", &[0xff])
        .unwrap_err()
        .to_string();
    assert!(utf8_error.contains("bad-utf8"));

    let line_error = read_json_array_or_jsonl_bytes::<serde_json::Value>(
        "bad-jsonl",
        b"{\"value\":1}\nnot-json\n",
    )
    .unwrap_err()
    .to_string();
    assert!(line_error.contains("bad-jsonl line 2"));
}

#[test]
fn jsonl_bytes_writes_one_json_record_per_line() {
    let bytes = jsonl_bytes(&[
        serde_json::json!({"value": 1}),
        serde_json::json!({"value": 2}),
    ])
    .unwrap();

    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "{\"value\":1}\n{\"value\":2}\n"
    );
}
