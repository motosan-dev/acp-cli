use acp_cli::cli::prompt_source::resolve_prompt;

#[test]
fn positional_args_joined_as_prompt() {
    let words: Vec<String> = vec!["hello".into(), "world".into()];
    let result = resolve_prompt(None, &words, true).unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn file_flag_reads_file_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prompt.txt");
    std::fs::write(&path, "file content here\n").unwrap();

    let result = resolve_prompt(Some(path.to_str().unwrap()), &[], true).unwrap();
    assert_eq!(result, "file content here");
}

#[test]
fn file_flag_with_positional_args_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prompt.txt");
    std::fs::write(&path, "content").unwrap();

    let result = resolve_prompt(Some(path.to_str().unwrap()), &["extra".into()], true);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot combine"));
}

#[test]
fn file_flag_dash_with_positional_is_error() {
    let result = resolve_prompt(Some("-"), &["extra".into()], true);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot combine"));
}

#[test]
fn missing_file_is_error() {
    let result = resolve_prompt(Some("/tmp/nonexistent-acp-test-xyz.txt"), &[], true);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("file not found"));
}

#[test]
fn no_file_no_positional_terminal_returns_empty() {
    let result = resolve_prompt(None, &[], true).unwrap();
    assert_eq!(result, "");
}

#[test]
fn file_trims_trailing_newlines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prompt.txt");
    std::fs::write(&path, "hello\n\n\n").unwrap();

    let result = resolve_prompt(Some(path.to_str().unwrap()), &[], true).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn file_preserves_internal_newlines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prompt.txt");
    std::fs::write(&path, "line one\nline two\n").unwrap();

    let result = resolve_prompt(Some(path.to_str().unwrap()), &[], true).unwrap();
    assert_eq!(result, "line one\nline two");
}

#[test]
fn piped_stdin_scenario_no_file_no_positional() {
    // When stdin_is_terminal=false and no positional args, resolve_prompt
    // would attempt to read stdin. We can't easily test actual stdin reading,
    // but we can verify the function signature handles the case.
    // In real usage: echo "hello" | acp-cli claude
    // This test verifies the positional-only path still works when terminal=false
    // but positional args are provided.
    let words: Vec<String> = vec!["from".into(), "args".into()];
    let result = resolve_prompt(None, &words, false).unwrap();
    assert_eq!(result, "from args");
}
