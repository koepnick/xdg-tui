//! Application-level state: which tab is active, list selections, and a
//! transient status line for showing the result of actions.

use crate::desktop_entries::{self, AppEntry};
use crate::mime::{self, MimeAssociation};
use crate::xdg_base::BaseDirs;
use anyhow::Result;
use ratatui::widgets::ListState;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    BaseDirs,
    MimeTypes,
    Applications,
}

impl Tab {
    pub const ALL: [Tab; 3] = [Tab::BaseDirs, Tab::MimeTypes, Tab::Applications];

    pub fn title(self) -> &'static str {
        match self {
            Tab::BaseDirs => "Base Directories",
            Tab::MimeTypes => "MIME Defaults",
            Tab::Applications => "Applications",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Tab::BaseDirs => Tab::MimeTypes,
            Tab::MimeTypes => Tab::Applications,
            Tab::Applications => Tab::BaseDirs,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Tab::BaseDirs => Tab::Applications,
            Tab::MimeTypes => Tab::BaseDirs,
            Tab::Applications => Tab::MimeTypes,
        }
    }
}

pub struct Status {
    pub message: String,
    pub is_error: bool,
    pub expires: Instant,
}

impl Status {
    pub fn info(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            is_error: false,
            expires: Instant::now() + Duration::from_secs(4),
        }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            is_error: true,
            expires: Instant::now() + Duration::from_secs(6),
        }
    }
    pub fn is_live(&self) -> bool {
        Instant::now() < self.expires
    }
}

pub struct App {
    pub tab: Tab,
    pub should_quit: bool,

    pub base_dirs: BaseDirs,
    pub base_state: ListState,

    pub mime_assocs: Vec<MimeAssociation>,
    pub mime_state: ListState,
    pub mime_filter: String,
    pub mime_filter_active: bool,

    pub apps: Vec<AppEntry>,
    pub app_state: ListState,
    pub app_filter: String,
    pub app_filter_active: bool,

    pub status: Option<Status>,
}

impl App {
    pub fn new() -> Result<Self> {
        let base_dirs = BaseDirs::discover();
        let mime_assocs = mime::read_associations().unwrap_or_default();
        let apps = desktop_entries::discover_apps().unwrap_or_default();

        let mut base_state = ListState::default();
        base_state.select(Some(0));
        let mut mime_state = ListState::default();
        if !mime_assocs.is_empty() {
            mime_state.select(Some(0));
        }
        let mut app_state = ListState::default();
        if !apps.is_empty() {
            app_state.select(Some(0));
        }

        Ok(Self {
            tab: Tab::BaseDirs,
            should_quit: false,
            base_dirs,
            base_state,
            mime_assocs,
            mime_state,
            mime_filter: String::new(),
            mime_filter_active: false,
            apps,
            app_state,
            app_filter: String::new(),
            app_filter_active: false,
            status: None,
        })
    }

    pub fn filtered_mime_indices(&self) -> Vec<usize> {
        if self.mime_filter.is_empty() {
            return (0..self.mime_assocs.len()).collect();
        }
        let q = self.mime_filter.to_lowercase();
        self.mime_assocs
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                m.mime_type.to_lowercase().contains(&q)
                    || m.default_app
                        .as_deref()
                        .map(|s| s.to_lowercase().contains(&q))
                        .unwrap_or(false)
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn filtered_app_indices(&self) -> Vec<usize> {
        if self.app_filter.is_empty() {
            return (0..self.apps.len()).collect();
        }
        let q = self.app_filter.to_lowercase();
        self.apps
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                a.name.to_lowercase().contains(&q)
                    || a.id.to_lowercase().contains(&q)
                    || a.categories.iter().any(|c| c.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn move_selection(&mut self, delta: i32) {
        let (state, len) = match self.tab {
            Tab::BaseDirs => (&mut self.base_state, self.base_dirs.dirs.len()),
            Tab::MimeTypes => {
                let len = self.filtered_mime_indices().len();
                (&mut self.mime_state, len)
            }
            Tab::Applications => {
                let len = self.filtered_app_indices().len();
                (&mut self.app_state, len)
            }
        };
        if len == 0 {
            state.select(None);
            return;
        }
        let current = state.selected().unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(len as i32) as usize;
        state.select(Some(next));
    }

    pub fn refresh(&mut self) {
        self.base_dirs = BaseDirs::discover();
        self.mime_assocs = mime::read_associations().unwrap_or_default();
        self.apps = desktop_entries::discover_apps().unwrap_or_default();
        self.status = Some(Status::info("Reloaded XDG data"));
    }
}
