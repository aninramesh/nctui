//! Layout and rendering for the interactive TUI.
//!
//! Composes the tree, heatmap, stats, search bar, and modal overlays into
//! a single terminal frame.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::app::{App, Focus, Modal};

/// Render the full application UI into a ratatui buffer.
pub fn draw(app: &App, area: Rect, buf: &mut Buffer) {
    // Top-level: vertical split into [main area | status bar]
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),    // main content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    let main_area = outer[0];
    let status_area = outer[1];

    // Main area: horizontal split [tree panel | right panel]
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(32), // tree + search
            Constraint::Min(20),   // heatmap + stats
        ])
        .split(main_area);

    let left_area = columns[0];
    let right_area = columns[1];

    // Left column: vertical split [search bar | tree]
    let left_col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // search bar
            Constraint::Min(4),   // tree
        ])
        .split(left_area);

    draw_search_bar(app, left_col[0], buf);
    draw_tree(app, left_col[1], buf);

    // Right side: vertical split [heatmap | stats]
    let right_col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(65), // heatmap
            Constraint::Min(6),         // stats
        ])
        .split(right_area);

    draw_heatmap(app, right_col[0], buf);
    draw_stats(app, right_col[1], buf);

    // Status bar
    draw_status_bar(app, status_area, buf);

    // Modal overlays (drawn on top of everything)
    match app.modal {
        Modal::Histogram => {
            app.histogram.render(main_area, buf);
        }
        Modal::Table => {
            if let Some(ref table) = app.table {
                table.render(main_area, buf);
            }
        }
        Modal::SlicePicker => {
            if let Some(ref picker) = app.slice_picker {
                picker.render(main_area, buf);
            }
        }
        Modal::Help => {
            draw_help_modal(main_area, buf);
        }
        Modal::None => {}
    }
}

fn draw_search_bar(app: &App, area: Rect, buf: &mut Buffer) {
    app.search.render_bar(area, buf);
    if app.search.has_filter() && app.focus == Focus::Search {
        // If search is active and has results, replace the tree with results
        // (handled in draw_tree instead)
    }
}

fn draw_tree(app: &App, area: Rect, buf: &mut Buffer) {
    if app.search.has_filter() {
        // Show search results instead of tree
        app.search.render_results(area, buf);
    } else {
        let border_color = if app.focus == Focus::Tree {
            Color::Yellow
        } else {
            Color::Gray
        };
        // We render the tree manually to control the border color based on focus
        let block = Block::default()
            .title(" Variables ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        Widget::render(block, area, buf);

        // Render tree rows inside the block
        render_tree_rows(&app.tree, inner, buf);
    }
}

fn render_tree_rows(tree: &crate::tree::TreeNavigator, area: Rect, buf: &mut Buffer) {
    use crate::tree::RowKind;

    if area.height == 0 || area.width == 0 {
        return;
    }

    // Compute scroll offset to keep selected row visible
    let visible_rows = area.height as usize;
    let scroll = if tree.selected >= visible_rows {
        tree.selected - visible_rows + 1
    } else {
        0
    };

    for (i, row) in tree.rows.iter().enumerate().skip(scroll).take(visible_rows) {
        let y = area.y + (i - scroll) as u16;
        if y >= area.y + area.height {
            break;
        }

        let indent = "  ".repeat(row.indent);
        let icon = match &row.kind {
            RowKind::Group { expanded: true } => "\u{25bc} ",
            RowKind::Group { expanded: false } => "\u{25b6} ",
            RowKind::Variable { is_coord: true } => "\u{25c6} ",
            RowKind::Variable { is_coord: false } => "  ",
        };
        let cursor = if i == tree.selected { "\u{203a} " } else { "  " };

        let style = if i == tree.selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            match &row.kind {
                RowKind::Group { .. } => Style::default().fg(Color::Cyan),
                RowKind::Variable { is_coord: true } => Style::default().fg(Color::DarkGray),
                RowKind::Variable { is_coord: false } => Style::default().fg(Color::White),
            }
        };

        let text = format!("{cursor}{indent}{icon}{}", row.label);
        let line_area = Rect::new(area.x, y, area.width, 1);
        let line = Line::from(Span::styled(text, style));
        Widget::render(line, line_area, buf);
    }
}

fn draw_heatmap(app: &App, area: Rect, buf: &mut Buffer) {
    match &app.heatmap {
        Some(hm) => hm.render(area, buf),
        None => {
            let block = Block::default()
                .title(" Heatmap ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray));
            let inner = block.inner(area);
            Widget::render(block, area, buf);
            if inner.width >= 20 && inner.height >= 1 {
                let msg = Line::from(Span::styled(
                    "Select a variable to visualize",
                    Style::default().fg(Color::DarkGray),
                ));
                Widget::render(msg, inner, buf);
            }
        }
    }
}

fn draw_stats(app: &App, area: Rect, buf: &mut Buffer) {
    app.stats.render(area, buf);
}

fn draw_status_bar(app: &App, area: Rect, buf: &mut Buffer) {
    let file_label = format!(" {} ", app.file_path);
    let var_label = match &app.current_var {
        Some(v) => format!(" | {v} "),
        None => String::new(),
    };
    let msg = if app.status_msg.is_empty() {
        String::new()
    } else {
        format!(" | {} ", app.status_msg)
    };
    let help_hint = " | ? help | q quit ";

    let line = Line::from(vec![
        Span::styled(file_label, Style::default().fg(Color::Cyan)),
        Span::styled(var_label, Style::default().fg(Color::Green)),
        Span::styled(msg, Style::default().fg(Color::White)),
        Span::styled(
            help_hint,
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let status_block = Paragraph::new(line)
        .style(
            Style::default()
                .bg(Color::Rgb(30, 30, 40))
                .fg(Color::White),
        );
    Widget::render(status_block, area, buf);
}

fn draw_help_modal(area: Rect, buf: &mut Buffer) {
    let modal_w = area.width.min(60).max(30);
    let modal_h = area.height.min(24).max(10);
    let x = area.x + (area.width.saturating_sub(modal_w)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_h)) / 2;
    let modal_area = Rect::new(x, y, modal_w, modal_h);

    Clear.render(modal_area, buf);

    let block = Block::default()
        .title(" Keybindings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let inner = block.inner(modal_area);
    Widget::render(block, modal_area, buf);

    let keys: Vec<(&str, &str)> = vec![
        ("j/k or \u{2191}\u{2193}", "Navigate tree"),
        ("Enter/Space", "Select variable / expand group"),
        ("g/G", "Jump to top / bottom"),
        ("/", "Open search"),
        ("Esc", "Close modal / cancel search"),
        ("h", "Toggle histogram overlay"),
        ("t", "Toggle table preview"),
        ("s", "Open slice picker (nD vars)"),
        ("?", "Toggle this help"),
        ("q", "Quit"),
        ("", ""),
        ("In histogram:", "+/- adjust bins"),
        ("In table:", "\u{2191}\u{2193}\u{2190}\u{2192} scroll"),
        ("In slicer:", "x/y/f role, h/l idx"),
    ];

    let ks = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let ds = Style::default().fg(Color::White);

    for (i, (key, desc)) in keys.iter().enumerate() {
        if i as u16 >= inner.height {
            break;
        }
        let line = if key.is_empty() {
            Line::from(Span::raw(""))
        } else {
            Line::from(vec![
                Span::styled(format!("{:<20}", key), ks),
                Span::styled(*desc, ds),
            ])
        };
        let line_area = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
        Widget::render(line, line_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn test_help_modal_renders() {
        let area = Rect::new(0, 0, 60, 24);
        let mut buf = Buffer::empty(area);
        draw_help_modal(area, &mut buf);
        // Just check it doesn't panic and renders the title
        let text = buffer_to_string(&buf);
        assert!(text.contains("Keybindings"));
    }

    fn buffer_to_string(buf: &Buffer) -> String {
        let area = buf.area;
        let mut lines = Vec::with_capacity(area.height as usize);
        for y in area.y..area.y + area.height {
            let mut line = String::with_capacity(area.width as usize);
            for x in area.x..area.x + area.width {
                let cell = &buf[(x, y)];
                line.push_str(cell.symbol());
            }
            lines.push(line.trim_end().to_string());
        }
        lines.join("\n")
    }
}
