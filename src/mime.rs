//! MIME type default-application handling.
//!
//! Reads the user's `mimeapps.list` (per the Association spec) to show
//! which .desktop file is the default handler for each registered MIME
//! type, and shells out to `xdg-mime` to change defaults.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct MimeAssociation {
    pub mime_type: String,
    pub default_app: Option<String>,
    pub added_apps: Vec<String>,
    pub removed_apps: Vec<String>,
}

pub fn mimeapps_list_path() -> PathBuf {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(".config")
        });
    config_home.join("mimeapps.list")
}

pub fn read_associations() -> Result<Vec<MimeAssociation>> {
    let path = mimeapps_list_path();
    let mut map: BTreeMap<String, MimeAssociation> = BTreeMap::new();

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;

    // The file uses INI-like sections: [Default Applications],
    // [Added Associations], [Removed Associations]. We parse it manually
    // rather than pulling a dep — the format is trivial.
    let mut section: &str = "";
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = &trimmed[1..trimmed.len() - 1];
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim().to_string();
        let apps: Vec<String> = value
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        let entry = map.entry(key.clone()).or_insert_with(|| MimeAssociation {
            mime_type: key,
            default_app: None,
            added_apps: Vec::new(),
            removed_apps: Vec::new(),
        });

        match section {
            "Default Applications" => {
                entry.default_app = apps.into_iter().next();
            }
            "Added Associations" => entry.added_apps = apps,
            "Removed Associations" => entry.removed_apps = apps,
            _ => {}
        }
    }

    Ok(map.into_values().collect())
}

/// Invokes `xdg-mime default <desktop_file> <mime_type>` to update the
/// user's default handler. Returns the command's stderr on non-zero exit.
pub fn set_default(desktop_file: &str, mime_type: &str) -> Result<()> {
    let output = Command::new("xdg-mime")
        .arg("default")
        .arg(desktop_file)
        .arg(mime_type)
        .output()
        .context("failed to run xdg-mime (is it installed?)")?;

    if !output.status.success() {
        anyhow::bail!(
            "xdg-mime exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}
