//! Automated TUI rendering tests using ratatui's in-memory `TestBackend`.
//!
//! These render the real `App` views into an off-screen buffer (no TTY) and
//! assert on the rendered text, so the terminal UI is covered by automated
//! tests just like the Web UI.

use paper2codes::ui::app::ViewMode;
use paper2codes::ui::App;
use ratatui::{backend::TestBackend, Terminal};

/// Render a given view into an 120x40 off-screen terminal and return the
/// rendered screen as a string (`TestBackend` implements `Display`).
fn render_view(view: ViewMode) -> String {
    let (mut app, _tx) = App::new();
    app.view_mode = view;
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).expect("create test terminal");
    terminal.draw(|f| app.draw(f)).expect("draw frame");
    format!("{}", terminal.backend())
}

#[test]
fn renders_domain_skills_view_with_catalog() {
    let screen = render_view(ViewMode::Skills);
    assert!(screen.contains("Domain Skills"), "title present");
    // The built-in catalog should be listed.
    assert!(screen.contains("Quantum Computation"));
    assert!(screen.contains("Computational Finance"));
    assert!(screen.contains("Computational Fluid Dynamics"));
}

#[test]
fn renders_dashboard_without_panicking() {
    let screen = render_view(ViewMode::Dashboard);
    assert!(!screen.trim().is_empty());
    // Status bar carries the product name.
    assert!(screen.contains("Paper2Codes"));
}

#[test]
fn renders_each_view_without_panicking() {
    for view in [
        ViewMode::Dashboard,
        ViewMode::Tasks,
        ViewMode::Modules,
        ViewMode::Code,
        ViewMode::Skills,
        ViewMode::Statistics,
        ViewMode::Settings,
        ViewMode::Logs,
        ViewMode::Storage,
        ViewMode::AdminConsole,
        ViewMode::Help,
    ] {
        let screen = render_view(view);
        assert!(!screen.is_empty(), "view {:?} rendered empty", view);
    }
}

#[test]
fn view_cycle_reaches_skills_after_code() {
    let (mut app, _tx) = App::new();
    app.view_mode = ViewMode::Code;
    app.cycle_view_mode();
    assert_eq!(app.view_mode, ViewMode::Skills);
}

#[test]
fn status_bar_labels_skills_view() {
    let screen = render_view(ViewMode::Skills);
    // The status bar view-mode label uses the friendly name.
    assert!(screen.contains("Domain Skills"));
}
