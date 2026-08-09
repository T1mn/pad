use super::activity::merge_or_insert_thread;
use super::history_codex::build_codex_history_entry;
use super::meta::apply_thread_meta;
use crate::codex_state::CodexThreadRef;
use crate::model::{AgentState, AgentType, PreviewTurn, SessionCacheState};
use crate::session_cache::SessionCacheSnapshot;
use crate::sidebar::model::{SidebarFolder, SidebarThread};
use crate::thread_meta::ThreadMeta;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) mod activity {
    use super::support::{cached_snapshot, codex_thread, folder, live_codex_thread_without_prompt};
    use super::*;

    pub(crate) fn merge_or_insert_preserves_history_prompt_when_live_thread_lacks_one() {
        let mut threads = vec![live_codex_thread_without_prompt()];
        let snapshot = cached_snapshot("newest prompt", None);
        let history = build_codex_history_entry(&folder(), &codex_thread(), Some(&snapshot), false);

        merge_or_insert_thread(&mut threads, history, &[], &HashMap::new());

        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].subtitle.as_deref(), Some("newest prompt"));
        assert_eq!(
            threads[0].last_user_prompt.as_deref(),
            Some("newest prompt")
        );
        assert_eq!(threads[0].cached_preview_turns.len(), 1);
        assert_eq!(
            threads[0].session_cache_state,
            Some(SessionCacheState::Cached)
        );
    }

    pub(crate) fn runtime_sort_activity_updates_history_order() {
        let mut threads = Vec::new();
        let history = build_codex_history_entry(&folder(), &codex_thread(), None, false);
        let runtime = HashMap::from([(String::from("codex:path:/repo/.codex/sid-1.jsonl"), 120)]);

        merge_or_insert_thread(&mut threads, history, &[], &runtime);

        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].sort_updated_at, 120);
    }
}

pub(crate) mod history {
    use super::support::{cached_snapshot, codex_thread, folder};
    use super::*;

    pub(crate) fn codex_history_prefers_session_cache_prompt_for_subtitle() {
        let snapshot = cached_snapshot("newest prompt", Some("answer"));
        let thread = build_codex_history_entry(&folder(), &codex_thread(), Some(&snapshot), false);

        assert_eq!(thread.subtitle.as_deref(), Some("newest prompt"));
        assert_eq!(thread.last_user_prompt.as_deref(), Some("newest prompt"));
        assert_eq!(thread.cached_preview_turns.len(), 1);
    }

    pub(crate) fn active_view_history_entries_do_not_sort_by_updated_at_without_explicit_activity()
    {
        let thread = build_codex_history_entry(&folder(), &codex_thread(), None, false);
        assert_eq!(thread.updated_at, 42);
        assert_eq!(thread.sort_updated_at, 0);
    }

    pub(crate) fn archived_view_history_entries_keep_updated_at_sorting() {
        let thread = build_codex_history_entry(&folder(), &codex_thread(), None, true);
        assert_eq!(thread.updated_at, 42);
        assert_eq!(thread.sort_updated_at, 42);
    }
}

pub(crate) mod meta {
    use super::support::{codex_thread, folder, generated_title_meta};
    use super::*;

    pub(crate) fn manual_title_override_wins_over_generated_summary_for_title() {
        let mut thread = build_codex_history_entry(&folder(), &codex_thread(), None, false);
        apply_thread_meta(&mut thread, &generated_title_meta(Some("Manual title")));

        assert_eq!(thread.title, "Manual title");
        assert_eq!(thread.generated_title.as_deref(), Some("Generated title"));
    }

    pub(crate) fn generated_summary_does_not_replace_session_title() {
        let mut thread = build_codex_history_entry(&folder(), &codex_thread(), None, false);
        apply_thread_meta(&mut thread, &generated_title_meta(None));

        assert_eq!(thread.title, "upstream title");
        assert_eq!(thread.generated_title.as_deref(), Some("Generated title"));
    }
}

mod support {
    use super::*;
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
}
