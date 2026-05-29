use crate::ids::{checksum_json, stable_id};
use crate::io::harness_job_key;
use crate::types::{HarnessJob, OrchestratorReport};
use crate::{JOB_SCHEMA_VERSION, PRODUCER_APP, REPORT_SCHEMA_VERSION};
use intel_candidate_app::error::AppResult;
use intel_candidate_app::model::IntelCandidateHypothesisState;
use std::collections::BTreeSet;

pub(crate) fn build_job_if_dirty(
    state: &IntelCandidateHypothesisState,
    changed: &BTreeSet<String>,
    created_at_ms: i64,
) -> AppResult<Option<HarnessJob>> {
    let matched = state
        .dirty_triggers
        .iter()
        .filter(|trigger| changed.contains(*trigger))
        .cloned()
        .collect::<Vec<_>>();
    if matched.is_empty() {
        return Ok(None);
    }
    let priority = if matched.iter().any(|trigger| {
        trigger == "candidate_app_version_changed" || trigger == "scoring_policy_version_changed"
    }) {
        "p0_recompute".to_owned()
    } else if matched.iter().any(|trigger| {
        trigger == "market_feature_delta_updated" || trigger == "derivatives_rollup_updated"
    }) {
        "p1_market_retest".to_owned()
    } else {
        "p2_observe_refresh".to_owned()
    };
    let mut job = HarnessJob {
        harness_job_id: stable_id(
            "harness_job",
            &[
                &state.hypothesis_id,
                &state.state_key,
                &state.updated_at_ms.to_string(),
                &created_at_ms.to_string(),
                &matched.join(","),
            ],
        ),
        schema_version: JOB_SCHEMA_VERSION.to_owned(),
        producer_app: PRODUCER_APP.to_owned(),
        producer_version: env!("CARGO_PKG_VERSION").to_owned(),
        created_at_ms,
        hypothesis_id: state.hypothesis_id.clone(),
        hypothesis_state_key: state.state_key.clone(),
        latest_screening_event_id: state.latest_screening_event_id.clone(),
        requested_harness_type: state.harness_queue_hint.clone(),
        changed_triggers: changed.iter().cloned().collect(),
        matched_dirty_triggers: matched,
        input_refs: state.lineage_refs.clone(),
        priority,
        idempotency_key: stable_id(
            "harness_job_idem",
            &[&state.hypothesis_id, &state.updated_at_ms.to_string()],
        ),
        checksum: String::new(),
    };
    job.checksum = checksum_json(&job)?;
    Ok(Some(job))
}

pub(crate) fn build_report(
    created_at_ms: i64,
    input_count: usize,
    input_s3_keys_read: usize,
    jobs: &[HarnessJob],
    changed: &BTreeSet<String>,
) -> AppResult<OrchestratorReport> {
    let report_id = stable_id(
        "recomp_report",
        &[&created_at_ms.to_string(), &input_count.to_string()],
    );
    let mut report = OrchestratorReport {
        orchestrator_report_id: report_id.clone(),
        schema_version: REPORT_SCHEMA_VERSION.to_owned(),
        producer_app: PRODUCER_APP.to_owned(),
        producer_version: env!("CARGO_PKG_VERSION").to_owned(),
        created_at_ms,
        input_hypothesis_count: input_count,
        input_s3_keys_read,
        selected_job_count: jobs.len(),
        skipped_count: input_count.saturating_sub(jobs.len()),
        changed_triggers: changed.iter().cloned().collect(),
        output_job_key: harness_job_key(created_at_ms, &report_id),
        checksum: String::new(),
    };
    report.checksum = checksum_json(&report)?;
    Ok(report)
}
