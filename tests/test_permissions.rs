use acp_cli::bridge::{PermissionKind, PermissionOption, PermissionOutcome, ToolCallInfo};
use acp_cli::client::permissions::{PermissionMode, resolve_permission};

fn make_tool(name: &str) -> ToolCallInfo {
    ToolCallInfo {
        name: name.to_string(),
        description: None,
    }
}

fn make_options() -> Vec<PermissionOption> {
    vec![
        PermissionOption {
            option_id: "deny-1".to_string(),
            name: "Deny".to_string(),
            kind: PermissionKind::Deny,
        },
        PermissionOption {
            option_id: "allow-1".to_string(),
            name: "Allow".to_string(),
            kind: PermissionKind::Allow,
        },
    ]
}

#[test]
fn approve_all_selects_first_allow() {
    let tool = make_tool("Bash");
    let options = make_options();
    let outcome = resolve_permission(&tool, &options, &PermissionMode::ApproveAll);

    match outcome {
        PermissionOutcome::Selected { option_id } => {
            assert_eq!(option_id, "allow-1");
        }
        PermissionOutcome::Cancelled => panic!("expected Selected, got Cancelled"),
    }
}

#[test]
fn approve_reads_denies_write_tools() {
    let options = make_options();

    for tool_name in &["Edit", "Bash", "Write"] {
        let tool = make_tool(tool_name);
        let outcome = resolve_permission(&tool, &options, &PermissionMode::ApproveReads);

        assert!(
            matches!(outcome, PermissionOutcome::Cancelled),
            "expected Cancelled for write tool '{}', got Selected",
            tool_name
        );
    }
}

#[test]
fn approve_reads_allows_read_tools() {
    let options = make_options();

    for tool_name in &["Read", "Glob", "Grep"] {
        let tool = make_tool(tool_name);
        let outcome = resolve_permission(&tool, &options, &PermissionMode::ApproveReads);

        match outcome {
            PermissionOutcome::Selected { option_id } => {
                assert_eq!(
                    option_id, "allow-1",
                    "wrong option selected for '{}'",
                    tool_name
                );
            }
            PermissionOutcome::Cancelled => {
                panic!(
                    "expected Selected for read tool '{}', got Cancelled",
                    tool_name
                );
            }
        }
    }
}

#[test]
fn deny_all_cancels() {
    let tool = make_tool("Read");
    let options = make_options();
    let outcome = resolve_permission(&tool, &options, &PermissionMode::DenyAll);

    assert!(
        matches!(outcome, PermissionOutcome::Cancelled),
        "expected Cancelled for DenyAll mode"
    );
}

#[test]
fn approve_all_with_no_allow_option_cancels() {
    let tool = make_tool("Bash");
    let options = vec![PermissionOption {
        option_id: "deny-only".to_string(),
        name: "Deny".to_string(),
        kind: PermissionKind::Deny,
    }];
    let outcome = resolve_permission(&tool, &options, &PermissionMode::ApproveAll);

    assert!(
        matches!(outcome, PermissionOutcome::Cancelled),
        "expected Cancelled when no Allow option exists"
    );
}
