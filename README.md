# Recomposition Orchestrator App

`recomposition-orchestrator-app` selects stored `intel_candidate_hypothesis_state_v1`
records that became dirty after a market, policy, app-version, or source-registry
change. It writes deterministic `hypothesis_harness_job_v1` work and a run report.

It does not mutate raw intel, run research, or change candidate scores.

```text
hypothesis_state prefix
  + changed_trigger[]
  -> dirty-trigger selection
  -> hypothesis_harness_job_v1
  -> recomposition_orchestrator_report_v1
```

## Local Run

```bash
cd /Volumes/WD/Developments/nangman-crypto/apps/recomposition-orchestrator-app
cargo run -- \
  --hypothesis-state-file /Volumes/WD/Developments/nangman-crypto/apps/hypothesis-harness-app/testdata/hypothesis-state.sample.jsonl \
  --changed-trigger market_feature_delta_updated \
  --output-dir /tmp/nangman-recomposition-smoke
```

## Scheduled ECS Run

Use an EventBridge Scheduler or one-shot ECS RunTask with S3 prefix inputs.

```bash
recomposition-orchestrator-app \
  --input-s3-bucket nangman-crypto-dev-intel-candidate-<account-suffix> \
  --input-s3-prefix hypothesis-state/schema=intel_candidate_hypothesis_state_v1/ \
  --input-s3-region ap-northeast-2 \
  --input-s3-max-keys 1000 \
  --changed-trigger market_feature_delta_updated \
  --output-s3-bucket nangman-crypto-dev-research-<account-suffix> \
  --output-s3-prefix recomposition/ \
  --output-s3-region ap-northeast-2
```

Run cadence recommendation:

```text
market_feature_delta_updated       every 5-15 minutes
derivatives_rollup_updated         every 5-15 minutes
market_context_updated             every 30-60 minutes
scoring_policy_version_changed     one-shot after deployment
candidate_app_version_changed      one-shot after deployment
source_registry_version_changed    one-shot after source registry deployment
```

Every invocation writes a report, even when there are no selected jobs. That is
intentional so operations can distinguish "ran and found nothing" from "did not
run."

## S3 Outputs

```text
s3://nangman-crypto-dev-research-<account-suffix>/recomposition/hypothesis-harness-job/schema=hypothesis_harness_job_v1/...
s3://nangman-crypto-dev-research-<account-suffix>/recomposition/recomposition-orchestrator-report/schema=recomposition_orchestrator_report_v1/...
```

## Safety Boundary

```text
no raw event mutation
no candidate score mutation
no research execution
no order or paper-trading action
always report run result
```
