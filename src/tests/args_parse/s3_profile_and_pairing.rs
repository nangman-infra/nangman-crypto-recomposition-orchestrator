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

#[test]
fn rejects_s3_bucket_without_matching_prefix() {
    let err = parse_args(
        [
            "--input-s3-bucket",
            "candidate-bucket",
            "--output-dir",
            "/tmp/out",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("--input-s3-prefix"));
}

#[test]
fn rejects_s3_prefix_without_matching_bucket() {
    let err = parse_args(
        [
            "--input-s3-prefix",
            "hypothesis-state/",
            "--output-dir",
            "/tmp/out",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("--input-s3-prefix"));
    assert!(err.contains("--input-s3-bucket"));
}
