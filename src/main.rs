use intel_candidate_app::error::{AppError, AppResult};
use std::collections::BTreeSet;
use std::env;

const JOB_SCHEMA_VERSION: &str = "hypothesis_harness_job_v1";
const REPORT_SCHEMA_VERSION: &str = "recomposition_orchestrator_report_v1";
const PRODUCER_APP: &str = "recomposition-orchestrator-app";
const DEFAULT_AWS_REGION: &str = "ap-northeast-2";

mod args;
mod ids;
mod io;
mod planning;
#[cfg(test)]
mod tests;
mod types;

use args::{log_event, parse_args};
use ids::now_ms;
use io::{read_hypothesis_states, write_outputs_to_dir, write_outputs_to_s3};
use planning::{build_job_if_dirty, build_report};
use types::{Args, RunSummary};

#[tokio::main]
async fn main() {
    let result = match parse_args(env::args().skip(1)) {
        Ok(args) => async_run(args).await,
        Err(error) => Err(error),
    };
    match result {
        Ok(summary) => match serde_json::to_string_pretty(&summary) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
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
    let mut jobs = Vec::new();
    for state in &states {
        if let Some(job) = build_job_if_dirty(state, &changed, created_at_ms)? {
            jobs.push(job);
        }
    }
    let report = build_report(
        created_at_ms,
        states.len(),
        input_s3_keys_read,
        &jobs,
        &changed,
    )?;

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
