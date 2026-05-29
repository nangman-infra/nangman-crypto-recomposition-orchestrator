use chrono::{DateTime, Datelike, Timelike, Utc};
use intel_candidate_app::error::AppResult;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub(crate) fn prefixed_key(prefix: &str, key: &str) -> String {
    let prefix = prefix.trim_matches('/');
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{prefix}/{key}")
    }
}

pub(crate) struct Partition {
    pub(crate) date: String,
    pub(crate) hour: u32,
}

pub(crate) fn partition(timestamp_ms: i64) -> Partition {
    let datetime =
        DateTime::<Utc>::from_timestamp_millis(timestamp_ms).unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
    Partition {
        date: format!(
            "{:04}-{:02}-{:02}",
            datetime.year(),
            datetime.month(),
            datetime.day()
        ),
        hour: datetime.hour(),
    }
}

pub(crate) fn stable_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    format!("{prefix}_{:x}", digest)[..prefix.len() + 1 + 24].to_owned()
}

pub(crate) fn checksum_json<T: Serialize>(value: &T) -> AppResult<String> {
    let bytes = serde_json::to_vec(value)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}
