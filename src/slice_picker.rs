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

/// Complete hyperslab specification for slicing an nD variable to 2D.
#[derive(Debug, Clone)]
pub struct SliceSpec {
    pub var_name: String,
    pub dims: Vec<DimState>,
}

impl SliceSpec {
    /// Create a default spec: last dim → X, second-last → Y, rest fixed at 0.
    pub fn default_for(var_name: &str, dim_names: &[String], dim_sizes: &[usize]) -> Self {
        let n = dim_names.len();
        let dims = dim_names
            .iter()
            .zip(dim_sizes.iter())
            .enumerate()
            .map(|(i, (name, &size))| {
                let role = if n >= 2 && i == n - 1 {
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

    /// Count of free (non-fixed) dimensions.
    pub fn free_dim_count(&self) -> usize {
        self.dims
            .iter()
            .filter(|d| !matches!(d.role, DimRole::Fixed(_)))
            .count()
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
        // Center modal
        let modal_w = area.width.min(50).max(30);
        let modal_h = (self.spec.dims.len() as u16 + 5).min(area.height);
        let x = area.x + (area.width.saturating_sub(modal_w)) / 2;
        let y = area.y + (area.height.saturating_sub(modal_h)) / 2;
        let modal_area = Rect::new(x, y, modal_w, modal_h);

        Clear.render(modal_area, buf);

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

        let title = format!(" Slice: {} ", self.spec.var_name);
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
        let table = Table::new(rows, widths)
            .header(header)
            .block(block);
        Widget::render(table, modal_area, buf);

        // Keybind hints at bottom
        if modal_area.height > 3 {
            let hint_y = modal_area.y + modal_area.height - 1;
            let hint = Line::from(vec![
                Span::styled("x", Style::default().fg(Color::Green)),
                Span::raw("/"),
                Span::styled("y", Style::default().fg(Color::Green)),
                Span::raw("/"),
                Span::styled("f", Style::default().fg(Color::Green)),
                Span::raw(" role  "),
                Span::styled("h", Style::default().fg(Color::Green)),
                Span::raw("/"),
                Span::styled("l", Style::default().fg(Color::Green)),
                Span::raw(" idx  "),
                Span::styled("Enter", Style::default().fg(Color::Green)),
                Span::raw(" ok  "),
                Span::styled("Esc", Style::default().fg(Color::Green)),
                Span::raw(" cancel"),
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
}
