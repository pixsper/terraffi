//! The shape of the public error type.

use terraffi_gen::{TerraffiError, TerraffiGeneratorBuilder};

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(format!(
        "{}/tests/fixtures/unresolved",
        env!("CARGO_MANIFEST_DIR")
    ))
}

/// Callers should be able to react to the failure, not just print it.
#[test]
fn unresolved_types_can_be_inspected_programmatically() {
    let err = TerraffiGeneratorBuilder::new()
        .disable_scan_dependencies()
        .export_public_types()
        .build(fixture_path())
        .generate()
        .expect_err("fixture has unresolved types");

    match err {
        TerraffiError::UnresolvedTypes(e) => {
            assert!(
                e.unresolved.iter().any(|u| u.type_name == "String"),
                "String missing from structured data"
            );
            assert!(
                e.unresolved
                    .iter()
                    .any(|u| u.location.contains("takes_string")),
                "location missing from structured data"
            );
        }
        other => panic!("expected UnresolvedTypes, got {other:?}"),
    }
}

#[test]
fn missing_crate_dir_names_the_path() {
    let err = TerraffiGeneratorBuilder::new()
        .build("/definitely/not/a/real/directory")
        .generate()
        .expect_err("missing directory should fail");

    match &err {
        TerraffiError::CrateDirNotFound(path) => {
            assert!(path.to_string_lossy().contains("not/a/real"));
        }
        other => panic!("expected CrateDirNotFound, got {other:?}"),
    }
    assert!(err.to_string().contains("is not a directory"));
}

/// Build scripts commonly return `Box<dyn Error>`, so `?` must still work.
#[test]
fn converts_into_box_dyn_error() {
    fn like_a_build_script() -> Result<String, Box<dyn std::error::Error>> {
        let header = TerraffiGeneratorBuilder::new()
            .build(format!(
                "{}/../terraffi_testlib",
                env!("CARGO_MANIFEST_DIR")
            ))
            .generate()?;
        Ok(header)
    }

    assert!(!like_a_build_script().expect("testlib generates").is_empty());
}

/// Errors are routinely moved across threads by build tooling.
#[test]
fn error_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<TerraffiError>();
}

/// `source()` should chain to the underlying cause where there is one.
#[test]
fn unresolved_types_error_is_a_source() {
    use std::error::Error;

    let err = TerraffiGeneratorBuilder::new()
        .disable_scan_dependencies()
        .export_public_types()
        .build(fixture_path())
        .generate()
        .expect_err("fixture has unresolved types");

    assert!(err.source().is_some(), "expected a chained source");
}
