use intel_candidate_app::error::{AppError, AppResult};
use serde::Serialize;
use std::fs;
use std::path::Path;

pub(super) fn jsonl_bytes<T: Serialize>(records: &[T]) -> AppResult<Vec<u8>> {
    let mut bytes = Vec::new();
    for record in records {
        serde_json::to_writer(&mut bytes, record)?;
        bytes.push(b'\n');
    }
    Ok(bytes)
}

pub(super) fn read_json_array_or_jsonl<T: serde::de::DeserializeOwned>(
    path: &Path,
) -> AppResult<Vec<T>> {
    let bytes = fs::read(path)?;
    read_json_array_or_jsonl_bytes(&path.display().to_string(), &bytes)
}

pub(super) fn read_json_array_or_jsonl_bytes<T: serde::de::DeserializeOwned>(
    label: &str,
    bytes: &[u8],
) -> AppResult<Vec<T>> {
    let text =
        std::str::from_utf8(bytes).map_err(|error| AppError::Json(format!("{label}: {error}")))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation(format!("{label} must not be empty")));
    }
    if trimmed.starts_with('[') {
        return Ok(serde_json::from_str(trimmed)?);
    }
    if trimmed.starts_with('{')
        && let Ok(value) = serde_json::from_str(trimmed)
    {
        return Ok(vec![value]);
    }
    let mut values = Vec::new();
    for (index, line) in trimmed.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        values.push(serde_json::from_str(line).map_err(|error| {
            AppError::Json(format!(
                "{label} line {} is not valid JSON: {error}",
                index + 1,
            ))
        })?);
    }
    Ok(values)
}
