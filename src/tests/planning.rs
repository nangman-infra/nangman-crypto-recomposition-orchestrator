use super::fixtures::state;
use crate::planning::build_job_if_dirty;
use std::collections::BTreeSet;

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
