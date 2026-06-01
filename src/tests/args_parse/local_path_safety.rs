#[test]
fn rejects_local_paths_with_relative_components() {
    for (flag, value) in [
        ("--hypothesis-state-file", "/tmp/../state.json"),
        ("--output-dir", "/tmp/./recomposition-out"),
    ] {
        let err = parse_args(
            [
                "--hypothesis-state-file",
                "/tmp/state.json",
                "--output-dir",
                "/tmp/out",
                flag,
                value,
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains(flag), "expected {flag} in {err}");
        assert!(err.contains("relative path components"));
    }
}
