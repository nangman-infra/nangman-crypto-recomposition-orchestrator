use crate::ids::prefixed_key;
use intel_candidate_app::error::{AppError, AppResult};
use std::path::{Component, Path};

const MAX_S3_OBJECT_KEY_BYTES: usize = 1024;

pub(super) fn validate_key_component(value: &str, label: &'static str) -> AppResult<()> {
    if value.is_empty() {
        return Err(AppError::validation(format!("{label} is required")));
    }
    if value == "." || value == ".." {
        return Err(AppError::validation(format!(
            "{label} must not be a period-only path segment"
        )));
    }
    if value
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err(AppError::validation(format!(
            "{label} must not contain path separators or control characters"
        )));
    }
    Ok(())
}

pub(super) fn validate_output_key(key: &str) -> AppResult<()> {
    validate_relative_key(key, false)
}

pub(super) fn safe_prefixed_key(prefix: &str, key: &str) -> AppResult<String> {
    let prefix = prefix.trim_matches('/');
    validate_relative_key(prefix, true)?;
    let key = prefixed_key(prefix, key);
    validate_output_key(&key)?;
    Ok(key)
}

fn validate_relative_key(value: &str, allow_empty: bool) -> AppResult<()> {
    if value.is_empty() {
        if allow_empty {
            return Ok(());
        }
        return Err(AppError::validation("output key is required"));
    }
    if value.len() > MAX_S3_OBJECT_KEY_BYTES {
        return Err(AppError::validation(format!(
            "output key must be at most {MAX_S3_OBJECT_KEY_BYTES} bytes"
        )));
    }
    if value.chars().any(|ch| ch.is_control() || ch == '\\') {
        return Err(AppError::validation(
            "output key must not contain control characters or backslashes",
        ));
    }
    if value
        .split('/')
        .any(|segment| matches!(segment, "." | ".."))
    {
        return Err(AppError::validation(
            "output key must not contain period-only path segments",
        ));
    }
    for component in Path::new(value).components() {
        match component {
            Component::Normal(_) => {}
            Component::Prefix(_)
            | Component::RootDir
            | Component::CurDir
            | Component::ParentDir => {
                return Err(AppError::validation(
                    "output key must be a relative object key without path traversal",
                ));
            }
        }
    }
    Ok(())
}
