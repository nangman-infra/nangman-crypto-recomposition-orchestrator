use crate::path_validation::validate_unambiguous_absolute_path;
use intel_candidate_app::error::{AppError, AppResult};
use std::path::PathBuf;

pub(in crate::args) fn absolute_path_arg(
    value: Option<String>,
    message: &str,
) -> AppResult<PathBuf> {
    let value = value.ok_or_else(|| AppError::config(message))?;
    let path = PathBuf::from(value);
    let label = message
        .split_once(" requires")
        .map(|(label, _)| label)
        .unwrap_or(message);
    validate_unambiguous_absolute_path(&path, label)
        .map_err(|error| AppError::config(format!("{message}; {error}")))?;
    Ok(path)
}

pub(in crate::args) fn next_string(
    values: &mut impl Iterator<Item = String>,
    message: &'static str,
) -> AppResult<String> {
    let value = values.next().ok_or_else(|| AppError::config(message))?;
    if value.trim().is_empty() {
        return Err(AppError::config(message));
    }
    Ok(value)
}

pub(in crate::args) fn non_negative_i64(value: Option<String>, name: &str) -> AppResult<i64> {
    let raw = value.ok_or_else(|| AppError::config(format!("{name} requires a number")))?;
    let parsed = raw
        .parse::<i64>()
        .map_err(|_| AppError::config(format!("{name} must be an integer")))?;
    if parsed < 0 {
        return Err(AppError::config(format!("{name} must be non-negative")));
    }
    Ok(parsed)
}

pub(in crate::args) fn positive_usize(value: Option<String>, name: &str) -> AppResult<usize> {
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
