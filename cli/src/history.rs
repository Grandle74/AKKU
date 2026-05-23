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
//   │  ↑/↓ to navigate     │                                            │
//   │  Enter to rollback   │                                            │
//   │  Esc to exit         │                                            │
//   └──────────────────────┴────────────────────────────────────────────┘
//
// Responsibilities:
//   - Fetch plan summaries via the API (read-only — never writes).
//   - Render the split-pane TUI via crossterm.
//   - Warn before rollback if newer completed plans touched the same target.
//   - Generate a rollback plan preview via `api::preview_rollback_intent`
//     (first Enter), then execute it via `api::approve_intent` (second Enter).
//
// Rollback is a two-step flow in the TUI:
//   1. First Enter  → preview_rollback_intent: generates + saves the rollback plan.
//                     The popup shows its steps (the restoration, not the failure).
//   2. Second Enter → approve_intent(plan, true): executes the saved plan.
//      Esc           → cancels, discards the pending plan.
//
// This module has zero rollback logic of its own — it is display and
// routing only. All state transitions live in the API and engine layers.

use api::{IntentOutcome, PlanSummary, approve_intent, list_plans, preview_rollback_intent};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{self, Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{self, Write};

// ── Public entry point ────────────────────────────────────────────────────────

/// Launches the history TUI. Called by `main` when the user types `history`.
///
/// Exits cleanly on Esc, Ctrl-C, or after a rollback completes.
/// All errors that would crash the TUI are surfaced as plain text instead —
/// the terminal is always restored before returning.
pub fn show_history() {
    let entries = match list_plans() {
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
    entries: Vec<PlanSummary>,
    selected: usize,
    /// Feedback line shown at the bottom after an action (rollback result, warning).
    message: Option<String>,
    /// True when the rollback preview popup is open and the next Enter fires.
    showing_popup: bool,
    /// The generated rollback plan shown in the popup, awaiting user confirmation.
    /// Set when showing_popup becomes true; consumed (and cleared) on confirmation.
    pending_rollback_plan: Option<PlanSummary>,
}

fn run_tui(entries: Vec<PlanSummary>) -> Result<(), String> {
    let mut stdout = io::stdout();

    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide).map_err(|e| e.to_string())?;

    let mut state = TuiState {
        entries,
        selected: 0,
        message: None,
        showing_popup: false,
        pending_rollback_plan: None,
    };

    // Initial draw before waiting for the first keypress.
    draw(&mut stdout, &state)?;

    loop {
        let event = match event::read() {
            Ok(e) => e,
            Err(_) => {
                // Transient read errors (e.g. SIGWINCH noise) — attempt a redraw
                // and continue rather than crashing out of the TUI.
                let _ = draw(&mut stdout, &state);
                continue;
            }
        };

        if let Event::Key(key) = event {
            // Ctrl-C exits unconditionally — even mid-confirm.
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                break;
            }

            match key.code {
                KeyCode::Esc => {
                    if state.showing_popup {
                        state.showing_popup = false;
                        if let Some(summary) = state.pending_rollback_plan.take() {
                            let _ = approve_intent(&summary.id, false);
                        }
                        state.message = Some("Rollback cancelled.".into());

                        // Reload so the rejected rollback plan appears in the list.
                        if let Ok(fresh) = list_plans() {
                            if state.selected >= fresh.len() {
                                state.selected = fresh.len().saturating_sub(1);
                            }
                            state.entries = fresh;
                        }
                    } else {
                        break;
                    }
                }

                // Navigation is blocked while the popup is open — it is modal.
                KeyCode::Up if !state.showing_popup => {
                    if state.selected > 0 {
                        state.selected -= 1;
                        // Clear feedback when moving — it belongs to the old selection.
                        state.message = None;
                    }
                }

                KeyCode::Down if !state.showing_popup => {
                    if state.selected + 1 < state.entries.len() {
                        state.selected += 1;
                        state.message = None;
                    }
                }

                KeyCode::Enter => handle_enter(&mut state),

                _ => {}
            }

            draw(&mut stdout, &state)?;
        }

        // Resize: redraw, or show the too-small message — draw() handles both.
        if let Event::Resize(_, _) = event {
            draw(&mut stdout, &state)?;
        }
    }

    terminal::disable_raw_mode().map_err(|e| e.to_string())?;
    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show).map_err(|e| e.to_string())?;

    Ok(())
}

/// Handles an Enter keypress.
///
/// First Enter: call `preview_rollback_intent` to generate (and save) the
/// rollback plan, then open the popup showing its steps — NOT the failed plan.
/// Second Enter (popup open): fire `approve_intent` with the stored plan.
///
/// After a successful rollback the entries are reloaded from disk so the
/// new rollback plan appears in the list without requiring a TUI restart.
fn handle_enter(state: &mut TuiState) {
    let entry = &state.entries[state.selected];

    if matches!(entry.status.as_str(), "rejected" | "executing" | "pending") {
        state.message = Some(format!("Cannot rollback a '{}' plan.", entry.status));
        return;
    }

    if state.showing_popup {
        state.showing_popup = false;

        // Single .take() — consume once, use the value.
        let summary = match state.pending_rollback_plan.take() {
            Some(s) => s,
            None => {
                state.message = Some("✗ No rollback plan ready — try again.".into());
                return;
            }
        };

        match approve_intent(&summary.id, true) {
            IntentOutcome::Immediate(lines) => {
                state.message = Some(lines.first().cloned().unwrap_or_else(|| "✔ Done.".into()));
            }
            _ => {
                state.message = Some("✗ Unexpected outcome — check system state.".into());
            }
        }

        // Reload on approval, same as the Esc/rejection path.
        if let Ok(fresh) = list_plans() {
            if state.selected >= fresh.len() {
                state.selected = fresh.len().saturating_sub(1);
            }
            state.entries = fresh;
        }
    } else {
        let plan_id = entry.id.clone();

        let preview_result = std::panic::catch_unwind(|| preview_rollback_intent(&plan_id))
            .unwrap_or_else(|_| Err(vec!["Internal error — engine panicked".into()]));

        match preview_result {
            Ok(summary) if summary.is_empty() => {
                state.message =
                    Some("✔ Already at pre-execution state — nothing to restore.".into());
            }
            Ok(summary) => {
                state.pending_rollback_plan = Some(summary);
                state.showing_popup = true;
                state.message = None;
            }
            Err(errors) => {
                state.message = Some(format!(
                    "✗ {}",
                    errors
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "Preview failed.".into())
                ));
            }
        }
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn draw(stdout: &mut impl Write, state: &TuiState) -> Result<(), String> {
    let (cols, rows) = terminal::size().map_err(|e| e.to_string())?;
    let cols = cols as usize;
    let rows = rows as usize;

    // Guard: terminal too small to render anything useful.
    // Handles the window-too-small case cleanly instead of panicking on underflows.
    if cols < 50 || rows < 8 {
        queue!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All),
            SetForegroundColor(Color::Yellow),
            Print("  ↔  Please resize your terminal"),
            ResetColor
        )
        .map_err(|e| e.to_string())?;
        return stdout.flush().map_err(|e| e.to_string());
    }

    // Reserve: 1 header + 1 column-label row + 1 footer + 1 message = 4 fixed rows.
    let content_rows = rows.saturating_sub(4);

    // Left pane is 1/3, right pane gets the rest. A '│' column sits between.
    let left_width = (cols / 3).max(22);
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

    // Popup is drawn last so it layers on top of everything else.
    if state.showing_popup {
        draw_popup(stdout, state, cols, rows)?;
    }

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
    let left_label = truncate("  # Date       Target        Status     Mode", left_width);
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
    entry: &PlanSummary,
    selected: bool,
    width: usize,
) -> Result<(), String> {
    let status_color = status_color(&entry.status);
    let num_col = " ";
    let date_col = &entry.date[..entry.date.len().min(10)];
    let target_col = truncate(&entry.target, 12);

    let mode_col = match entry.mode.as_deref() {
        Some("normal") => "Normal",
        Some("force") => "Force",
        Some("rollback") => "Rollback",
        _ => "—",
    };

    let row = format!(
        "{} {}  {:<12}  {:<9}  {:<8}",
        num_col, date_col, target_col, &entry.status, mode_col
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
    lines.push(format!(
        "Mode    {}",
        match entry.mode.as_deref() {
            Some("normal") => "Normal",
            Some("force") => "Force",
            Some("rollback") => "Rollback",
            _ => "—",
        }
    ));

    if let Some(origin) = &entry.rollback_of {
        lines.push(format!("Origin  {}", origin));
    }

    lines.push(sep.clone());

    // ── Steps with per-step execution status ──────────────────────────────────
    if entry.steps.is_empty() {
        lines.push("  (no steps recorded)".into());
    } else {
        for step in &entry.steps {
            // Status mark appended inline — visible without widening the pane.
            let mark = match step.status.as_deref() {
                Some("completed") => "  ✔",
                Some("failed") => "  ✗",
                _ => "",
            };
            lines.push(format!("  • {}{}", step.description, mark));
        }
    }

    lines.push(sep);
    lines.push(format!("Summary  {}", entry.summary));

    lines
}

/// Draws the rollback confirmation popup — a floating box centered over the TUI.
///
/// Layout:
///   ╭─ Confirm Rollback ──────────────────────────────╮
///   │  Restoring  nginx                               │
///   │  Origin     svc_20260407_143022_a3f2            │
///   │  ─────────────────────────────────────────────  │
///   │    • unmask nginx                               │
///   │    • enable nginx                               │
///   │    • start nginx                                │
///   │  ─────────────────────────────────────────────  │
///   │    1 later completed plan also touched 'nginx'  │  (if conflict)
///   ├─────────────────────────────────────────────────┤
///   │        [ Enter: Apply ]   [ Esc: Cancel ]       │
///   ╰─────────────────────────────────────────────────╯
fn draw_popup(
    stdout: &mut impl Write,
    state: &TuiState,
    cols: usize,
    rows: usize,
) -> Result<(), String> {
    let entry = &state.entries[state.selected];

    // ── Build content lines ───────────────────────────────────────────────────
    let mut body: Vec<(String, Color)> = Vec::new();

    body.push((format!("  Restoring  {}", entry.target), Color::Reset));
    body.push((format!("  Origin     {}", entry.id), Color::DarkGrey));

    let sep = "─".repeat(40);
    body.push((format!("  {}", sep), Color::DarkGrey));

    match &state.pending_rollback_plan {
        Some(summary) if !summary.steps.is_empty() => {
            for step in &summary.steps {
                body.push((format!("    • {}", step.description), Color::Reset));
            }
        }
        Some(_) => body.push((
            "    (already at pre-execution state)".into(),
            Color::DarkGrey,
        )),
        None => body.push(("    (rollback plan unavailable)".into(), Color::DarkGrey)),
    }

    body.push((format!("  {}", sep), Color::DarkGrey));

    if let Some(warn) = conflict_warning(&state.entries, state.selected) {
        body.push((format!("  ⚠  {}", warn), Color::Yellow));
    }

    // ── Size the box ─────────────────────────────────────────────────────────
    let inner_width = body
        .iter()
        .map(|(l, _)| l.chars().count())
        .max()
        .unwrap_or(44)
        .max(44);
    let box_width = inner_width + 4; // 2 border chars + 2 padding chars per side

    // Action bar is always one row, separated by a mid-border line.
    let box_height = body.len() + 4; // top + body rows + divider + action bar + bottom

    let col = cols.saturating_sub(box_width) / 2;
    let row = rows.saturating_sub(box_height) / 2;

    // ── Top border ────────────────────────────────────────────────────────────
    let title = " Confirm Rollback ";
    let fill = box_width.saturating_sub(title.len() + 3);
    let top = format!("╭─{}{}╮", title, "─".repeat(fill));
    queue!(
        stdout,
        cursor::MoveTo(col as u16, row as u16),
        SetForegroundColor(Color::Cyan),
        Print(&top),
        ResetColor,
    )
    .map_err(|e| e.to_string())?;

    // ── Body rows ─────────────────────────────────────────────────────────────
    for (i, (line, color)) in body.iter().enumerate() {
        let padded = format!("│{:<width$}│", format!("{}", line), width = box_width - 2);
        queue!(
            stdout,
            cursor::MoveTo(col as u16, (row + 1 + i) as u16),
            SetForegroundColor(*color),
            Print(&padded),
            ResetColor,
        )
        .map_err(|e| e.to_string())?;
    }

    // ── Mid divider ───────────────────────────────────────────────────────────
    let mid_row = row + 1 + body.len();
    let divider = format!("├{}┤", "─".repeat(box_width - 2));
    queue!(
        stdout,
        cursor::MoveTo(col as u16, mid_row as u16),
        SetForegroundColor(Color::Cyan),
        Print(&divider),
        ResetColor,
    )
    .map_err(|e| e.to_string())?;

    // ── Action bar ────────────────────────────────────────────────────────────
    let apply = "[ Enter: Apply ]";
    let cancel = "[ Esc: Cancel ]";
    let gap = 3;
    let actions_len = apply.len() + gap + cancel.len();
    let padding = (box_width.saturating_sub(2).saturating_sub(actions_len)) / 2;
    let trail = box_width
        .saturating_sub(2)
        .saturating_sub(padding + actions_len);

    queue!(
        stdout,
        cursor::MoveTo(col as u16, (mid_row + 1) as u16),
        SetForegroundColor(Color::DarkGrey),
        Print(format!("│{}", " ".repeat(padding))),
        SetForegroundColor(Color::Green),
        Print(apply),
        SetForegroundColor(Color::DarkGrey),
        Print(format!(
            "{}{}{}",
            " ".repeat(gap),
            cancel,
            " ".repeat(trail)
        )),
        Print("│"),
        ResetColor,
    )
    .map_err(|e| e.to_string())?;

    // ── Bottom border ─────────────────────────────────────────────────────────
    let bottom = format!("╰{}╯", "─".repeat(box_width - 2));
    queue!(
        stdout,
        cursor::MoveTo(col as u16, (mid_row + 2) as u16),
        SetForegroundColor(Color::Cyan),
        Print(&bottom),
        ResetColor,
    )
    .map_err(|e| e.to_string())
}

fn draw_footer(
    stdout: &mut impl Write,
    rows: usize,
    cols: usize,
    state: &TuiState,
) -> Result<(), String> {
    let hint = if state.showing_popup {
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

/// Returns a warning string if any plan *after* the selected one also
/// completed changes on the same target.
///
/// "After" is defined by position in the sorted list (newest-last).
/// Returns None when it is safe to proceed without a warning.
///
// TODO: Move to engine (plan_store) and expose through API once a broader
// use case exists beyond the TUI. Domain judgement should not live here.
fn conflict_warning(entries: &[PlanSummary], selected: usize) -> Option<String> {
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

// ── Utilities ─────────────────────────────────────────────────────────────────

fn scroll_offset(selected: usize, visible_rows: usize) -> usize {
    if selected < visible_rows {
        0
    } else {
        selected - visible_rows + 1
    }
}

fn truncate(s: &str, max: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max {
        s.to_string()
    } else if max > 1 {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", truncated)
    } else {
        s.chars().take(max).collect()
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
