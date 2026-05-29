pub(crate) fn is_json_payload_key(key: &str) -> bool {
    key.ends_with(".json") || key.ends_with(".jsonl")
}
