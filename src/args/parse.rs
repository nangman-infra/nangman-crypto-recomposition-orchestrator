mod defaults;
mod finalize;

use super::help::help_text;
use super::validation::{absolute_path_arg, next_string, non_negative_i64, positive_usize};
use crate::types::Args;
use defaults::{default_args, default_input_s3, default_output_s3};
use finalize::finalize_args;
use intel_candidate_app::error::{AppError, AppResult};

pub(crate) fn parse_args(mut values: impl Iterator<Item = String>) -> AppResult<Args> {
    let mut args = default_args();
    let mut input_s3 = default_input_s3();
    let mut s3 = default_output_s3();
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
    finalize_args(args, input_s3, s3)
}
