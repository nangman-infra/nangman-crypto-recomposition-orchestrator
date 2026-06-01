use crate::DEFAULT_AWS_REGION;
use crate::types::{Args, S3InputArgs, S3OutputArgs};

pub(super) fn default_args() -> Args {
    Args {
        hypothesis_state_file: None,
        input_s3: None,
        changed_trigger: Vec::new(),
        output_dir: None,
        output_s3: None,
        now_ms: None,
    }
}

pub(super) fn default_input_s3() -> S3InputArgs {
    S3InputArgs {
        bucket: String::new(),
        region: DEFAULT_AWS_REGION.to_owned(),
        prefix: String::new(),
        profile: None,
        max_keys: 1_000,
    }
}

pub(super) fn default_output_s3() -> S3OutputArgs {
    S3OutputArgs {
        bucket: String::new(),
        region: DEFAULT_AWS_REGION.to_owned(),
        prefix: String::new(),
        profile: None,
    }
}
