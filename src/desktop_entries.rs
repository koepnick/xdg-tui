//! Desktop entry (.desktop file) discovery and parsing.
//!
//! Walks the standard application directories and parses each .desktop
//! file, exposing the Name, Exec, Icon, Categories, and MIME associations.

use anyhow::Result;
use freedesktop_desktop_entry::{default_paths, DesktopEntry, Iter};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub mime_types: Vec<String>,
    pub no_display: bool,
    pub terminal: bool,
    pub path: PathBuf,
}

impl AppEntry {
    fn from_entry(entry: &DesktopEntry<'_>, path: PathBuf) -> Self {
        let locales: &[&str] = &[];
        Self {
            id: entry.appid.to_string(),
            name: entry
                .name(locales)
                .map(|c| c.to_string())
                .unwrap_or_else(|| entry.appid.to_string()),
            generic_name: entry.generic_name(locales).map(|c| c.to_string()),
            comment: entry.comment(locales).map(|c| c.to_string()),
            exec: entry.exec().map(|s| s.to_string()),
            icon: entry.icon().map(|s| s.to_string()),
            categories: entry
                .categories()
                .map(|v| v.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default(),
            mime_types: entry
                .mime_type()
                .map(|v| v.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default(),
            no_display: entry.no_display(),
            terminal: entry.terminal(),
            path,
        }
    }
}

pub fn discover_apps() -> Result<Vec<AppEntry>> {
    // default_paths() already returns Vec<PathBuf>; Iter::new takes it directly.
    let paths: Vec<PathBuf> = default_paths();
    let locales: &[&str] = &[];
    let mut entries: Vec<AppEntry> = Iter::new(paths)
        .filter_map(|path| {
            let content = std::fs::read_to_string(&path).ok()?;
            // from_str borrows from `content` and `path`, so we clone path
            // for storage in AppEntry after we extract what we need.
            let entry = DesktopEntry::from_str(&path, &content, locales).ok()?;
            Some(AppEntry::from_entry(&entry, path.clone()))
        })
        .collect();

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    entries.dedup_by(|a, b| a.id == b.id);
    Ok(entries)
}
