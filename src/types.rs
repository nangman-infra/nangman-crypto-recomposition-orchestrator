use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Args {
    pub(crate) hypothesis_state_file: Option<PathBuf>,
    pub(crate) input_s3: Option<S3InputArgs>,
    pub(crate) changed_trigger: Vec<String>,
    pub(crate) output_dir: Option<PathBuf>,
    pub(crate) output_s3: Option<S3OutputArgs>,
    pub(crate) now_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct S3InputArgs {
    pub(crate) bucket: String,
    pub(crate) region: String,
    pub(crate) prefix: String,
    pub(crate) profile: Option<String>,
    pub(crate) max_keys: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct S3OutputArgs {
    pub(crate) bucket: String,
    pub(crate) region: String,
    pub(crate) prefix: String,
    pub(crate) profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct HarnessJob {
    pub(crate) harness_job_id: String,
    pub(crate) schema_version: String,
    pub(crate) producer_app: String,
    pub(crate) producer_version: String,
    pub(crate) created_at_ms: i64,
    pub(crate) hypothesis_id: String,
    pub(crate) hypothesis_state_key: String,
    pub(crate) latest_screening_event_id: String,
    pub(crate) requested_harness_type: String,
    pub(crate) changed_triggers: Vec<String>,
    pub(crate) matched_dirty_triggers: Vec<String>,
    pub(crate) input_refs: Vec<String>,
    pub(crate) priority: String,
    pub(crate) idempotency_key: String,
    pub(crate) checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct OrchestratorReport {
    pub(crate) orchestrator_report_id: String,
    pub(crate) schema_version: String,
    pub(crate) producer_app: String,
    pub(crate) producer_version: String,
    pub(crate) created_at_ms: i64,
    pub(crate) input_hypothesis_count: usize,
    pub(crate) input_s3_keys_read: usize,
    pub(crate) selected_job_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) changed_triggers: Vec<String>,
    pub(crate) output_job_key: String,
    pub(crate) checksum: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RunSummary {
    pub(crate) hypothesis_states_read: usize,
    pub(crate) input_s3_keys_read: usize,
    pub(crate) harness_jobs_created: usize,
    pub(crate) report_created: bool,
    pub(crate) output_files: Vec<String>,
    pub(crate) output_s3_uris: Vec<String>,
}
