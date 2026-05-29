use crate::args::is_json_payload_key;
use crate::types::Args;
use intel_candidate_app::error::AppResult;
use intel_candidate_app::model::IntelCandidateHypothesisState;
use intel_candidate_app::storage::{ObjectStore, ObjectStoreConfig};

use super::json_payload::{read_json_array_or_jsonl, read_json_array_or_jsonl_bytes};

pub(crate) async fn read_hypothesis_states(
    args: &Args,
) -> AppResult<(Vec<IntelCandidateHypothesisState>, usize)> {
    let mut states = Vec::new();
    let mut input_s3_keys_read = 0;
    if let Some(path) = args.hypothesis_state_file.as_deref() {
        states.extend(read_json_array_or_jsonl::<IntelCandidateHypothesisState>(
            path,
        )?);
    }
    if let Some(s3) = args.input_s3.as_ref() {
        let store = ObjectStore::connect(ObjectStoreConfig {
            bucket: s3.bucket.clone(),
            region: s3.region.clone(),
            profile: s3.profile.clone(),
            access_key_id: None,
            secret_access_key: None,
        })
        .await?;
        let keys = store.list_keys(&s3.prefix, s3.max_keys).await?;
        for key in keys {
            if !is_json_payload_key(&key) {
                continue;
            }
            let bytes = store.get_bytes(&key).await?;
            states.extend(read_json_array_or_jsonl_bytes::<
                IntelCandidateHypothesisState,
            >(
                &format!("s3://{}/{}", s3.bucket, key), &bytes
            )?);
            input_s3_keys_read += 1;
        }
    }
    Ok((states, input_s3_keys_read))
}
