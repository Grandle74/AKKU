// cli/src/history.rs
//
// History browser: an interactive TUI for reviewing past plans and
// triggering manual rollback.
//
// Layout (terminal-width responsive):
//
//   ┌─ History ─────────────────────────────────────────────────────────┐
//   │ [left ~1/3]          │ [right ~2/3]                               │
//   │  scrollable list     │  selected plan detail                      │
//   │  ↑/↓ to navigate    │                                            │
//   │  Enter to rollback   │                                            │
//   │  Esc to exit         │                                            │
//   └──────────────────────┴────────────────────────────────────────────┘
//
// Responsibilities:
//   - Read plan files from disk (read-only — never writes).
//   - Render the split-pane TUI via crossterm.
//   - Warn before rollback if newer completed plans touched the same target.
//   - Route confirmed rollbacks through `api::rollback_intent`.
//
// This module has zero rollback logic of its own — it is display and
// routing only. All state transitions live in the API and engine layers.

use api::rollback_intent;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{self, Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::PathBuf,
};

// ── Plan file schema (read-only mirror of plan_store's format) ────────────────

/// A step as recorded in the plan file.
#[derive(Deserialize)]
struct StoredStep {
    action: String,
    target: String,
    description: String,
}

/// A plan file as written by plan_store — only the fields history needs.
#[derive(Deserialize)]
struct StoredPlan {
    id: String,
    target: String,
    status: String,
    steps: Vec<StoredStep>,
    rollback_of: Option<String>,
}

// ── Display model ─────────────────────────────────────────────────────────────

/// Everything the TUI needs to render one row in the list pane and the
/// full detail pane. Built once at load time from the raw StoredPlan.
struct PlanEntry {
    id: String,
    target: String,
    status: String,
    /// Human-readable date derived from the ID timestamp segment.
    date: String,
    /// One-line action summary for the list pane.
    summary: String,
    /// Full step descriptions for the detail pane.
    steps: Vec<String>,
    /// Present when this plan was itself a rollback of another plan.
    rollback_of: Option<String>,
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Launches the history TUI. Called by `main` when the user types `history`.
///
/// Exits cleanly on Esc, Ctrl-C, or after a rollback completes.
/// All errors that would crash the TUI are surfaced as plain text instead —
/// the terminal is always restored before returning.
pub fn show_history() {
    let entries = match load_entries() {
        Ok(e) if e.is_empty() => {
            println!("No plan history found.");
            return;
        }
        Ok(e) => e,
        Err(e) => {
            println!("✗ Error: Could not load history — {}", e);
            return;
        }
    };

    if let Err(e) = run_tui(entries) {
        // Ensure the terminal is clean even on unexpected errors.
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), terminal::LeaveAlternateScreen, cursor::Show);
        println!("✗ Error: {}", e);
    }
}

// ── TUI loop ──────────────────────────────────────────────────────────────────

/// State for the running TUI session.
struct TuiState {
    entries: Vec<PlanEntry>,
    selected: usize,
    /// Feedback line shown at the bottom after an action (rollback result, warning).
    message: Option<String>,
    /// True when a rollback warning is pending and the next keypress is a confirm.
    awaiting_confirm: bool,
}

fn run_tui(entries: Vec<PlanEntry>) -> Result<(), String> {
    let mut stdout = io::stdout();

    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide).map_err(|e| e.to_string())?;

    let mut state = TuiState {
        entries,
        selected: 0,
        message: None,
        awaiting_confirm: false,
    };

    // Initial draw before waiting for the first keypress.
    draw(&mut stdout, &state)?;

    loop {
        let event = event::read().map_err(|e| e.to_string())?;

        if let Event::Key(key) = event {
            // Ctrl-C exits unconditionally — even mid-confirm.
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                break;
            }

            match key.code {
                KeyCode::Esc => {
                    if state.awaiting_confirm {
                        // Cancel the pending rollback rather than exiting.
                        state.awaiting_confirm = false;
                        state.message = Some("Rollback cancelled.".into());
                    } else {
                        break;
                    }
                }

                KeyCode::Up => {
                    if state.selected > 0 {
                        state.selected -= 1;
                        // Clear feedback when moving — it belongs to the old selection.
                        state.message = None;
                        state.awaiting_confirm = false;
                    }
                }

                KeyCode::Down => {
                    if state.selected + 1 < state.entries.len() {
                        state.selected += 1;
                        state.message = None;
                        state.awaiting_confirm = false;
                    }
                }

                KeyCode::Enter => handle_enter(&mut state),

                _ => {}
            }

            draw(&mut stdout, &state)?;

            // After a completed rollback (non-confirm Enter on a done action),
            // the message is set and awaiting_confirm is false — we stay open
            // so the user can see the result and press Esc to exit.
        }

        // Resize: just redraw.
        if let Event::Resize(_, _) = event {
            draw(&mut stdout, &state)?;
        }
    }

    terminal::disable_raw_mode().map_err(|e| e.to_string())?;
    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show).map_err(|e| e.to_string())?;

    Ok(())
}

/// Handles an Enter keypress: first press warns (if needed) and arms the
/// confirm flag; second press fires the rollback.
fn handle_enter(state: &mut TuiState) {
    let entry = &state.entries[state.selected];

    // Only pending or failed plans can be rolled back.
    // Completed plans whose execution succeeded have a snapshot and are valid targets.
    // Rejected plans never executed — nothing to restore.
    if matches!(entry.status.as_str(), "rejected" | "executing") {
        state.message = Some(format!("Cannot rollback a '{}' plan.", entry.status));
        return;
    }

    if state.awaiting_confirm {
        // Second Enter — fire.
        state.awaiting_confirm = false;
        let plan_id = entry.id.clone();
        let outcome = rollback_intent(&plan_id);
        state.message = Some(format_rollback_result(outcome));
    } else {
        // First Enter — arm, with optional conflict warning.
        let warn = conflict_warning(&state.entries, state.selected);
        state.awaiting_confirm = true;
        state.message = Some(match warn {
            Some(w) => format!("⚠  {}  —  Enter to confirm, Esc to cancel.", w),
            None => "Press Enter again to confirm rollback, Esc to cancel.".into(),
        });
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn draw(stdout: &mut impl Write, state: &TuiState) -> Result<(), String> {
    let (cols, rows) = terminal::size().map_err(|e| e.to_string())?;
    let cols = cols as usize;
    let rows = rows as usize;

    // Reserve: 1 header + 1 divider + 1 footer + 1 message = 4 fixed rows.
    // Everything in between is list/detail content.
    let content_rows = rows.saturating_sub(4);

    // Left pane is 1/3, right pane gets the rest. A '│' column sits between.
    let left_width = (cols / 3).max(20);
    let divider_col = left_width + 1;
    let right_width = cols.saturating_sub(divider_col + 1);

    queue!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(ClearType::All)
    )
    .map_err(|e| e.to_string())?;

    draw_header(stdout, cols)?;
    draw_panes(
        stdout,
        state,
        content_rows,
        left_width,
        divider_col,
        right_width,
    )?;
    draw_footer(stdout, rows, cols, state)?;

    stdout.flush().map_err(|e| e.to_string())
}

fn draw_header(stdout: &mut impl Write, cols: usize) -> Result<(), String> {
    let title = " History ";
    let bar = format!(
        "─{}{}",
        title,
        "─".repeat(cols.saturating_sub(title.len() + 1))
    );

    queue!(
        stdout,
        cursor::MoveTo(0, 0),
        SetForegroundColor(Color::Cyan),
        Print(&bar),
        ResetColor
    )
    .map_err(|e| e.to_string())
}

fn draw_panes(
    stdout: &mut impl Write,
    state: &TuiState,
    content_rows: usize,
    left_width: usize,
    divider_col: usize,
    right_width: usize,
) -> Result<(), String> {
    // Scroll offset: keep the selected row visible.
    let scroll = scroll_offset(state.selected, content_rows);

    for row in 0..content_rows {
        let screen_row = (row + 2) as u16; // +2: header + column-label row
        let entry_idx = scroll + row;

        // ── Left pane ─────────────────────────────────────────────────────────
        queue!(stdout, cursor::MoveTo(0, screen_row)).map_err(|e| e.to_string())?;

        if let Some(entry) = state.entries.get(entry_idx) {
            let is_selected = entry_idx == state.selected;
            draw_list_row(stdout, entry, is_selected, left_width)?;
        } else {
            // Empty rows below the list — fill with spaces to clear artifacts.
            queue!(stdout, Print(" ".repeat(left_width))).map_err(|e| e.to_string())?;
        }

        // ── Divider ───────────────────────────────────────────────────────────
        queue!(
            stdout,
            cursor::MoveTo(divider_col as u16, screen_row),
            SetForegroundColor(Color::DarkGrey),
            Print("│"),
            ResetColor
        )
        .map_err(|e| e.to_string())?;

        // ── Right pane: only for the selected entry's rows ────────────────────
        let detail_start_row = 2u16;
        let detail_row = screen_row.saturating_sub(detail_start_row) as usize;
        let detail_lines = build_detail(state, right_width);

        queue!(stdout, cursor::MoveTo((divider_col + 2) as u16, screen_row))
            .map_err(|e| e.to_string())?;

        if let Some(line) = detail_lines.get(detail_row) {
            // Truncate to fit; pad to clear any leftover characters.
            let truncated = truncate(line, right_width);
            let padded = format!("{:<width$}", truncated, width = right_width);
            queue!(stdout, Print(padded)).map_err(|e| e.to_string())?;
        } else {
            queue!(stdout, Print(" ".repeat(right_width))).map_err(|e| e.to_string())?;
        }
    }

    // Column labels on row 1 (between header and first entry).
    draw_column_labels(stdout, left_width, divider_col, right_width)
}

fn draw_column_labels(
    stdout: &mut impl Write,
    left_width: usize,
    divider_col: usize,
    right_width: usize,
) -> Result<(), String> {
    let left_label = truncate("  # Date       Target          Status", left_width);
    let right_label = "  Detail";

    queue!(
        stdout,
        cursor::MoveTo(0, 1),
        SetForegroundColor(Color::DarkGrey),
        Print(format!("{:<width$}", left_label, width = left_width)),
        cursor::MoveTo(divider_col as u16, 1),
        Print("│"),
        cursor::MoveTo((divider_col + 2) as u16, 1),
        Print(truncate(right_label, right_width)),
        ResetColor
    )
    .map_err(|e| e.to_string())
}

fn draw_list_row(
    stdout: &mut impl Write,
    entry: &PlanEntry,
    selected: bool,
    width: usize,
) -> Result<(), String> {
    let status_color = status_color(&entry.status);
    let num_col = " ";
    let date_col = &entry.date[..entry.date.len().min(10)];

    // Truncate target to keep the row from overflowing.
    let target_col = truncate(&entry.target, 14);

    let row = format!(
        "{} {}  {:<14}  {}",
        num_col, date_col, target_col, &entry.status
    );
    let row = truncate(&row, width.saturating_sub(2));

    if selected {
        queue!(
            stdout,
            SetForegroundColor(Color::Black),
            style::SetBackgroundColor(Color::Cyan),
            Print(format!(" {:<width$}", row, width = width.saturating_sub(1))),
            ResetColor
        )
        .map_err(|e| e.to_string())
    } else {
        queue!(
            stdout,
            SetForegroundColor(status_color),
            Print(format!(" {:<width$}", row, width = width.saturating_sub(1))),
            ResetColor
        )
        .map_err(|e| e.to_string())
    }
}

/// Builds the right-pane detail lines for the currently selected entry.
fn build_detail(state: &TuiState, width: usize) -> Vec<String> {
    let entry = &state.entries[state.selected];
    let sep = "─".repeat(width.min(48));
    let mut lines: Vec<String> = Vec::new();

    // ── Identity ──────────────────────────────────────────────────────────────
    lines.push(format!("Plan    {}", entry.id));
    lines.push(format!("Target  {}", entry.target));
    lines.push(format!("Date    {}", entry.date));
    lines.push(format!("Status  {}", entry.status));

    if let Some(origin) = &entry.rollback_of {
        lines.push(format!("Origin  {}", origin));
    }

    lines.push(sep.clone());

    // ── Steps ─────────────────────────────────────────────────────────────────
    if entry.steps.is_empty() {
        lines.push("  (no steps recorded)".into());
    } else {
        for step in &entry.steps {
            lines.push(format!("  • {}", step));
        }
    }

    lines.push(sep);
    lines.push(format!("Summary  {}", entry.summary));

    lines
}

fn draw_footer(
    stdout: &mut impl Write,
    rows: usize,
    cols: usize,
    state: &TuiState,
) -> Result<(), String> {
    let hint = if state.awaiting_confirm {
        "  Enter confirm  ·  Esc cancel"
    } else {
        "  ↑/↓ navigate  ·  Enter rollback  ·  Esc exit"
    };

    let msg = state.message.as_deref().unwrap_or("");
    let msg_truncated = truncate(msg, cols.saturating_sub(hint.len() + 2));

    let footer_row = (rows - 1) as u16;

    queue!(
        stdout,
        cursor::MoveTo(0, footer_row),
        terminal::Clear(ClearType::CurrentLine),
        SetForegroundColor(Color::DarkGrey),
        Print(hint),
    )
    .map_err(|e| e.to_string())?;

    if !msg_truncated.is_empty() {
        let msg_col = (cols.saturating_sub(msg_truncated.len() + 2)) as u16;
        let color = if msg.starts_with("✗") || msg.starts_with("⚠") {
            Color::Yellow
        } else if msg.starts_with("✔") {
            Color::Green
        } else {
            Color::DarkGrey
        };
        queue!(
            stdout,
            cursor::MoveTo(msg_col, footer_row),
            SetForegroundColor(color),
            Print(&msg_truncated),
            ResetColor
        )
        .map_err(|e| e.to_string())?;
    }

    queue!(stdout, ResetColor).map_err(|e| e.to_string())
}

// ── Conflict detection ────────────────────────────────────────────────────────

/// Returns a warning string if any plan *after* the selected one in the
/// sorted list also completed changes on the same target.
///
/// "After" is defined by position in the sorted list (newest-last),
/// meaning all entries with a higher index than `selected`.
///
/// Returns None when it is safe to proceed without a warning.
fn conflict_warning(entries: &[PlanEntry], selected: usize) -> Option<String> {
    let target = &entries[selected].target;

    let conflicts: Vec<&str> = entries[selected + 1..]
        .iter()
        .filter(|e| e.target == *target && e.status == "completed")
        .map(|e| e.id.as_str())
        .collect();

    if conflicts.is_empty() {
        return None;
    }

    let count = conflicts.len();
    Some(format!(
        "{} later completed plan{} also touched '{}'",
        count,
        if count == 1 { "" } else { "s" },
        target
    ))
}

// ── Rollback result rendering ─────────────────────────────────────────────────

/// Converts an IntentOutcome from rollback_intent() into a one-line
/// message for the TUI footer. Detailed output is not rendered inside
/// the TUI — the user exits and the result is printed to the terminal.
fn format_rollback_result(outcome: api::IntentOutcome) -> String {
    match outcome {
        api::IntentOutcome::RolledBack { .. } => {
            "✔ Rollback applied — press Esc to exit and see details.".into()
        }
        api::IntentOutcome::RollbackFailed { errors, .. } => {
            format!(
                "✗ Rollback failed — {}",
                errors.first().cloned().unwrap_or_default()
            )
        }
        // rollback_intent() only ever returns the two variants above.
        // Any other variant here is a contract violation — surface it clearly.
        _ => "✗ Unexpected outcome from rollback — check system state.".into(),
    }
}

// ── Data loading ──────────────────────────────────────────────────────────────

/// Reads all plan files from disk, parses them, and returns them sorted
/// oldest-first (ascending by ID, which encodes the creation timestamp).
fn load_entries() -> Result<Vec<PlanEntry>, String> {
    let dir = plans_dir();

    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut entries: Vec<PlanEntry> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map_or(false, |x| x == "json")
        })
        .filter_map(|e| {
            let content = fs::read_to_string(e.path()).ok()?;
            let stored: StoredPlan = serde_json::from_str(&content).ok()?;
            Some(to_entry(stored))
        })
        .collect();

    // IDs are lexicographically sortable by timestamp — no date parsing needed.
    entries.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(entries)
}

fn plans_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".yast3").join("plans")
}

/// Converts a raw StoredPlan into the display model used by the TUI.
fn to_entry(p: StoredPlan) -> PlanEntry {
    let date = date_from_id(&p.id);
    let summary = build_summary(&p);
    let steps = p.steps.iter().map(|s| s.description.clone()).collect();

    PlanEntry {
        id: p.id,
        target: p.target,
        status: p.status,
        date,
        summary,
        steps,
        rollback_of: p.rollback_of,
    }
}

// ── ID parsing ────────────────────────────────────────────────────────────────

/// Extracts the human-readable date portion from a plan ID.
///
/// ID format: `<prefix>_<YYYYMMDD>_<HHMMSS>_<hex>`
/// Example:   `svc_20260407_143022_a3f2`  →  `2026-04-07 14:30`
fn date_from_id(id: &str) -> String {
    let parts: Vec<&str> = id.split('_').collect();

    // A well-formed ID has at least 4 segments: prefix, date, time, hex.
    if parts.len() < 4 {
        return id.to_string();
    }

    let date = parts[1]; // YYYYMMDD
    let time = parts[2]; // HHMMSS

    if date.len() == 8 && time.len() == 6 {
        format!(
            "{}-{}-{} {}:{}",
            &date[0..4],
            &date[4..6],
            &date[6..8],
            &time[0..2],
            &time[2..4],
        )
    } else {
        id.to_string()
    }
}

// ── Summary generation ────────────────────────────────────────────────────────

/// Builds a one-line human-readable action summary from a plan's steps.
///
/// Examples:
///   "start nginx"                  (1 step)
///   "enable, start nginx"          (2 steps, same target)
///   "unmask, enable, start nginx"  (3 steps)
///   "rolled back svc_20260407_..."  (rollback plan)
fn build_summary(plan: &StoredPlan) -> String {
    if let Some(origin) = &plan.rollback_of {
        return format!("rolled back {}", origin);
    }

    if plan.steps.is_empty() {
        return "—".into();
    }

    // Collect unique action names, preserving order.
    let mut seen = std::collections::HashSet::new();
    let actions: Vec<&str> = plan
        .steps
        .iter()
        .map(|s| s.action.as_str())
        .filter(|a| seen.insert(*a))
        .collect();

    format!("{} {}", actions.join(", "), plan.target)
}

// ── Utilities ─────────────────────────────────────────────────────────────────

fn scroll_offset(selected: usize, visible_rows: usize) -> usize {
    if selected < visible_rows {
        0
    } else {
        selected - visible_rows + 1
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 1 {
        format!("{}…", &s[..max.saturating_sub(1)])
    } else {
        s[..max].to_string()
    }
}

fn status_color(status: &str) -> Color {
    match status {
        "completed" => Color::Green,
        "failed" => Color::Red,
        "executing" => Color::Yellow,
        "rejected" => Color::DarkGrey,
        _ => Color::Reset, // "pending" and anything unknown
    }
}
