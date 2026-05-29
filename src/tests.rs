use crate::args::parse_args;
use crate::planning::build_job_if_dirty;
use intel_candidate_app::model::{
    CandidateClass, IntelCandidateHypothesisState, MarketContextStatus, ScoreBreakdown,
};
use std::collections::BTreeSet;

fn state() -> IntelCandidateHypothesisState {
    IntelCandidateHypothesisState {
        hypothesis_id: "hyp_001".to_owned(),
        state_key: "hypothesis-state/state.json".to_owned(),
        schema_version: "intel_candidate_hypothesis_state_v1".to_owned(),
        producer_app: "intel-candidate-app".to_owned(),
        producer_version: "0.1.0".to_owned(),
        created_at_ms: 1_000,
        updated_at_ms: 1_000,
        input_packet_id: "packet_001".to_owned(),
        input_packet_family_id: "family_001".to_owned(),
        input_packet_revision: 0,
        source_structured_packet_ids: vec!["packet_001".to_owned()],
        source_event_ids: vec!["source_001".to_owned()],
        supersedes_packet_id: None,
        supersedes_hypothesis_id: None,
        latest_screening_event_id: "screen_001".to_owned(),
        scoring_policy_version: "policy_v1".to_owned(),
        normalized_symbols: vec!["BTC".to_owned()],
        event_type: "funding_shift".to_owned(),
        hypothesis_type: "derivatives_pressure_shift".to_owned(),
        current_state: CandidateClass::WeakCandidate,
        current_score: 45,
        previous_score: None,
        research_eligible: false,
        transition: "created_or_refreshed".to_owned(),
        next_action: "rerun_when_market_feature_delta_updates".to_owned(),
        reasons: vec!["derivatives_metric_delta_missing".to_owned()],
        retryable_reasons: vec!["derivatives_metric_delta_missing".to_owned()],
        terminal_reasons: Vec::new(),
        selected_market_artifacts: Vec::new(),
        market_context_ref: None,
        market_context_status: MarketContextStatus::StaleButUsable,
        evidence_quality_reasons: Vec::new(),
        score_breakdown: ScoreBreakdown {
            components: Vec::new(),
            final_score: 45,
        },
        lineage_refs: vec!["packet_001".to_owned()],
        dirty_triggers: vec!["market_feature_delta_updated".to_owned()],
        harness_queue_hint: "derivatives_delta_persistence".to_owned(),
        idempotency_key: "idem_001".to_owned(),
        checksum: "checksum".to_owned(),
    }
}

#[test]
fn selects_state_when_trigger_matches() {
    let changed = BTreeSet::from(["market_feature_delta_updated".to_owned()]);
    let job = build_job_if_dirty(&state(), &changed, 7_200_000)
        .unwrap()
        .expect("job selected");
    assert_eq!(job.priority, "p1_market_retest");
    assert_eq!(job.requested_harness_type, "derivatives_delta_persistence");
}

#[test]
fn skips_state_when_trigger_does_not_match() {
    let changed = BTreeSet::from(["source_registry_version_changed".to_owned()]);
    assert!(
        build_job_if_dirty(&state(), &changed, 7_200_000)
            .unwrap()
            .is_none()
    );
}

#[test]
fn job_id_includes_state_revision_identity() {
    let changed = BTreeSet::from(["market_feature_delta_updated".to_owned()]);
    let mut first = state();
    let mut second = state();
    second.state_key = "hypothesis-state/next-state.json".to_owned();
    second.updated_at_ms = 2_000;

    let first_job = build_job_if_dirty(&first, &changed, 7_200_000)
        .unwrap()
        .expect("job selected");
    let second_job = build_job_if_dirty(&second, &changed, 7_200_000)
        .unwrap()
        .expect("job selected");

    assert_ne!(first_job.harness_job_id, second_job.harness_job_id);

    first.state_key = second.state_key;
    first.updated_at_ms = second.updated_at_ms;
    let repeated_job = build_job_if_dirty(&first, &changed, 7_200_000)
        .unwrap()
        .expect("job selected");
    assert_eq!(second_job.harness_job_id, repeated_job.harness_job_id);
}

#[test]
fn aws_profile_applies_to_input_and_output_s3() {
    let args = parse_args(
        [
            "--input-s3-bucket",
            "candidate-bucket",
            "--input-s3-prefix",
            "hypothesis-state/",
            "--output-s3-bucket",
            "research-bucket",
            "--output-s3-prefix",
            "recomposition/",
            "--changed-trigger",
            "market_feature_delta_updated",
            "--aws-profile",
            "dev-profile",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap();

    assert_eq!(
        args.input_s3.and_then(|s3| s3.profile),
        Some("dev-profile".to_owned())
    );
    assert_eq!(
        args.output_s3.and_then(|s3| s3.profile),
        Some("dev-profile".to_owned())
    );
}
