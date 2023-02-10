use std::env;

use ara_forest::config::Config;
use ara_forest::error::Error;
use ara_forest::source::SourcesBuilder;

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[test]
fn test_collecting_files_in_project_a() {
    let root = format!("{MANIFEST_DIR}/tests/examples/project-a");
    let config = Config::new(root).with_source("src").with_definitions(vec![
        format!("vendor/std-bar/definitions"),
        format!("vendor/std-foo/definitions"),
    ]);
    let sources = SourcesBuilder::new(&config).build().unwrap();

    assert_eq!(sources.len(), 6);
}

#[test]
fn test_trying_to_collect_files_in_a_fake_directory() {
    let root = format!("{MANIFEST_DIR}/tests/examples/project-fake");

    let config = Config::new(root).with_source("src");
    let result = SourcesBuilder::new(&config).build();

    assert!(
        matches!(result, Err(Error::InvalidPath(_))),
        "Expected an InvalidSource error, but got something else",
    );
}

#[test]
fn test_trying_to_collect_files_in_a_invalid_path() {
    let root = format!("{MANIFEST_DIR}/tests/examples/project-a");

    let config = Config::new(root).with_source("src/foo.ara");
    let result = SourcesBuilder::new(&config).build();

    assert!(
        matches!(result, Err(Error::InvalidPath(_))),
        "Expected an InvalidSource error, but got something else",
    );
}
