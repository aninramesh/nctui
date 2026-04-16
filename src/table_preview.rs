use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Row as TableRow, Table, Widget},
};

/// Table preview mode for inspecting exact numeric values of 1D variables
/// or small 2D slices.
pub struct TablePreview {
    pub visible: bool,
    pub title: String,
    pub col_headers: Vec<String>,
    pub row_headers: Vec<String>,
    /// 2D data: rows × columns. For 1D data, there is a single column.
    pub data: Vec<Vec<f64>>,
    /// Scroll offset for row display.
    pub scroll_row: usize,
    /// Scroll offset for column display.
    pub scroll_col: usize,
}

impl TablePreview {
    /// Create a table preview from a 1D data vector.
    pub fn from_1d(data: &[f64], var_name: &str, dim_name: &str, coords: Option<&[f64]>) -> Self {
        let row_headers: Vec<String> = match coords {
            Some(c) if c.len() == data.len() => c.iter().map(|v| format_cell(*v)).collect(),
            _ => (0..data.len()).map(|i| format!("{i}")).collect(),
        };
        Self {
            visible: true,
            title: format!("{var_name} [{dim_name}]"),
            col_headers: vec!["Value".to_string()],
            row_headers,
            data: data.iter().map(|v| vec![*v]).collect(),
            scroll_row: 0,
            scroll_col: 0,
        }
    }

    /// Create a table preview from a 2D data grid.
    pub fn from_2d(
        data: &[Vec<f64>],
        var_name: &str,
        row_dim: &str,
        col_dim: &str,
        row_coords: Option<&[f64]>,
        col_coords: Option<&[f64]>,
    ) -> Self {
        let nrows = data.len();
        let ncols = data.first().map_or(0, |r| r.len());
        let row_headers: Vec<String> = match row_coords {
            Some(c) if c.len() == nrows => c.iter().map(|v| format_cell(*v)).collect(),
            _ => (0..nrows).map(|i| format!("{i}")).collect(),
        };
        let col_headers: Vec<String> = match col_coords {
            Some(c) if c.len() == ncols => c.iter().map(|v| format_cell(*v)).collect(),
            _ => (0..ncols).map(|i| format!("{i}")).collect(),
        };
        Self {
            visible: true,
            title: format!("{var_name} [{row_dim} \u{00d7} {col_dim}]"),
            col_headers,
            row_headers,
            data: data.to_vec(),
            scroll_row: 0,
            scroll_col: 0,
        }
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.data.len().saturating_sub(1);
        self.scroll_row = (self.scroll_row + n).min(max);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_row = self.scroll_row.saturating_sub(n);
    }

    pub fn scroll_right(&mut self, n: usize) {
        let ncols = self.col_headers.len();
        if ncols > 0 {
            self.scroll_col = (self.scroll_col + n).min(ncols.saturating_sub(1));
        }
    }

    pub fn scroll_left(&mut self, n: usize) {
        self.scroll_col = self.scroll_col.saturating_sub(n);
    }

    /// Total number of rows in the data.
    pub fn total_rows(&self) -> usize {
        self.data.len()
    }

    /// Total number of columns in the data.
    pub fn total_cols(&self) -> usize {
        self.col_headers.len()
    }

    /// Render the table preview as a centered modal overlay.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }
        if area.width < 16 || area.height < 6 {
            return;
        }

        // Size the modal to fit available space
        let modal_w = area.width.min(76).max(24);
        let modal_h = area.height.min(30).max(8);
        let x = area.x + (area.width.saturating_sub(modal_w)) / 2;
        let y = area.y + (area.height.saturating_sub(modal_h)) / 2;
        let modal_area = Rect::new(x, y, modal_w, modal_h);

        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(format!(" Table: {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));
        let inner = block.inner(modal_area);
        Widget::render(block, modal_area, buf);

        if inner.width < 8 || inner.height < 3 {
            return;
        }

        // Determine how many columns we can fit
        let col_w: u16 = 12; // width per data column
        let row_hdr_w: u16 = 10; // width for the row header
        let avail_w = inner.width.saturating_sub(row_hdr_w);
        let visible_cols = (avail_w / col_w).max(1) as usize;
        let col_start = self.scroll_col;
        let col_end = (col_start + visible_cols).min(self.col_headers.len());

        // Build header row
        let mut header_cells = vec![Cell::from("Idx").style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )];
        for ci in col_start..col_end {
            header_cells.push(
                Cell::from(truncate(&self.col_headers[ci], col_w as usize - 1)).style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            );
        }
        let header = TableRow::new(header_cells);

        // Build visible rows
        let visible_rows = (inner.height.saturating_sub(3)) as usize; // header + hints
        let row_start = self.scroll_row;
        let row_end = (row_start + visible_rows).min(self.data.len());

        let rows: Vec<TableRow> = (row_start..row_end)
            .map(|ri| {
                let mut cells = vec![Cell::from(truncate(&self.row_headers[ri], row_hdr_w as usize - 1))
                    .style(Style::default().fg(Color::DarkGray))];
                for ci in col_start..col_end {
                    let val = self
                        .data
                        .get(ri)
                        .and_then(|row| row.get(ci))
                        .copied()
                        .unwrap_or(f64::NAN);
                    let style = if val.is_nan() {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    cells.push(Cell::from(format_cell(val)).style(style));
                }
                TableRow::new(cells)
            })
            .collect();

        // Column width constraints
        let mut widths = vec![Constraint::Length(row_hdr_w)];
        for _ in col_start..col_end {
            widths.push(Constraint::Length(col_w));
        }

        let table = Table::new(rows, widths).header(header);

        // Leave room for hints at the bottom
        let table_h = inner.height.saturating_sub(1);
        let table_area = Rect::new(inner.x, inner.y, inner.width, table_h);
        Widget::render(table, table_area, buf);

        // Scroll hints
        let hint_y = inner.y + inner.height - 1;
        let scroll_info = format!(
            "Row {}-{}/{} Col {}-{}/{} | \u{2191}\u{2193}\u{2190}\u{2192} scroll  Esc close",
            row_start + 1,
            row_end,
            self.data.len(),
            col_start + 1,
            col_end,
            self.col_headers.len(),
        );
        let hint_area = Rect::new(inner.x, hint_y, inner.width, 1);
        let hint_line = Line::from(Span::styled(
            truncate(&scroll_info, inner.width as usize),
            Style::default().fg(Color::DarkGray),
        ));
        Widget::render(hint_line, hint_area, buf);
    }
}

fn format_cell(v: f64) -> String {
    if v.is_nan() {
        "NaN".to_string()
    } else if v.is_infinite() {
        if v.is_sign_positive() {
            "Inf".to_string()
        } else {
            "-Inf".to_string()
        }
    } else if v == v.trunc() && v.abs() < 1e9 {
        format!("{}", v as i64)
    } else if v.abs() >= 1e6 || (v != 0.0 && v.abs() < 1e-3) {
        format!("{:.3e}", v)
    } else {
        format!("{:.4}", v)
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max_len.saturating_sub(1)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_1d() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let tp = TablePreview::from_1d(&data, "temperature", "lat", None);
        assert_eq!(tp.total_rows(), 5);
        assert_eq!(tp.total_cols(), 1);
        assert_eq!(tp.col_headers, vec!["Value"]);
        assert_eq!(tp.row_headers, vec!["0", "1", "2", "3", "4"]);
    }

    #[test]
    fn test_from_1d_with_coords() {
        let data = vec![10.0, 20.0, 30.0];
        let coords = vec![-90.0, 0.0, 90.0];
        let tp = TablePreview::from_1d(&data, "temp", "lat", Some(&coords));
        assert_eq!(tp.row_headers, vec!["-90", "0", "90"]);
    }

    #[test]
    fn test_from_2d() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        let tp = TablePreview::from_2d(&data, "sst", "lat", "lon", None, None);
        assert_eq!(tp.total_rows(), 3);
        assert_eq!(tp.total_cols(), 2);
        assert!(tp.title.contains("lat"));
        assert!(tp.title.contains("lon"));
    }

    #[test]
    fn test_from_2d_with_coords() {
        let data = vec![vec![1.0, 2.0]];
        let row_c = vec![45.0];
        let col_c = vec![10.5, 20.5];
        let tp = TablePreview::from_2d(&data, "v", "y", "x", Some(&row_c), Some(&col_c));
        assert_eq!(tp.row_headers, vec!["45"]);
        assert_eq!(tp.col_headers, vec!["10.5000", "20.5000"]);
    }

    #[test]
    fn test_scroll() {
        let data: Vec<Vec<f64>> = (0..50).map(|r| vec![r as f64]).collect();
        let mut tp = TablePreview::from_1d(
            &data.iter().map(|r| r[0]).collect::<Vec<_>>(),
            "v",
            "x",
            None,
        );
        assert_eq!(tp.scroll_row, 0);
        tp.scroll_down(10);
        assert_eq!(tp.scroll_row, 10);
        tp.scroll_up(3);
        assert_eq!(tp.scroll_row, 7);
        tp.scroll_up(100);
        assert_eq!(tp.scroll_row, 0);
        tp.scroll_down(1000);
        assert_eq!(tp.scroll_row, 49);
    }

    #[test]
    fn test_scroll_cols() {
        let data = vec![vec![1.0; 20]];
        let cols: Vec<String> = (0..20).map(|i| format!("c{i}")).collect();
        let mut tp = TablePreview {
            visible: true,
            title: "test".into(),
            col_headers: cols,
            row_headers: vec!["0".into()],
            data,
            scroll_row: 0,
            scroll_col: 0,
        };
        tp.scroll_right(5);
        assert_eq!(tp.scroll_col, 5);
        tp.scroll_left(2);
        assert_eq!(tp.scroll_col, 3);
        tp.scroll_right(100);
        assert_eq!(tp.scroll_col, 19);
    }

    #[test]
    fn test_format_cell() {
        assert_eq!(format_cell(42.0), "42");
        assert_eq!(format_cell(f64::NAN), "NaN");
        assert_eq!(format_cell(f64::INFINITY), "Inf");
        assert_eq!(format_cell(f64::NEG_INFINITY), "-Inf");
        assert_eq!(format_cell(3.14159), "3.1416");
        // Large integers use integer format, fractional large values use sci
        assert_eq!(format_cell(100000000.0), "100000000");
        assert!(format_cell(1.5e6 + 0.1).contains('e'));
        assert!(format_cell(1e-5).contains('e'));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 6), "hello\u{2026}");
    }

    #[test]
    fn test_hidden_no_panic() {
        let tp = TablePreview {
            visible: false,
            title: String::new(),
            col_headers: vec![],
            row_headers: vec![],
            data: vec![],
            scroll_row: 0,
            scroll_col: 0,
        };
        let area = Rect::new(0, 0, 60, 20);
        let mut buf = Buffer::empty(area);
        tp.render(area, &mut buf); // should not panic
    }
}
