//! xdg-tui — a terminal UI for inspecting and managing XDG settings.

mod app;
mod desktop_entries;
mod mime;
mod ui;
mod xdg_base;

use anyhow::Result;
use app::{App, Status, Tab};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

fn main() -> Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app, catching errors so we can always clean up.
    let mut app = App::new()?;
    let result = run(&mut terminal, &mut app);

    // Teardown — always, even on error.
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Poll with a timeout so status-line expiry triggers a redraw even
        // when the user is idle.
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                handle_key(app, key.code, key.modifiers);
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    // If a filter input is active, most keys go to the filter instead of
    // the global command set.
    let filter_active = match app.tab {
        Tab::MimeTypes => app.mime_filter_active,
        Tab::Applications => app.app_filter_active,
        Tab::BaseDirs => false,
    };

    if filter_active {
        handle_filter_key(app, code);
        return;
    }

    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => app.should_quit = true,

        KeyCode::Tab | KeyCode::Right if mods.is_empty() => app.tab = app.tab.next(),
        KeyCode::BackTab | KeyCode::Left => app.tab = app.tab.prev(),

        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),

        KeyCode::Char('r') => app.refresh(),

        KeyCode::Char('/') => match app.tab {
            Tab::MimeTypes => {
                app.mime_filter_active = true;
                app.mime_filter.clear();
            }
            Tab::Applications => {
                app.app_filter_active = true;
                app.app_filter.clear();
            }
            Tab::BaseDirs => {}
        },

        KeyCode::Enter => handle_enter(app),

        _ => {}
    }
}

fn handle_filter_key(app: &mut App, code: KeyCode) {
    let (filter, active, list_state, list_len): (&mut String, &mut bool, _, usize) = match app.tab {
        Tab::MimeTypes => {
            let len = app.filtered_mime_indices().len();
            (
                &mut app.mime_filter,
                &mut app.mime_filter_active,
                &mut app.mime_state,
                len,
            )
        }
        Tab::Applications => {
            let len = app.filtered_app_indices().len();
            (
                &mut app.app_filter,
                &mut app.app_filter_active,
                &mut app.app_state,
                len,
            )
        }
        Tab::BaseDirs => return,
    };

    match code {
        KeyCode::Esc => {
            filter.clear();
            *active = false;
        }
        KeyCode::Enter => {
            *active = false;
        }
        KeyCode::Backspace => {
            filter.pop();
        }
        KeyCode::Char(c) => {
            filter.push(c);
        }
        _ => {}
    }

    // After the filter changes, make sure selection is in range.
    if list_len == 0 {
        list_state.select(None);
    } else if list_state.selected().map(|s| s >= list_len).unwrap_or(true) {
        list_state.select(Some(0));
    }
}

fn handle_enter(app: &mut App) {
    // Enter on the MIME tab means "set a new default for this type".
    // We need the user to pick the application — the simplest flow without
    // adding a full modal stack is: require them to first highlight the
    // target app on the Applications tab, then come back and press Enter.
    // We store the last-viewed app id implicitly via the app_state.
    if app.tab != Tab::MimeTypes {
        return;
    }

    let mime_indices = app.filtered_mime_indices();
    let Some(&mime_idx) = mime_indices.get(app.mime_state.selected().unwrap_or(0)) else {
        return;
    };
    let mime_type = app.mime_assocs[mime_idx].mime_type.clone();

    let app_indices = app.filtered_app_indices();
    let Some(&app_idx) = app_indices.get(app.app_state.selected().unwrap_or(0)) else {
        app.status = Some(Status::error(
            "Pick an application on the Applications tab first, then return here and press Enter.",
        ));
        return;
    };
    let desktop_id = app.apps[app_idx].id.clone();
    let desktop_file = if desktop_id.ends_with(".desktop") {
        desktop_id.clone()
    } else {
        format!("{}.desktop", desktop_id)
    };

    match mime::set_default(&desktop_file, &mime_type) {
        Ok(()) => {
            app.status = Some(Status::info(format!(
                "Set default for {} → {}",
                mime_type, desktop_file
            )));
            // Reload so the UI reflects the new default.
            app.mime_assocs = mime::read_associations().unwrap_or_default();
        }
        Err(e) => {
            app.status = Some(Status::error(format!("Failed: {}", e)));
        }
    }
}

#[cfg(test)]
mod smoke_tests {
    use super::*;

    #[test]
    fn base_dirs_discover_returns_all_seven() {
        let b = xdg_base::BaseDirs::discover();
        assert_eq!(b.dirs.len(), 7);
        // Every entry should have a non-empty name and path.
        for d in &b.dirs {
            assert!(!d.name.is_empty());
            assert!(d.value.as_os_str().len() > 0);
        }
    }

    #[test]
    fn mime_associations_does_not_panic() {
        // We don't assert contents — the test env may not have a mimeapps.list
        // — but it must not panic or error fatally.
        let _ = mime::read_associations();
    }

    #[test]
    fn desktop_entries_discover_does_not_panic() {
        let _ = desktop_entries::discover_apps();
    }

    #[test]
    fn tab_cycling() {
        use app::Tab;
        assert_eq!(Tab::BaseDirs.next(), Tab::MimeTypes);
        assert_eq!(Tab::MimeTypes.next(), Tab::Applications);
        assert_eq!(Tab::Applications.next(), Tab::BaseDirs);
        assert_eq!(Tab::BaseDirs.prev(), Tab::Applications);
    }
}
