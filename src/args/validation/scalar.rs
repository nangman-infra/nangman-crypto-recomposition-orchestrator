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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_negative_i64_accepts_zero_and_positive_values() {
        assert_eq!(
            non_negative_i64(Some("0".to_owned()), "--now-ms").unwrap(),
            0
        );
        assert_eq!(
            non_negative_i64(Some("7200000".to_owned()), "--now-ms").unwrap(),
            7_200_000
        );
    }

    #[test]
    fn non_negative_i64_rejects_missing_invalid_and_negative_values() {
        for (value, expected) in [
            (None, "requires a number"),
            (Some("abc".to_owned()), "must be an integer"),
            (Some("-1".to_owned()), "must be non-negative"),
        ] {
            let error = non_negative_i64(value, "--now-ms").unwrap_err().to_string();
            assert!(
                error.contains(expected),
                "expected {expected:?}, got {error:?}"
            );
        }
    }

    #[test]
    fn positive_usize_accepts_positive_values() {
        assert_eq!(
            positive_usize(Some("1".to_owned()), "--input-s3-max-keys").unwrap(),
            1
        );
        assert_eq!(
            positive_usize(Some("1000".to_owned()), "--input-s3-max-keys").unwrap(),
            1000
        );
    }

    #[test]
    fn positive_usize_rejects_missing_invalid_and_zero_values() {
        for (value, expected) in [
            (None, "requires a number"),
            (Some("abc".to_owned()), "must be a positive integer"),
            (Some("0".to_owned()), "must be greater than zero"),
        ] {
            let error = positive_usize(value, "--input-s3-max-keys")
                .unwrap_err()
                .to_string();
            assert!(
                error.contains(expected),
                "expected {expected:?}, got {error:?}"
            );
        }
    }
}
