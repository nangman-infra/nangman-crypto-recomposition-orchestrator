use super::validation::{absolute_path_arg, next_string, non_negative_i64, positive_usize};
use crate::DEFAULT_AWS_REGION;
use crate::types::{Args, S3InputArgs, S3OutputArgs};
use intel_candidate_app::error::{AppError, AppResult};

pub(crate) fn parse_args(mut values: impl Iterator<Item = String>) -> AppResult<Args> {
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
