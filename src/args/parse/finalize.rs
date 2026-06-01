use super::super::validation::{validate_s3_input_arg, validate_s3_output_arg};
use crate::types::{Args, S3InputArgs, S3OutputArgs};
use intel_candidate_app::error::{AppError, AppResult};

pub(super) fn finalize_args(
    mut args: Args,
    input_s3: S3InputArgs,
    output_s3: S3OutputArgs,
) -> AppResult<Args> {
    validate_s3_input_arg(&input_s3, "--input-s3-bucket", "--input-s3-prefix")?;
    validate_s3_output_arg(&output_s3, "--output-s3-bucket", "--output-s3-prefix")?;
    if !input_s3.bucket.trim().is_empty() {
        args.input_s3 = Some(input_s3);
    }
    if args.hypothesis_state_file.is_none() && args.input_s3.is_none() {
        return Err(AppError::config(
            "--hypothesis-state-file or --input-s3-bucket/--input-s3-prefix is required",
        ));
    }
    if args.output_dir.is_none() && output_s3.bucket.trim().is_empty() {
        return Err(AppError::config(
            "at least one output target is required: --output-dir or --output-s3-bucket",
        ));
    }
    if !output_s3.bucket.trim().is_empty() {
        args.output_s3 = Some(output_s3);
    }
    Ok(args)
}
