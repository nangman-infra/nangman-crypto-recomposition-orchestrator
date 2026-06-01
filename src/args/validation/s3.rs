use crate::types::{S3InputArgs, S3OutputArgs};
use intel_candidate_app::error::{AppError, AppResult};

const MAX_S3_OBJECT_KEY_BYTES: usize = 1024;

pub(in crate::args) fn validate_s3_input_arg(
    s3: &S3InputArgs,
    bucket_flag: &str,
    prefix_flag: &str,
) -> AppResult<()> {
    validate_bucket_prefix_pair(&s3.bucket, &s3.prefix, bucket_flag, prefix_flag)
}

pub(in crate::args) fn validate_s3_output_arg(
    s3: &S3OutputArgs,
    bucket_flag: &str,
    prefix_flag: &str,
) -> AppResult<()> {
    validate_bucket_prefix_pair(&s3.bucket, &s3.prefix, bucket_flag, prefix_flag)
}

fn validate_bucket_prefix_pair(
    bucket: &str,
    prefix: &str,
    bucket_flag: &str,
    prefix_flag: &str,
) -> AppResult<()> {
    if bucket.trim().is_empty() {
        if !prefix.trim().is_empty() {
            return Err(AppError::config(format!(
                "{prefix_flag} requires {bucket_flag}"
            )));
        }
        return Ok(());
    }

    validate_bucket(bucket, bucket_flag)?;
    validate_prefix(prefix, prefix_flag)
}

fn validate_bucket(bucket: &str, bucket_flag: &str) -> AppResult<()> {
    if bucket.trim() != bucket {
        return Err(AppError::config(format!(
            "{bucket_flag} must not include leading or trailing whitespace"
        )));
    }
    if bucket.contains('<') || bucket.contains('>') {
        return Err(AppError::config(format!(
            "{bucket_flag} must be a real bucket name, not a public-doc placeholder"
        )));
    }
    if bucket.to_ascii_lowercase().starts_with("s3://") || bucket.contains('/') {
        return Err(AppError::config(format!(
            "{bucket_flag} must be a bucket name, not an S3 URI"
        )));
    }
    if bucket
        .chars()
        .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return Err(AppError::config(format!(
            "{bucket_flag} must not contain whitespace or control characters"
        )));
    }
    Ok(())
}

fn validate_prefix(prefix: &str, prefix_flag: &str) -> AppResult<()> {
    if prefix.is_empty() {
        return Err(AppError::config(format!("{prefix_flag} is required")));
    }
    if prefix.trim() != prefix {
        return Err(AppError::config(format!(
            "{prefix_flag} must not include leading or trailing whitespace"
        )));
    }
    if prefix.len() > MAX_S3_OBJECT_KEY_BYTES {
        return Err(AppError::config(format!(
            "{prefix_flag} must be at most {MAX_S3_OBJECT_KEY_BYTES} bytes"
        )));
    }
    if prefix.starts_with('/') || prefix.to_ascii_lowercase().starts_with("s3://") {
        return Err(AppError::config(format!(
            "{prefix_flag} must be an object prefix, not a URI or absolute path"
        )));
    }
    if prefix.contains('?') || prefix.contains('#') {
        return Err(AppError::config(format!(
            "{prefix_flag} must not include query or fragment markers"
        )));
    }
    if prefix
        .chars()
        .any(|ch| ch.is_control() || ch.is_whitespace() || ch == '\\')
    {
        return Err(AppError::config(format!(
            "{prefix_flag} must not contain control characters, whitespace, or backslashes"
        )));
    }

    let normalized = prefix.strip_suffix('/').unwrap_or(prefix);
    if normalized.is_empty() || normalized.split('/').any(str::is_empty) {
        return Err(AppError::config(format!(
            "{prefix_flag} must not contain empty path segments"
        )));
    }
    if normalized
        .split('/')
        .any(|segment| matches!(segment, "." | ".."))
    {
        return Err(AppError::config(format!(
            "{prefix_flag} must not contain period-only path segments"
        )));
    }
    Ok(())
}
