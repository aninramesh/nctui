use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Row as TableRow, Table, Widget},
};

/// Role assigned to a dimension in a slice specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DimRole {
    Fixed(usize),
    AxisX,
    AxisY,
}

/// Per-dimension state within a slice spec.
#[derive(Debug, Clone)]
pub struct DimState {
    pub name: String,
    pub size: usize,
    pub role: DimRole,
}

/// Complete hyperslab specification for slicing an nD variable to 1D or 2D.
///
/// - **1 free axis** (X or Y) → line plot along that dimension
/// - **2 free axes** (X and Y) → 2D heatmap
///
/// Remaining dimensions are fixed at a chosen index. Works for any rank
/// (3D, 4D, 5D, …).
#[derive(Debug, Clone)]
pub struct SliceSpec {
    pub var_name: String,
    pub dims: Vec<DimState>,
}

impl SliceSpec {
    /// Create a default heatmap-oriented spec: last dim → X, second-last → Y,
    /// rest fixed at 0. For rank-1 variables the sole dim is X (line plot).
    pub fn default_for(var_name: &str, dim_names: &[String], dim_sizes: &[usize]) -> Self {
        let n = dim_names.len();
        let dims = dim_names
            .iter()
            .zip(dim_sizes.iter())
            .enumerate()
            .map(|(i, (name, &size))| {
                let role = if n == 1 && i == 0 {
                    DimRole::AxisX
                } else if n >= 2 && i == n - 1 {
                    DimRole::AxisX
                } else if n >= 2 && i == n - 2 {
                    DimRole::AxisY
                } else {
                    DimRole::Fixed(0)
                };
                DimState {
                    name: name.clone(),
                    size,
                    role,
                }
            })
            .collect();
        Self {
            var_name: var_name.to_string(),
            dims,
        }
    }

    /// Line-plot mode: last dimension free (X), every earlier dim fixed at 0.
    pub fn default_line_for(var_name: &str, dim_names: &[String], dim_sizes: &[usize]) -> Self {
        let n = dim_names.len();
        let dims = dim_names
            .iter()
            .zip(dim_sizes.iter())
            .enumerate()
            .map(|(i, (name, &size))| {
                let role = if n >= 1 && i == n - 1 {
                    DimRole::AxisX
                } else {
                    DimRole::Fixed(0)
                };
                DimState {
                    name: name.clone(),
                    size,
                    role,
                }
            })
            .collect();
        Self {
            var_name: var_name.to_string(),
            dims,
        }
    }

    /// Reset roles to heatmap mode (2 free axes when rank ≥ 2).
    pub fn set_heatmap_mode(&mut self) {
        let n = self.dims.len();
        for (i, d) in self.dims.iter_mut().enumerate() {
            d.role = if n == 1 && i == 0 {
                DimRole::AxisX
            } else if n >= 2 && i == n - 1 {
                DimRole::AxisX
            } else if n >= 2 && i == n - 2 {
                DimRole::AxisY
            } else {
                DimRole::Fixed(0)
            };
        }
    }

    /// Reset roles to line mode (1 free axis: last dim).
    pub fn set_line_mode(&mut self) {
        let n = self.dims.len();
        for (i, d) in self.dims.iter_mut().enumerate() {
            d.role = if n >= 1 && i == n - 1 {
                DimRole::AxisX
            } else {
                DimRole::Fixed(0)
            };
        }
    }

    /// Count of free (non-fixed) dimensions.
    pub fn free_dim_count(&self) -> usize {
        self.dims
            .iter()
            .filter(|d| !matches!(d.role, DimRole::Fixed(_)))
            .count()
    }

    /// Whether the current free-axis assignment is valid for plotting.
    ///
    /// Accepts 1 free axis (line) or 2 free axes with both X and Y (heatmap).
    pub fn is_valid_plot(&self) -> bool {
        match self.free_dim_count() {
            1 => self.line_axis().is_some(),
            2 => self.xy_axes().is_some(),
            _ => false,
        }
    }

    /// Return the free axis index when exactly one dimension is free (X or Y).
    pub fn line_axis(&self) -> Option<usize> {
        if self.free_dim_count() != 1 {
            return None;
        }
        self.dims
            .iter()
            .position(|d| matches!(d.role, DimRole::AxisX | DimRole::AxisY))
    }

    /// Return (row_axis_idx, col_axis_idx) for the two free axes.
    pub fn xy_axes(&self) -> Option<(usize, usize)> {
        let y = self.dims.iter().position(|d| d.role == DimRole::AxisY)?;
        let x = self.dims.iter().position(|d| d.role == DimRole::AxisX)?;
        Some((y, x))
    }

    /// Assign a dimension to a given role, bumping any existing holder of that role to Fixed(0).
    pub fn assign_axis(&mut self, dim_idx: usize, role: DimRole) {
        if matches!(role, DimRole::AxisX | DimRole::AxisY) {
            for d in &mut self.dims {
                if d.role == role {
                    d.role = DimRole::Fixed(0);
                }
            }
        }
        self.dims[dim_idx].role = role;
    }

    /// Increment the fixed index of a dimension (wraps at size).
    pub fn increment_fixed(&mut self, dim_idx: usize) {
        let size = self.dims[dim_idx].size;
        if let DimRole::Fixed(ref mut idx) = self.dims[dim_idx].role {
            *idx = (*idx + 1) % size;
        }
    }

    /// Decrement the fixed index of a dimension (wraps at size).
    pub fn decrement_fixed(&mut self, dim_idx: usize) {
        let size = self.dims[dim_idx].size;
        if let DimRole::Fixed(ref mut idx) = self.dims[dim_idx].role {
            if *idx == 0 {
                *idx = size - 1;
            } else {
                *idx -= 1;
            }
        }
    }

    /// Human-readable summary of fixed dimensions, e.g. `time=0, level=3`.
    pub fn fixed_summary(&self) -> String {
        self.dims
            .iter()
            .filter_map(|d| {
                if let DimRole::Fixed(idx) = d.role {
                    Some(format!("{}={}", d.name, idx))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Modal widget state for the slice picker.
pub struct SlicePicker {
    pub visible: bool,
    pub spec: SliceSpec,
    pub selected: usize,
}

impl SlicePicker {
    pub fn new(spec: SliceSpec) -> Self {
        Self {
            visible: true,
            spec,
            selected: 0,
        }
    }

    /// Render the slice picker modal into a buffer area.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible || area.width < 10 || area.height < 6 {
            return;
        }
        // Center modal — extra row for mode hint
        let modal_w = area.width.min(56).max(30);
        let modal_h = (self.spec.dims.len() as u16 + 6).min(area.height);
        let x = area.x + (area.width.saturating_sub(modal_w)) / 2;
        let y = area.y + (area.height.saturating_sub(modal_h)) / 2;
        let modal_area = Rect::new(x, y, modal_w, modal_h);

        Clear.render(modal_area, buf);

        let mode = match self.spec.free_dim_count() {
            1 => "line plot",
            2 if self.spec.xy_axes().is_some() => "heatmap",
            n => {
                if n == 0 {
                    "fix all dims"
                } else {
                    "need 1 or 2 free"
                }
            }
        };
        let title = format!(" Slice: {} → {} ", self.spec.var_name, mode);

        let header = TableRow::new(vec![
            Cell::from("Dim").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Size").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Role").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Index").style(Style::default().add_modifier(Modifier::BOLD)),
        ]);

        let rows: Vec<TableRow> = self
            .spec
            .dims
            .iter()
            .enumerate()
            .map(|(i, dim)| {
                let style = if i == self.selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let role_str = match &dim.role {
                    DimRole::Fixed(_) => "Fixed",
                    DimRole::AxisX => "X-axis",
                    DimRole::AxisY => "Y-axis",
                };
                let idx_str = match &dim.role {
                    DimRole::Fixed(idx) => format!("{idx}"),
                    _ => "-".to_string(),
                };
                TableRow::new(vec![
                    Cell::from(dim.name.clone()).style(style),
                    Cell::from(format!("{}", dim.size)).style(style),
                    Cell::from(role_str).style(style),
                    Cell::from(idx_str).style(style),
                ])
            })
            .collect();

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));

        let widths = [
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(6),
        ];
        // Leave bottom row for hints
        let table_area = Rect::new(
            modal_area.x,
            modal_area.y,
            modal_area.width,
            modal_area.height.saturating_sub(1),
        );
        let table = Table::new(rows, widths)
            .header(header)
            .block(block);
        Widget::render(table, table_area, buf);

        // Keybind hints at bottom
        if modal_area.height > 3 {
            let hint_y = modal_area.y + modal_area.height - 1;
            let hint = Line::from(vec![
                Span::styled("x", Style::default().fg(Color::Green)),
                Span::raw("/"),
                Span::styled("y", Style::default().fg(Color::Green)),
                Span::raw("/"),
                Span::styled("f", Style::default().fg(Color::Green)),
                Span::raw("  "),
                Span::styled("1", Style::default().fg(Color::Green)),
                Span::raw(" line "),
                Span::styled("2", Style::default().fg(Color::Green)),
                Span::raw(" map  "),
                Span::styled("h", Style::default().fg(Color::Green)),
                Span::raw("/"),
                Span::styled("l", Style::default().fg(Color::Green)),
                Span::raw(" idx  "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw("  "),
                Span::styled("Esc", Style::default().fg(Color::Green)),
            ]);
            let hint_area = Rect::new(modal_area.x + 1, hint_y, modal_area.width - 2, 1);
            Widget::render(hint, hint_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_spec_2d() {
        let spec = SliceSpec::default_for(
            "temp",
            &["lat".into(), "lon".into()],
            &[180, 360],
        );
        assert_eq!(spec.free_dim_count(), 2);
        assert_eq!(spec.dims[0].role, DimRole::AxisY);
        assert_eq!(spec.dims[1].role, DimRole::AxisX);
        assert!(spec.is_valid_plot());
    }

    #[test]
    fn test_default_spec_3d() {
        let spec = SliceSpec::default_for(
            "temp",
            &["time".into(), "lat".into(), "lon".into()],
            &[12, 180, 360],
        );
        assert_eq!(spec.free_dim_count(), 2);
        assert_eq!(spec.dims[0].role, DimRole::Fixed(0));
        assert_eq!(spec.dims[1].role, DimRole::AxisY);
        assert_eq!(spec.dims[2].role, DimRole::AxisX);
    }

    #[test]
    fn test_default_spec_4d() {
        let spec = SliceSpec::default_for(
            "temp",
            &["time".into(), "level".into(), "lat".into(), "lon".into()],
            &[12, 10, 180, 360],
        );
        assert_eq!(spec.free_dim_count(), 2);
        assert_eq!(spec.dims[0].role, DimRole::Fixed(0));
        assert_eq!(spec.dims[1].role, DimRole::Fixed(0));
        assert!(spec.is_valid_plot());
    }

    #[test]
    fn test_default_spec_5d() {
        let names = [
            "ens".into(),
            "time".into(),
            "level".into(),
            "lat".into(),
            "lon".into(),
        ];
        let sizes = [3usize, 12, 10, 180, 360];
        let spec = SliceSpec::default_for("temp", &names, &sizes);
        assert_eq!(spec.dims.len(), 5);
        assert_eq!(spec.free_dim_count(), 2);
        assert_eq!(spec.dims[0].role, DimRole::Fixed(0));
        assert_eq!(spec.dims[1].role, DimRole::Fixed(0));
        assert_eq!(spec.dims[2].role, DimRole::Fixed(0));
        assert_eq!(spec.dims[3].role, DimRole::AxisY);
        assert_eq!(spec.dims[4].role, DimRole::AxisX);
    }

    #[test]
    fn test_line_mode_3d() {
        let mut spec = SliceSpec::default_for(
            "temp",
            &["time".into(), "lat".into(), "lon".into()],
            &[12, 180, 360],
        );
        spec.set_line_mode();
        assert_eq!(spec.free_dim_count(), 1);
        assert_eq!(spec.line_axis(), Some(2)); // lon
        assert!(spec.is_valid_plot());
        assert!(spec.xy_axes().is_none());
    }

    #[test]
    fn test_line_mode_5d() {
        let names = [
            "ens".into(),
            "time".into(),
            "level".into(),
            "lat".into(),
            "lon".into(),
        ];
        let sizes = [3usize, 12, 10, 180, 360];
        let spec = SliceSpec::default_line_for("temp", &names, &sizes);
        assert_eq!(spec.free_dim_count(), 1);
        assert_eq!(spec.line_axis(), Some(4));
        assert_eq!(spec.dims[0].role, DimRole::Fixed(0));
        assert_eq!(spec.dims[4].role, DimRole::AxisX);
    }

    #[test]
    fn test_heatmap_mode_from_line() {
        let mut spec = SliceSpec::default_line_for(
            "temp",
            &["time".into(), "lat".into(), "lon".into()],
            &[12, 180, 360],
        );
        assert_eq!(spec.free_dim_count(), 1);
        spec.set_heatmap_mode();
        assert_eq!(spec.free_dim_count(), 2);
        assert!(spec.xy_axes().is_some());
    }

    #[test]
    fn test_assign_axis_bumps_existing() {
        let mut spec = SliceSpec::default_for(
            "temp",
            &["time".into(), "lat".into(), "lon".into()],
            &[12, 180, 360],
        );
        // lat is Y, lon is X; assign time to X → lon should become Fixed
        spec.assign_axis(0, DimRole::AxisX);
        assert_eq!(spec.dims[0].role, DimRole::AxisX);
        assert_eq!(spec.dims[2].role, DimRole::Fixed(0));
        assert_eq!(spec.free_dim_count(), 2);
    }

    #[test]
    fn test_increment_decrement_fixed() {
        let mut spec = SliceSpec::default_for(
            "temp",
            &["time".into(), "lat".into(), "lon".into()],
            &[12, 180, 360],
        );
        assert_eq!(spec.dims[0].role, DimRole::Fixed(0));
        spec.increment_fixed(0);
        assert_eq!(spec.dims[0].role, DimRole::Fixed(1));
        spec.decrement_fixed(0);
        assert_eq!(spec.dims[0].role, DimRole::Fixed(0));
        spec.decrement_fixed(0); // wrap
        assert_eq!(spec.dims[0].role, DimRole::Fixed(11));
    }

    #[test]
    fn test_xy_axes() {
        let spec = SliceSpec::default_for(
            "temp",
            &["time".into(), "lat".into(), "lon".into()],
            &[12, 180, 360],
        );
        let (y, x) = spec.xy_axes().unwrap();
        assert_eq!(y, 1); // lat
        assert_eq!(x, 2); // lon
    }

    #[test]
    fn test_fixed_summary() {
        let mut spec = SliceSpec::default_for(
            "temp",
            &["time".into(), "level".into(), "lat".into(), "lon".into()],
            &[12, 10, 180, 360],
        );
        spec.increment_fixed(1);
        assert_eq!(spec.fixed_summary(), "time=0, level=1");
    }

    #[test]
    fn test_invalid_when_zero_or_three_free() {
        let mut spec = SliceSpec::default_for(
            "temp",
            &["a".into(), "b".into(), "c".into()],
            &[2, 3, 4],
        );
        // Fix everything → invalid
        spec.assign_axis(1, DimRole::Fixed(0));
        spec.assign_axis(2, DimRole::Fixed(0));
        assert_eq!(spec.free_dim_count(), 0);
        assert!(!spec.is_valid_plot());
    }
}
