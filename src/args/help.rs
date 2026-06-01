pub(super) fn help_text() -> &'static str {
    r#"recomposition-orchestrator-app
Usage:
  recomposition-orchestrator-app \
    --input-s3-bucket nangman-crypto-dev-intel-candidate-<account-suffix> \
    --input-s3-prefix hypothesis-state/schema=intel_candidate_hypothesis_state_v1/ \
    --changed-trigger market_feature_delta_updated \
    --output-s3-bucket nangman-crypto-dev-research-<account-suffix> \
    --output-s3-prefix recomposition/

Selects dirty hypothesis_state records and emits hypothesis-harness-job plus an
orchestrator report. This app does not mutate raw events and does not run
research directly."#
}
