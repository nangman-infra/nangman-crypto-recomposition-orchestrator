use crate::path_validation::validate_unambiguous_absolute_path;
use crate::types::{HarnessJob, OrchestratorReport, S3OutputArgs};
use intel_candidate_app::error::{AppError, AppResult};
use intel_candidate_app::storage::{ObjectStore, ObjectStoreConfig};
use serde::Serialize;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use super::json_payload::jsonl_bytes;
use super::keys::{harness_job_key, orchestrator_report_key};
use super::validation::{safe_prefixed_key, validate_key_component, validate_output_key};

pub(crate) fn write_outputs_to_dir(
    output_dir: &Path,
    created_at_ms: i64,
    jobs: &[HarnessJob],
    report: &OrchestratorReport,
) -> AppResult<Vec<String>> {
    validate_key_component(&report.orchestrator_report_id, "orchestrator report id")?;
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

pub(crate) async fn write_outputs_to_s3(
    s3: &S3OutputArgs,
    created_at_ms: i64,
    jobs: &[HarnessJob],
    report: &OrchestratorReport,
) -> AppResult<Vec<String>> {
    validate_key_component(&report.orchestrator_report_id, "orchestrator report id")?;
    let store = ObjectStore::connect(ObjectStoreConfig {
        bucket: s3.bucket.clone(),
        region: s3.region.clone(),
        profile: s3.profile.clone(),
        access_key_id: None,
        secret_access_key: None,
    })
    .await?;
    let job_key = safe_prefixed_key(
        &s3.prefix,
        &harness_job_key(created_at_ms, &report.orchestrator_report_id),
    )?;
    let report_key = safe_prefixed_key(
        &s3.prefix,
        &orchestrator_report_key(created_at_ms, &report.orchestrator_report_id),
    )?;
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

fn write_jsonl<T: Serialize>(output_dir: &Path, key: &str, records: &[T]) -> AppResult<String> {
    write_bytes(output_dir, key, &jsonl_bytes(records)?)
}

fn write_json<T: Serialize>(output_dir: &Path, key: &str, record: &T) -> AppResult<String> {
    write_bytes(output_dir, key, &serde_json::to_vec_pretty(record)?)
}

fn write_bytes(output_dir: &Path, key: &str, bytes: &[u8]) -> AppResult<String> {
    validate_unambiguous_absolute_path(output_dir, "output dir").map_err(AppError::validation)?;
    validate_output_key(key)?;
    let path = output_dir.join(key);
    let parent = path.parent().ok_or_else(|| {
        AppError::validation(format!("output path has no parent: {}", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    let mut file = create_output_file(&path)?;
    file.write_all(bytes)?;
    Ok(path.display().to_string())
}

fn create_output_file(path: &Path) -> AppResult<File> {
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(AppError::validation(format!(
            "output path must not be a symlink: {}",
            path.display()
        )));
    }
    Ok(File::create(path)?)
}
