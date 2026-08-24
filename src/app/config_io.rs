use super::App;

impl App {
    pub(super) fn update_config_file<F>(&mut self, error_context: &str, update: F) -> bool
    where
        F: FnOnce(&str) -> String,
    {
        #[cfg(test)]
        if std::env::var_os(crate::config::CONFIG_PATH_ENV_VAR).is_none() {
            return false;
        }

        let path = crate::config::config_path();
        if let Some(parent) = path.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                crate::logging::config_write_failed(&path, error_context, &err.to_string());
                self.state.config_diagnostic =
                    Some(format!("failed to save {error_context}: {err}"));
                self.config_diagnostic_deadline =
                    Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
                return false;
            }
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let new_content = update(&content);
        if let Err(err) = std::fs::write(&path, new_content) {
            crate::logging::config_write_failed(&path, error_context, &err.to_string());
            self.state.config_diagnostic = Some(format!("failed to save {error_context}: {err}"));
            self.config_diagnostic_deadline =
                Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
            return false;
        }

        true
    }

    pub(super) fn mark_onboarding_complete(&mut self) {
        self.update_config_file("onboarding setting", |content| {
            crate::config::upsert_top_level_bool(content, "onboarding", false)
        });
    }

    pub(super) fn save_theme(&mut self, name: &str) {
        if self.update_config_file("theme", |content| {
            let content = crate::config::upsert_section_value(
                content,
                "theme",
                "name",
                &format!("\"{name}\""),
            );
            crate::config::upsert_section_bool(&content, "theme", "auto_switch", false)
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_status_indicators(&mut self, style: crate::config::StatusIndicatorStyle) {
        if self.update_config_file("status indicators", |content| {
            crate::config::upsert_section_value(
                content,
                "ui",
                "status_indicators",
                &format!("\"{}\"", style.as_str()),
            )
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_sound(&mut self, enabled: bool) {
        if self.update_config_file("sound setting", |content| {
            crate::config::upsert_section_bool(content, "ui.sound", "enabled", enabled)
        }) {
            self.apply_config_from_disk(false);
        }
    }

    /// Persist the Projects-tab default agent picked in the new-chat selector
    /// so later plain "+" clicks (and restarts) keep using it.
    /// Persist the send picker's pinned devices.
    ///
    /// Written as a TOML array of the tailnet DNS names, in the reader's order.
    /// The config file is the right home rather than session state: a pin is a
    /// standing preference that should survive a session being discarded.
    pub(super) fn save_tailscale_pinned_devices(&mut self, pinned: &[String]) {
        let quoted = pinned
            .iter()
            // A tailnet name cannot contain a quote or a backslash, but the
            // value is written into a config file the user also edits by hand,
            // and building TOML by concatenation without escaping is how a
            // config file stops parsing.
            .map(|target| format!("\"{}\"", target.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(", ");
        if self.update_config_file("tailscale pinned devices", |content| {
            crate::config::upsert_section_value(
                content,
                "tailscale",
                "pinned_devices",
                &format!("[{quoted}]"),
            )
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_default_chat_agent(&mut self, agent: &str) {
        if self.update_config_file("default chat agent", |content| {
            crate::config::upsert_section_value(
                content,
                "projects",
                "default_chat_agent",
                &format!("\"{agent}\""),
            )
        }) {
            self.apply_config_from_disk(false);
        }
    }

    /// Persist the footer "actives" filter of the Projects tab so restarts
    /// keep the chosen visibility.
    pub(super) fn save_projects_actives_only(&mut self, enabled: bool) {
        if self.update_config_file("projects actives filter", |content| {
            crate::config::upsert_section_bool(content, "projects", "actives_only", enabled)
        }) {
            self.apply_config_from_disk(false);
        }
    }

    /// Persist the Spaces tab's focus filter as the starting point for new
    /// clients. The live toggle stays this display's own (TP-FOCUS-SW-05);
    /// what lands on disk is only where the next screen begins.
    pub(super) fn save_spaces_focus_only(&mut self, enabled: bool) {
        if self.update_config_file("spaces focus filter", |content| {
            crate::config::upsert_section_bool(content, "ui.sidebar.spaces", "focus_only", enabled)
        }) {
            self.apply_config_from_disk(false);
        }
    }

    /// Persist the preview placement mode picked in settings. Only available
    /// modes reach this point (the "soon" rows are inert in the UI).
    pub(super) fn save_preview_placement(&mut self, placement: crate::config::PreviewPlacement) {
        if self.update_config_file("preview placement", |content| {
            crate::config::upsert_section_value(
                content,
                "preview",
                "placement",
                &format!("\"{}\"", placement.config_value()),
            )
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_toast_delivery(&mut self, delivery: crate::config::ToastDelivery) {
        let value = match delivery {
            crate::config::ToastDelivery::Off => "\"off\"",
            crate::config::ToastDelivery::Herdr => "\"herdr\"",
            crate::config::ToastDelivery::Terminal => "\"terminal\"",
            crate::config::ToastDelivery::System => "\"system\"",
        };
        if self.update_config_file("toast setting", |content| {
            let content =
                crate::config::upsert_section_value(content, "ui.toast", "delivery", value);
            crate::config::remove_section_key(&content, "ui.toast", "enabled")
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_agent_border_labels(&mut self, enabled: bool) {
        if self.update_config_file("agent border labels", |content| {
            crate::config::upsert_section_bool(
                content,
                "ui",
                "show_agent_labels_on_pane_borders",
                enabled,
            )
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_pane_history_persistence(&mut self, enabled: bool) {
        if self.update_config_file("pane screen history", |content| {
            crate::config::upsert_section_bool(content, "experimental", "pane_history", enabled)
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_switch_ascii_input_source_in_prefix(&mut self, enabled: bool) {
        if self.update_config_file("prefix ascii input source", |content| {
            crate::config::upsert_section_bool(
                content,
                "experimental",
                "switch_ascii_input_source_in_prefix",
                enabled,
            )
        }) {
            self.apply_config_from_disk(false);
        }
    }

    pub(super) fn save_agent_panel_sort(&mut self, sort: crate::app::state::AgentPanelSort) {
        let value = match sort {
            crate::app::state::AgentPanelSort::Spaces => {
                crate::config::AgentPanelSortConfig::Spaces.as_str()
            }
            crate::app::state::AgentPanelSort::Priority => {
                crate::config::AgentPanelSortConfig::Priority.as_str()
            }
        };
        if self.update_config_file("agent panel sort", |content| {
            crate::config::upsert_section_value(
                content,
                "ui",
                "agent_panel_sort",
                &format!("\"{value}\""),
            )
        }) {
            self.apply_config_from_disk(false);
        }
    }
}
