use crate::ids::partition;
use crate::{JOB_SCHEMA_VERSION, REPORT_SCHEMA_VERSION};

pub(crate) fn harness_job_key(created_at_ms: i64, report_id: &str) -> String {
    let part = partition(created_at_ms);
    format!(
        "hypothesis-harness-job/schema={}/dt={}/hour={:02}/orchestrator_report_id={}/part-000001.jsonl",
        JOB_SCHEMA_VERSION, part.date, part.hour, report_id
    )
}

pub(super) fn orchestrator_report_key(created_at_ms: i64, report_id: &str) -> String {
    let part = partition(created_at_ms);
    format!(
        "recomposition-orchestrator-report/schema={}/dt={}/hour={:02}/orchestrator_report_id={}/report.json",
        REPORT_SCHEMA_VERSION, part.date, part.hour, report_id
    )
}
