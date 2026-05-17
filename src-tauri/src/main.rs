mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(commands::ReviewDeskTauriState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_status,
            commands::get_ai_connection_status,
            commands::get_codex_bridge_status,
            commands::start_github_oauth,
            commands::poll_github_oauth,
            commands::cancel_github_oauth,
            commands::logout_github,
            commands::refresh_github_auth_status,
            commands::start_codex_chatgpt_login,
            commands::poll_codex_chatgpt_login,
            commands::cancel_codex_chatgpt_login,
            commands::read_codex_account,
            commands::list_ai_models,
            commands::select_ai_model,
            commands::read_codex_rate_limits,
            commands::logout_codex_chatgpt,
            commands::refresh_ai_account_status,
            commands::start_chatgpt_oauth,
            commands::poll_chatgpt_oauth,
            commands::list_repositories,
            commands::load_review_queue,
            commands::collect_pr_context,
            commands::set_private_diff_consent,
            commands::start_agent_run,
            commands::read_agent_run,
            commands::cancel_agent_run,
            commands::generate_review_draft,
            commands::create_draft_from_run,
            commands::save_review_draft,
            commands::read_review_draft,
            commands::list_review_drafts,
            commands::mark_active_draft,
            commands::save_draft,
            commands::prepare_submit_review,
            commands::confirm_submit_review,
            commands::open_external_url,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ReviewDesk Tauri shell");
}
