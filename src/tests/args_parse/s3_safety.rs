#[test]
fn rejects_unsafe_s3_prefixes() {
    for (flag, value) in [
        (
            "--input-s3-prefix",
            "s3://candidate-bucket/hypothesis-state/",
        ),
        ("--output-s3-prefix", "/recomposition/"),
        ("--output-s3-prefix", "recomposition/../jobs/"),
        ("--output-s3-prefix", "recomposition jobs/"),
    ] {
        let err = parse_args(
            [
                "--input-s3-bucket",
                "candidate-bucket",
                "--input-s3-prefix",
                "hypothesis-state/",
                "--output-s3-bucket",
                "research-bucket",
                "--output-s3-prefix",
                "recomposition/",
                flag,
                value,
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains(flag), "expected {flag} in {err}");
    }
}

#[test]
fn rejects_s3_uri_as_bucket_name() {
    let err = parse_args(
        [
            "--hypothesis-state-file",
            "/tmp/state.json",
            "--output-s3-bucket",
            "s3://research-bucket",
            "--output-s3-prefix",
            "recomposition/",
        ]
        .into_iter()
        .map(str::to_owned),
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("--output-s3-bucket"));
    assert!(err.contains("S3 URI"));
}

#[test]
fn rejects_unsafe_s3_bucket_names() {
    for value in [
        " research-bucket",
        "research bucket",
        "research/bucket",
        "nangman-crypto-dev-research-<account-suffix>",
    ] {
        let err = parse_args(
            [
                "--hypothesis-state-file",
                "/tmp/state.json",
                "--output-s3-bucket",
                value,
                "--output-s3-prefix",
                "recomposition/",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap_err()
        .to_string();

        assert!(
            err.contains("--output-s3-bucket"),
            "bucket={value} err={err}"
        );
    }
}
