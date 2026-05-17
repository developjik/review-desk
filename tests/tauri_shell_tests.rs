use reviewdesk::app_core::allowed_ipc_commands;
use serde_json::Value;
use std::fs;

#[test]
fn tauri_manifest_and_capability_files_exist() {
    for path in [
        "src-tauri/Cargo.toml",
        "src-tauri/build.rs",
        "src-tauri/tauri.conf.json",
        "src-tauri/capabilities/main.json",
        "src-tauri/src/main.rs",
        "src-tauri/src/commands.rs",
    ] {
        assert!(fs::metadata(path).is_ok(), "missing {path}");
    }
}

#[test]
fn tauri_manifest_is_release_ready_for_0_1_0() {
    let manifest = fs::read_to_string("src-tauri/tauri.conf.json").expect("tauri config exists");
    let manifest: Value = serde_json::from_str(&manifest).expect("tauri config is valid json");

    assert_eq!(manifest["version"], "0.1.0");
    assert_eq!(manifest["app"]["windows"][0]["minWidth"], 800);
    assert_eq!(manifest["bundle"]["active"], true);
    assert!(
        manifest["bundle"]["targets"]
            .as_array()
            .is_some_and(|targets| targets.iter().any(|target| target == "app")),
        "bundle targets must include app"
    );
    assert!(
        manifest["bundle"]["icon"]
            .as_array()
            .is_some_and(|icons| icons.iter().any(|icon| icon == "icons/icon.png")),
        "bundle icon metadata must include icons/icon.png"
    );
}

#[test]
fn tauri_build_manifest_lists_the_same_safe_commands_as_app_core() {
    let build_rs = fs::read_to_string("src-tauri/build.rs").expect("build.rs exists");

    for command in allowed_ipc_commands() {
        assert!(
            build_rs.contains(command.name),
            "missing command {} in build.rs allowlist",
            command.name
        );
    }

    for forbidden in [
        "get_token",
        "export_token",
        "arbitrary_shell",
        "arbitrary_http",
        "read_file",
        "write_file",
    ] {
        assert!(
            !build_rs.contains(forbidden),
            "forbidden command appears in build.rs: {forbidden}"
        );
    }
}

#[test]
fn tauri_allowlist_contains_run_draft_inline_publish_commands() {
    let build_rs = fs::read_to_string("src-tauri/build.rs").expect("build.rs exists");
    for command in [
        "list_analysis_runs",
        "start_analysis_run",
        "read_analysis_run",
        "cancel_analysis_run",
        "archive_analysis_run",
        "create_draft_from_run",
        "save_review_draft",
        "read_review_draft",
        "list_review_drafts",
        "mark_active_draft",
        "validate_inline_comments",
    ] {
        assert!(build_rs.contains(command), "missing command {command}");
    }
}

#[test]
fn tauri_capability_allows_all_safe_reviewdesk_commands() {
    let capability =
        fs::read_to_string("src-tauri/capabilities/main.json").expect("capability exists");

    for command in allowed_ipc_commands() {
        let permission = format!("allow-{}", command.name.replace('_', "-"));
        assert!(
            capability.contains(&format!("\"{permission}\"")),
            "missing capability permission {permission}"
        );
    }
}

#[test]
fn tauri_capability_does_not_enable_broad_fs_shell_or_http_permissions() {
    let capability =
        fs::read_to_string("src-tauri/capabilities/main.json").expect("capability exists");
    let lowered = capability.to_ascii_lowercase();

    for forbidden in [
        "fs:",
        "shell:",
        "http:",
        "process:",
        "upload:",
        "websocket:",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "broad permission appears in capability: {forbidden}"
        );
    }

    assert!(lowered.contains("opener:allow-open-url"));
}
