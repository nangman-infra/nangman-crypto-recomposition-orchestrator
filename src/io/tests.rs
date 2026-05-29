use std::path::Path;

use crate::types::OrchestratorReport;

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
