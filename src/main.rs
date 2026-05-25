use chrono::{DateTime, Datelike, Timelike, Utc};
use intel_candidate_app::error::{AppError, AppResult};
use intel_candidate_app::model::IntelCandidateHypothesisState;
use intel_candidate_app::storage::{ObjectStore, ObjectStoreConfig};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

const JOB_SCHEMA_VERSION: &str = "hypothesis_harness_job_v1";
const REPORT_SCHEMA_VERSION: &str = "recomposition_orchestrator_report_v1";
const PRODUCER_APP: &str = "recomposition-orchestrator-app";
const DEFAULT_AWS_REGION: &str = "ap-northeast-2";

#[derive(Debug, Clone, PartialEq)]
struct Args {
    hypothesis_state_file: Option<PathBuf>,
    input_s3: Option<S3InputArgs>,
    changed_trigger: Vec<String>,
    output_dir: Option<PathBuf>,
    output_s3: Option<S3OutputArgs>,
    now_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct S3InputArgs {
    bucket: String,
    region: String,
    prefix: String,
    profile: Option<String>,
    max_keys: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct S3OutputArgs {
    bucket: String,
    region: String,
    prefix: String,
    profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct HarnessJob {
    harness_job_id: String,
    schema_version: String,
    producer_app: String,
    producer_version: String,
    created_at_ms: i64,
    hypothesis_id: String,
    hypothesis_state_key: String,
    latest_screening_event_id: String,
    requested_harness_type: String,
    changed_triggers: Vec<String>,
    matched_dirty_triggers: Vec<String>,
    input_refs: Vec<String>,
    priority: String,
    idempotency_key: String,
    checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct OrchestratorReport {
    orchestrator_report_id: String,
    schema_version: String,
    producer_app: String,
    producer_version: String,
    created_at_ms: i64,
    input_hypothesis_count: usize,
    input_s3_keys_read: usize,
    selected_job_count: usize,
    skipped_count: usize,
    changed_triggers: Vec<String>,
    output_job_key: String,
    checksum: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct RunSummary {
    hypothesis_states_read: usize,
    input_s3_keys_read: usize,
    harness_jobs_created: usize,
    report_created: bool,
    output_files: Vec<String>,
    output_s3_uris: Vec<String>,
}

#[tokio::main]
async fn main() {
    let result = match parse_args(env::args().skip(1)) {
        Ok(args) => async_run(args).await,
        Err(error) => Err(error),
    };
    match result {
        Ok(summary) => println!("{}", serde_json::to_string_pretty(&summary).unwrap()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

async fn async_run(args: Args) -> AppResult<RunSummary> {
    log_event(
        "orchestrator_started",
        serde_json::json!({
            "changed_triggers": args.changed_trigger,
            "file_input_configured": args.hypothesis_state_file.is_some(),
            "s3_input_configured": args.input_s3.is_some(),
            "s3_output_configured": args.output_s3.is_some()
        }),
    );
    let (states, input_s3_keys_read) = read_hypothesis_states(&args).await?;
    let changed = args
        .changed_trigger
        .iter()
        .map(|trigger| trigger.trim().to_owned())
        .filter(|trigger| !trigger.is_empty())
        .collect::<BTreeSet<_>>();
    if changed.is_empty() {
        return Err(AppError::config("--changed-trigger is required"));
    }
    let created_at_ms = args.now_ms.unwrap_or_else(now_ms);
    let jobs = states
        .iter()
        .filter_map(|state| build_job_if_dirty(state, &changed, created_at_ms))
        .collect::<Vec<_>>();
    let report = build_report(
        created_at_ms,
        states.len(),
        input_s3_keys_read,
        &jobs,
        &changed,
    );

    let mut output_files = Vec::new();
    if let Some(output_dir) = args.output_dir.as_deref() {
        output_files.extend(write_outputs_to_dir(
            output_dir,
            created_at_ms,
            &jobs,
            &report,
        )?);
    }

    let mut output_s3_uris = Vec::new();
    if let Some(s3) = args.output_s3.as_ref() {
        output_s3_uris.extend(write_outputs_to_s3(s3, created_at_ms, &jobs, &report).await?);
    }

    Ok(RunSummary {
        hypothesis_states_read: states.len(),
        input_s3_keys_read,
        harness_jobs_created: jobs.len(),
        report_created: true,
        output_files,
        output_s3_uris,
    })
}

async fn read_hypothesis_states(
    args: &Args,
) -> AppResult<(Vec<IntelCandidateHypothesisState>, usize)> {
    let mut states = Vec::new();
    let mut input_s3_keys_read = 0;
    if let Some(path) = args.hypothesis_state_file.as_deref() {
        states.extend(read_json_array_or_jsonl::<IntelCandidateHypothesisState>(
            path,
        )?);
    }
    if let Some(s3) = args.input_s3.as_ref() {
        let store = ObjectStore::connect(ObjectStoreConfig {
            bucket: s3.bucket.clone(),
            region: s3.region.clone(),
            profile: s3.profile.clone(),
            access_key_id: None,
            secret_access_key: None,
        })
        .await?;
        let keys = store.list_keys(&s3.prefix, s3.max_keys).await?;
        for key in keys {
            if !is_json_payload_key(&key) {
                continue;
            }
            let bytes = store.get_bytes(&key).await?;
            states.extend(read_json_array_or_jsonl_bytes::<
                IntelCandidateHypothesisState,
            >(
                &format!("s3://{}/{}", s3.bucket, key), &bytes
            )?);
            input_s3_keys_read += 1;
        }
    }
    Ok((states, input_s3_keys_read))
}

fn build_job_if_dirty(
    state: &IntelCandidateHypothesisState,
    changed: &BTreeSet<String>,
    created_at_ms: i64,
) -> Option<HarnessJob> {
    let matched = state
        .dirty_triggers
        .iter()
        .filter(|trigger| changed.contains(*trigger))
        .cloned()
        .collect::<Vec<_>>();
    if matched.is_empty() {
        return None;
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
    job.checksum = checksum_json(&job);
    Some(job)
}

fn build_report(
    created_at_ms: i64,
    input_count: usize,
    input_s3_keys_read: usize,
    jobs: &[HarnessJob],
    changed: &BTreeSet<String>,
) -> OrchestratorReport {
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
    report.checksum = checksum_json(&report);
    report
}

fn write_outputs_to_dir(
    output_dir: &Path,
    created_at_ms: i64,
    jobs: &[HarnessJob],
    report: &OrchestratorReport,
) -> AppResult<Vec<String>> {
    Ok(vec![
        write_jsonl(
            output_dir,
            &harness_job_key(created_at_ms, &report.orchestrator_report_id),
            jobs,
        )?,
        write_json(
            output_dir,
            &orchestrator_report_key(created_at_ms, &report.orchestrator_report_id),
            report,
        )?,
    ])
}

async fn write_outputs_to_s3(
    s3: &S3OutputArgs,
    created_at_ms: i64,
    jobs: &[HarnessJob],
    report: &OrchestratorReport,
) -> AppResult<Vec<String>> {
    let store = ObjectStore::connect(ObjectStoreConfig {
        bucket: s3.bucket.clone(),
        region: s3.region.clone(),
        profile: s3.profile.clone(),
        access_key_id: None,
        secret_access_key: None,
    })
    .await?;
    let job_key = prefixed_key(
        &s3.prefix,
        &harness_job_key(created_at_ms, &report.orchestrator_report_id),
    );
    let report_key = prefixed_key(
        &s3.prefix,
        &orchestrator_report_key(created_at_ms, &report.orchestrator_report_id),
    );
    store
        .put_bytes_idempotent(&job_key, jsonl_bytes(jobs)?, "application/x-ndjson")
        .await?;
    store
        .put_bytes_idempotent(
            &report_key,
            serde_json::to_vec_pretty(report)?,
            "application/json",
        )
        .await?;
    Ok(vec![
        format!("s3://{}/{}", s3.bucket, job_key),
        format!("s3://{}/{}", s3.bucket, report_key),
    ])
}

fn harness_job_key(created_at_ms: i64, report_id: &str) -> String {
    let part = partition(created_at_ms);
    format!(
        "hypothesis-harness-job/schema={}/dt={}/hour={:02}/orchestrator_report_id={}/part-000001.jsonl",
        JOB_SCHEMA_VERSION, part.date, part.hour, report_id
    )
}

fn orchestrator_report_key(created_at_ms: i64, report_id: &str) -> String {
    let part = partition(created_at_ms);
    format!(
        "recomposition-orchestrator-report/schema={}/dt={}/hour={:02}/orchestrator_report_id={}/report.json",
        REPORT_SCHEMA_VERSION, part.date, part.hour, report_id
    )
}

fn write_jsonl<T: Serialize>(output_dir: &Path, key: &str, records: &[T]) -> AppResult<String> {
    write_bytes(output_dir, key, &jsonl_bytes(records)?)
}

fn write_json<T: Serialize>(output_dir: &Path, key: &str, record: &T) -> AppResult<String> {
    write_bytes(output_dir, key, &serde_json::to_vec_pretty(record)?)
}

fn write_bytes(output_dir: &Path, key: &str, bytes: &[u8]) -> AppResult<String> {
    let path = output_dir.join(key);
    let parent = path.parent().ok_or_else(|| {
        AppError::validation(format!("output path has no parent: {}", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    let mut file = File::create(&path)?;
    file.write_all(bytes)?;
    Ok(path.display().to_string())
}

fn jsonl_bytes<T: Serialize>(records: &[T]) -> AppResult<Vec<u8>> {
    let mut bytes = Vec::new();
    for record in records {
        serde_json::to_writer(&mut bytes, record)?;
        bytes.push(b'\n');
    }
    Ok(bytes)
}

fn read_json_array_or_jsonl<T: serde::de::DeserializeOwned>(path: &Path) -> AppResult<Vec<T>> {
    let bytes = fs::read(path)?;
    read_json_array_or_jsonl_bytes(&path.display().to_string(), &bytes)
}

fn read_json_array_or_jsonl_bytes<T: serde::de::DeserializeOwned>(
    label: &str,
    bytes: &[u8],
) -> AppResult<Vec<T>> {
    let text =
        std::str::from_utf8(bytes).map_err(|error| AppError::Json(format!("{label}: {error}")))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation(format!("{label} must not be empty")));
    }
    if trimmed.starts_with('[') {
        return Ok(serde_json::from_str(trimmed)?);
    }
    if trimmed.starts_with('{')
        && let Ok(value) = serde_json::from_str(trimmed)
    {
        return Ok(vec![value]);
    }
    let mut values = Vec::new();
    for (index, line) in trimmed.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        values.push(serde_json::from_str(line).map_err(|error| {
            AppError::Json(format!(
                "{label} line {} is not valid JSON: {error}",
                index + 1,
            ))
        })?);
    }
    Ok(values)
}

fn parse_args(mut values: impl Iterator<Item = String>) -> AppResult<Args> {
    let mut args = Args {
        hypothesis_state_file: None,
        input_s3: None,
        changed_trigger: Vec::new(),
        output_dir: None,
        output_s3: None,
        now_ms: None,
    };
    let mut input_s3 = S3InputArgs {
        bucket: String::new(),
        region: DEFAULT_AWS_REGION.to_owned(),
        prefix: String::new(),
        profile: None,
        max_keys: 1_000,
    };
    let mut s3 = S3OutputArgs {
        bucket: String::new(),
        region: DEFAULT_AWS_REGION.to_owned(),
        prefix: String::new(),
        profile: None,
    };
    while let Some(arg) = values.next() {
        match arg.as_str() {
            "--hypothesis-state-file" => {
                args.hypothesis_state_file = Some(absolute_path_arg(
                    values.next(),
                    "--hypothesis-state-file requires an absolute path",
                )?);
            }
            "--changed-trigger" => {
                args.changed_trigger.push(next_string(
                    &mut values,
                    "--changed-trigger requires a value",
                )?);
            }
            "--output-dir" => {
                args.output_dir = Some(absolute_path_arg(
                    values.next(),
                    "--output-dir requires an absolute path",
                )?);
            }
            "--input-s3-bucket" => {
                input_s3.bucket = next_string(&mut values, "--input-s3-bucket requires a bucket")?;
            }
            "--input-s3-region" => {
                input_s3.region = next_string(&mut values, "--input-s3-region requires a region")?;
            }
            "--input-s3-prefix" => {
                input_s3.prefix = next_string(&mut values, "--input-s3-prefix requires a prefix")?;
            }
            "--input-s3-max-keys" => {
                input_s3.max_keys = positive_usize(values.next(), "--input-s3-max-keys")?;
            }
            "--output-s3-bucket" => {
                s3.bucket = next_string(&mut values, "--output-s3-bucket requires a bucket")?
            }
            "--output-s3-region" => {
                s3.region = next_string(&mut values, "--output-s3-region requires a region")?
            }
            "--output-s3-prefix" => {
                s3.prefix = next_string(&mut values, "--output-s3-prefix requires a prefix")?
            }
            "--aws-profile" => {
                let profile = Some(next_string(
                    &mut values,
                    "--aws-profile requires a profile",
                )?);
                input_s3.profile = profile.clone();
                s3.profile = profile;
            }
            "--now-ms" => args.now_ms = Some(non_negative_i64(values.next(), "--now-ms")?),
            "-h" | "--help" => return Err(AppError::config(help_text())),
            other => {
                return Err(AppError::config(format!(
                    "unknown argument: {other}\n\n{}",
                    help_text()
                )));
            }
        }
    }
    if !input_s3.bucket.trim().is_empty() {
        args.input_s3 = Some(input_s3);
    }
    if args.hypothesis_state_file.is_none() && args.input_s3.is_none() {
        return Err(AppError::config(
            "--hypothesis-state-file or --input-s3-bucket/--input-s3-prefix is required",
        ));
    }
    if args.output_dir.is_none() && s3.bucket.trim().is_empty() {
        return Err(AppError::config(
            "at least one output target is required: --output-dir or --output-s3-bucket",
        ));
    }
    if !s3.bucket.trim().is_empty() {
        args.output_s3 = Some(s3);
    }
    Ok(args)
}

fn help_text() -> &'static str {
    r#"recomposition-orchestrator-app
Usage:
  recomposition-orchestrator-app \
    --input-s3-bucket nangman-crypto-dev-intel-candidate-<account-suffix> \
    --input-s3-prefix hypothesis-state/schema=intel_candidate_hypothesis_state_v1/ \
    --changed-trigger market_feature_delta_updated \
    --output-s3-bucket nangman-crypto-dev-research-<account-suffix> \
    --output-s3-prefix recomposition/

Selects dirty hypothesis_state records and emits hypothesis-harness-job plus an
orchestrator report. This app does not mutate raw events and does not run
research directly."#
}

fn absolute_path_arg(value: Option<String>, message: &str) -> AppResult<PathBuf> {
    let value = value.ok_or_else(|| AppError::config(message))?;
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(AppError::config(format!(
            "{message}; got {}",
            path.display()
        )));
    }
    Ok(path)
}

fn next_string(
    values: &mut impl Iterator<Item = String>,
    message: &'static str,
) -> AppResult<String> {
    let value = values.next().ok_or_else(|| AppError::config(message))?;
    if value.trim().is_empty() {
        return Err(AppError::config(message));
    }
    Ok(value)
}

fn non_negative_i64(value: Option<String>, name: &str) -> AppResult<i64> {
    let raw = value.ok_or_else(|| AppError::config(format!("{name} requires a number")))?;
    let parsed = raw
        .parse::<i64>()
        .map_err(|_| AppError::config(format!("{name} must be an integer")))?;
    if parsed < 0 {
        return Err(AppError::config(format!("{name} must be non-negative")));
    }
    Ok(parsed)
}

fn positive_usize(value: Option<String>, name: &str) -> AppResult<usize> {
    let raw = value.ok_or_else(|| AppError::config(format!("{name} requires a number")))?;
    let parsed = raw
        .parse::<usize>()
        .map_err(|_| AppError::config(format!("{name} must be a positive integer")))?;
    if parsed == 0 {
        return Err(AppError::config(format!(
            "{name} must be greater than zero"
        )));
    }
    Ok(parsed)
}

fn is_json_payload_key(key: &str) -> bool {
    key.ends_with(".json") || key.ends_with(".jsonl")
}

fn log_event(event: &str, payload: serde_json::Value) {
    eprintln!(
        "{}",
        serde_json::json!({
            "event": event,
            "producer_app": PRODUCER_APP,
            "timestamp_ms": now_ms(),
            "payload": payload
        })
    );
}

fn prefixed_key(prefix: &str, key: &str) -> String {
    let prefix = prefix.trim_matches('/');
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{prefix}/{key}")
    }
}

struct Partition {
    date: String,
    hour: u32,
}

fn partition(timestamp_ms: i64) -> Partition {
    let datetime =
        DateTime::<Utc>::from_timestamp_millis(timestamp_ms).unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
    Partition {
        date: format!(
            "{:04}-{:02}-{:02}",
            datetime.year(),
            datetime.month(),
            datetime.day()
        ),
        hour: datetime.hour(),
    }
}

fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    format!("{prefix}_{:x}", digest)[..prefix.len() + 1 + 24].to_owned()
}

fn checksum_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable checksum payload");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use intel_candidate_app::model::{CandidateClass, MarketContextStatus, ScoreBreakdown};

    fn state() -> IntelCandidateHypothesisState {
        IntelCandidateHypothesisState {
            hypothesis_id: "hyp_001".to_owned(),
            state_key: "hypothesis-state/state.json".to_owned(),
            schema_version: "intel_candidate_hypothesis_state_v1".to_owned(),
            producer_app: "intel-candidate-app".to_owned(),
            producer_version: "0.1.0".to_owned(),
            created_at_ms: 1_000,
            updated_at_ms: 1_000,
            input_packet_id: "packet_001".to_owned(),
            input_packet_family_id: "family_001".to_owned(),
            input_packet_revision: 0,
            source_structured_packet_ids: vec!["packet_001".to_owned()],
            source_event_ids: vec!["source_001".to_owned()],
            supersedes_packet_id: None,
            supersedes_hypothesis_id: None,
            latest_screening_event_id: "screen_001".to_owned(),
            scoring_policy_version: "policy_v1".to_owned(),
            normalized_symbols: vec!["BTC".to_owned()],
            event_type: "funding_shift".to_owned(),
            hypothesis_type: "derivatives_pressure_shift".to_owned(),
            current_state: CandidateClass::WeakCandidate,
            current_score: 45,
            previous_score: None,
            research_eligible: false,
            transition: "created_or_refreshed".to_owned(),
            next_action: "rerun_when_market_feature_delta_updates".to_owned(),
            reasons: vec!["derivatives_metric_delta_missing".to_owned()],
            retryable_reasons: vec!["derivatives_metric_delta_missing".to_owned()],
            terminal_reasons: Vec::new(),
            selected_market_artifacts: Vec::new(),
            market_context_ref: None,
            market_context_status: MarketContextStatus::StaleButUsable,
            evidence_quality_reasons: Vec::new(),
            score_breakdown: ScoreBreakdown {
                components: Vec::new(),
                final_score: 45,
            },
            lineage_refs: vec!["packet_001".to_owned()],
            dirty_triggers: vec!["market_feature_delta_updated".to_owned()],
            harness_queue_hint: "derivatives_delta_persistence".to_owned(),
            idempotency_key: "idem_001".to_owned(),
            checksum: "checksum".to_owned(),
        }
    }

    #[test]
    fn selects_state_when_trigger_matches() {
        let changed = BTreeSet::from(["market_feature_delta_updated".to_owned()]);
        let job = build_job_if_dirty(&state(), &changed, 7_200_000).expect("job selected");
        assert_eq!(job.priority, "p1_market_retest");
        assert_eq!(job.requested_harness_type, "derivatives_delta_persistence");
    }

    #[test]
    fn skips_state_when_trigger_does_not_match() {
        let changed = BTreeSet::from(["source_registry_version_changed".to_owned()]);
        assert!(build_job_if_dirty(&state(), &changed, 7_200_000).is_none());
    }

    #[test]
    fn job_id_includes_state_revision_identity() {
        let changed = BTreeSet::from(["market_feature_delta_updated".to_owned()]);
        let mut first = state();
        let mut second = state();
        second.state_key = "hypothesis-state/next-state.json".to_owned();
        second.updated_at_ms = 2_000;

        let first_job = build_job_if_dirty(&first, &changed, 7_200_000).expect("job selected");
        let second_job = build_job_if_dirty(&second, &changed, 7_200_000).expect("job selected");

        assert_ne!(first_job.harness_job_id, second_job.harness_job_id);

        first.state_key = second.state_key;
        first.updated_at_ms = second.updated_at_ms;
        let repeated_job = build_job_if_dirty(&first, &changed, 7_200_000).expect("job selected");
        assert_eq!(second_job.harness_job_id, repeated_job.harness_job_id);
    }

    #[test]
    fn aws_profile_applies_to_input_and_output_s3() {
        let args = parse_args(
            [
                "--input-s3-bucket",
                "candidate-bucket",
                "--input-s3-prefix",
                "hypothesis-state/",
                "--output-s3-bucket",
                "research-bucket",
                "--output-s3-prefix",
                "recomposition/",
                "--changed-trigger",
                "market_feature_delta_updated",
                "--aws-profile",
                "dev-profile",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert_eq!(
            args.input_s3.and_then(|s3| s3.profile),
            Some("dev-profile".to_owned())
        );
        assert_eq!(
            args.output_s3.and_then(|s3| s3.profile),
            Some("dev-profile".to_owned())
        );
    }
}
