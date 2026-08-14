use crate::api::schema::{ResponseResult, SessionSnapshot};
use crate::app::App;

use super::responses::{encode_error, encode_success};

impl App {
    pub(super) fn handle_session_snapshot(&mut self, id: String) -> String {
        encode_success(
            id,
            ResponseResult::SessionSnapshot {
                snapshot: Box::new(self.session_snapshot()),
            },
        )
    }

    fn session_snapshot(&self) -> SessionSnapshot {
        let focused_workspace_id = self
            .state
            .active
            .map(|ws_idx| self.public_workspace_id(ws_idx));
        let focused_tab_id = self.state.active.and_then(|ws_idx| {
            let ws = self.state.workspaces.get(ws_idx)?;
            self.public_tab_id(ws_idx, ws.active_tab_index())
        });
        let focused_pane_id = self.state.active.and_then(|ws_idx| {
            let ws = self.state.workspaces.get(ws_idx)?;
            self.public_pane_id(ws_idx, ws.focused_pane_id()?)
        });

        let mut workspaces = Vec::new();
        let mut tabs = Vec::new();
        let mut layouts = Vec::new();
        for (ws_idx, ws) in self.state.workspaces.iter().enumerate() {
            workspaces.push(self.workspace_info(ws_idx));
            for tab_idx in 0..ws.tabs.len() {
                if let Some(tab) = self.tab_info(ws_idx, tab_idx) {
                    tabs.push(tab);
                }
                if let Some(layout) = self.pane_layout_snapshot(ws_idx, tab_idx) {
                    layouts.push(layout);
                }
            }
        }

        SessionSnapshot {
            version: crate::build_info::version(),
            protocol: crate::protocol::PROTOCOL_VERSION,
            focused_workspace_id,
            focused_tab_id,
            focused_pane_id,
            workspaces,
            tabs,
            panes: self.collect_panes_for_workspace(None).unwrap_or_default(),
            layouts,
            agents: self.collect_agent_infos(),
            closed_agents: self.collect_closed_agent_infos(),
        }
    }

    /// Revival rides the protocol, not a private TUI gesture: the same method
    /// every client asks through, and a refusal answers with a stable code
    /// and a readable reason (a refusal nobody can read is a silent bug).
    pub(super) fn handle_closed_agent_revive(
        &mut self,
        id: String,
        params: crate::api::schema::ClosedAgentReviveParams,
    ) -> String {
        match self.revive_closed_agent(&params.agent_id) {
            Ok(()) => encode_success(id, ResponseResult::Ok {}),
            Err(refusal) => encode_error(id, refusal.code(), refusal.message()),
        }
    }

    fn collect_closed_agent_infos(&self) -> Vec<crate::api::schema::ClosedAgentInfo> {
        self.state
            .closed_agents
            .entries()
            .map(|record| crate::api::schema::ClosedAgentInfo {
                agent_id: record.agent_id.clone(),
                label: record.label.clone(),
                cwd: record.cwd.as_ref().map(|p| p.display().to_string()),
                workspace_id: record.workspace_key.clone(),
                session_id: record.session_value().map(str::to_string),
                closed_at: record.closed_at,
                reviving: record.revival == crate::app::closed_agents::RevivalState::Reviving,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::api::schema::{EmptyParams, Method, ResponseResult, SuccessResponse};
    use crate::{config::Config, workspace::Workspace};

    fn app_with_two_tabs() -> crate::app::App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        let mut workspace = Workspace::test_new("snapshot");
        workspace.test_add_tab(None);
        app.state.workspaces = vec![workspace];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app
    }

    fn ghost(id: &str, closed_at: u64) -> crate::app::closed_agents::ClosedAgentRecord {
        crate::app::closed_agents::ClosedAgentRecord {
            agent_id: id.to_string(),
            label: format!("Claude — {id}"),
            cwd: Some(std::path::PathBuf::from(format!("/tmp/{id}"))),
            workspace_key: Some("ws-main".into()),
            session: crate::agent_resume::AgentSessionRef::id(format!("session-{id}")).map(
                |session_ref| crate::agent_resume::PersistedAgentSession {
                    source: "herdr:claude".into(),
                    agent: "claude".into(),
                    session_ref,
                },
            ),
            closed_at,
            revival: crate::app::closed_agents::RevivalState::Dormant,
        }
    }

    // TP-AGPANEL-15: closed agents travel the same protocol surface as live
    // ones. The graveyard is server truth, not a TUI ornament — every client
    // (TUI, PWA) must see one list, so it rides the session snapshot, newest
    // first, with the in-flight revival state on each row.
    #[test]
    fn the_session_snapshot_carries_closed_agents_newest_first() {
        let mut app = app_with_two_tabs();
        app.state.closed_agents.record_closed(ghost("ghost-a", 1));
        app.state.closed_agents.record_closed(ghost("ghost-b", 2));
        assert!(app.state.closed_agents.try_begin_revival("ghost-b"));

        let response = app.handle_api_request(crate::api::schema::Request {
            id: "req_ghosts".into(),
            method: Method::SessionSnapshot(EmptyParams::default()),
        });
        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::SessionSnapshot { snapshot } = success.result else {
            panic!("expected session snapshot response");
        };

        let ghosts = &snapshot.closed_agents;
        assert_eq!(ghosts.len(), 2);
        assert_eq!(ghosts[0].agent_id, "ghost-b");
        assert_eq!(ghosts[0].closed_at, 2);
        assert!(
            ghosts[0].reviving,
            "in-flight revival state travels the wire"
        );
        assert_eq!(ghosts[0].cwd.as_deref(), Some("/tmp/ghost-b"));
        assert_eq!(ghosts[0].workspace_id.as_deref(), Some("ws-main"));
        assert_eq!(ghosts[0].session_id.as_deref(), Some("session-ghost-b"));
        assert_eq!(ghosts[1].agent_id, "ghost-a");
        assert!(!ghosts[1].reviving);
    }

    // TP-AGPANEL-16: an empty graveyard leaves no trace on the wire. The field
    // is default + skip-if-empty, so old clients parse new servers and new
    // clients parse old servers — compatibility by construction, not hope.
    #[test]
    fn an_empty_graveyard_is_absent_from_the_snapshot_wire() {
        let mut app = app_with_two_tabs();
        let response = app.handle_api_request(crate::api::schema::Request {
            id: "req_quiet".into(),
            method: Method::SessionSnapshot(EmptyParams::default()),
        });
        assert!(
            !response.contains("closed_agents"),
            "an empty list must not appear on the wire: {response}"
        );
        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::SessionSnapshot { snapshot } = success.result else {
            panic!("expected session snapshot response");
        };
        assert!(snapshot.closed_agents.is_empty());
    }

    // TP-AGPANEL-24: revival is a protocol method, not a private TUI gesture —
    // the one road every client (TUI today, PWA tomorrow) asks through. A
    // refusal answers with a stable code; a revival whose conversation is
    // already wired to a live tab lands as plain success.
    #[test]
    fn revival_travels_the_api_road_with_visible_refusals() {
        let mut app = app_with_two_tabs();

        let response = app.handle_api_request(crate::api::schema::Request {
            id: "req_revive_unknown".into(),
            method: Method::ClosedAgentRevive(crate::api::schema::ClosedAgentReviveParams {
                agent_id: "nobody".into(),
            }),
        });
        assert!(
            response.contains("unknown_closed_agent"),
            "a refusal names its reason: {response}"
        );

        app.state.workspaces[0].tabs[1].resumed_session_id = Some("session-ghost-a".into());
        let mut record = ghost("ghost-a", 1);
        record.cwd = Some(std::path::PathBuf::from("/tmp"));
        app.state.closed_agents.record_closed(record);

        let response = app.handle_api_request(crate::api::schema::Request {
            id: "req_revive".into(),
            method: Method::ClosedAgentRevive(crate::api::schema::ClosedAgentReviveParams {
                agent_id: "ghost-a".into(),
            }),
        });
        assert!(
            !response.contains("\"error\""),
            "a wired-tab revival lands as success: {response}"
        );
        assert!(
            app.state.closed_agents.entries().next().is_none(),
            "the landed revival left the graveyard"
        );
    }

    #[test]
    fn session_snapshot_bootstraps_runtime_resources() {
        let mut app = app_with_two_tabs();
        let response = app.handle_api_request(crate::api::schema::Request {
            id: "req_snapshot".into(),
            method: Method::SessionSnapshot(EmptyParams::default()),
        });

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        let ResponseResult::SessionSnapshot { snapshot } = success.result else {
            panic!("expected session snapshot response");
        };
        assert_eq!(success.id, "req_snapshot");
        assert_eq!(snapshot.workspaces.len(), 1);
        assert_eq!(snapshot.tabs.len(), 2);
        assert_eq!(snapshot.panes.len(), 2);
        assert_eq!(snapshot.layouts.len(), 2);
        assert_eq!(
            snapshot.focused_workspace_id.as_deref(),
            Some(snapshot.workspaces[0].workspace_id.as_str())
        );
        assert_eq!(
            snapshot.focused_tab_id.as_deref(),
            Some(snapshot.tabs[0].tab_id.as_str())
        );
        assert_eq!(
            snapshot.focused_pane_id.as_deref(),
            Some(snapshot.panes[0].pane_id.as_str())
        );
    }
}
