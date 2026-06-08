pub(super) fn folder() -> SidebarFolder {
    SidebarFolder {
        key: "/repo".into(),
        path: "/repo".into(),
        label: "repo".into(),
        updated_at: 0,
        threads: Vec::new(),
    }
}

pub(super) fn codex_thread() -> CodexThreadRef {
    CodexThreadRef {
        thread_id: "sid-1".into(),
        cwd: PathBuf::from("/repo"),
        updated_at: 42,
        rollout_path: PathBuf::from("/repo/.codex/sid-1.jsonl"),
        title: Some("upstream title".into()),
        first_user_message: Some("old first prompt".into()),
        source: None,
        archived: false,
    }
}

pub(super) fn cached_snapshot(question: &str, answer: Option<&str>) -> SessionCacheSnapshot {
    SessionCacheSnapshot {
        agent_session_id: "sid-1".into(),
        transcript_path: Some("/repo/.codex/sid-1.jsonl".into()),
        recent_turns: vec![PreviewTurn {
            question: question.into(),
            answer: answer.map(str::to_string),
        }]
        .into(),
        last_user_prompt: Some(question.into()),
        last_assistant_message: answer.map(str::to_string),
        state: SessionCacheState::Cached,
    }
}

pub(super) fn live_codex_thread_without_prompt() -> Arc<SidebarThread> {
    Arc::new(SidebarThread {
        key: "live:%1".into(),
        folder_key: "/repo".into(),
        working_dir: "/repo".into(),
        folder_label: "repo".into(),
        agent_type: AgentType::Codex,
        session_id: Some("sid-1".into()),
        transcript_path: Some("/repo/.codex/sid-1.jsonl".into()),
        session_provider_name: None,
        title: "live".into(),
        upstream_title: None,
        generated_title: None,
        subtitle: None,
        title_override: None,
        note: None,
        share_url: None,
        cost: None,
        token_summary: None,
        tags: Vec::new(),
        pinned: false,
        updated_at: 1,
        sort_updated_at: 1,
        live_pane_id: Some("%1".into()),
        live_location: None,
        pid: None,
        git_info: None,
        state: AgentState::Idle,
        is_active: false,
        cached_preview_turns: Default::default(),
        session_cache_state: None,
        last_user_prompt: None,
        last_assistant_message: None,
        has_unread_stop: false,
        archived: false,
        deleted: false,
    })
}

pub(super) fn generated_title_meta(title_override: Option<&str>) -> ThreadMeta {
    ThreadMeta {
        title_override: title_override.map(str::to_string),
        generated_title: Some("Generated title".into()),
        generated_turn_count: Some(9),
        generated_updated_at: Some(123),
        deleted: false,
        deleted_at: None,
        note: None,
        pinned: false,
        tags: Vec::new(),
        updated_at: 123,
    }
}
