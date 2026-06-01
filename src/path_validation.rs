use std::path::{Component, Path};

pub(crate) fn validate_unambiguous_absolute_path(path: &Path, label: &str) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(format!("{label} must be an absolute path"));
    }
    let text = path.as_os_str().to_string_lossy();
    if text
        .split(['/', '\\'])
        .any(|segment| matches!(segment, "." | ".."))
        || path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("{label} must not contain relative path components"));
    }
    if text.chars().any(char::is_control) {
        return Err(format!("{label} must not contain control characters"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_or_ambiguous_absolute_paths() {
        for path in [
            Path::new("relative-path"),
            Path::new("/tmp/../recomposition"),
            Path::new("/tmp/./recomposition"),
            Path::new("/tmp/recomposition\nout"),
        ] {
            let error = validate_unambiguous_absolute_path(path, "test path")
                .expect_err("unsafe path must be rejected");
            assert!(
                error.contains("test path"),
                "expected label in error for {path:?}, got {error}"
            );
        }
    }

    #[test]
    fn accepts_absolute_path_without_relative_components() {
        validate_unambiguous_absolute_path(Path::new("/tmp/recomposition"), "test path")
            .expect("absolute path is accepted");
    }
}
