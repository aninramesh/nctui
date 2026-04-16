use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

/// A 2D heatmap panel rendering data with Unicode block characters.
pub struct HeatmapPanel {
    pub data: Vec<Vec<f64>>,
    pub row_labels: Vec<String>,
    pub col_labels: Vec<String>,
    pub title: String,
    pub vmin: f64,
    pub vmax: f64,
}

/// Unicode block characters for 4-level quantization.
const BLOCKS: [char; 4] = ['\u{2591}', '\u{2592}', '\u{2593}', '\u{2588}']; // ░▒▓█

impl HeatmapPanel {
    pub fn new(data: Vec<Vec<f64>>, title: &str) -> Self {
        let (vmin, vmax) = data_range(&data);
        let nrows = data.len();
        let ncols = data.first().map_or(0, |r| r.len());
        let row_labels: Vec<String> = (0..nrows).map(|i| format!("{i}")).collect();
        let col_labels: Vec<String> = (0..ncols).map(|i| format!("{i}")).collect();
        Self {
            data,
            row_labels,
            col_labels,
            title: title.to_string(),
            vmin,
            vmax,
        }
    }

    /// Map a value to a color using a blue-to-red palette.
    fn value_color(&self, v: f64) -> Color {
        if v.is_nan() {
            return Color::DarkGray;
        }
        let range = self.vmax - self.vmin;
        let t = if range.abs() < 1e-12 {
            0.5
        } else {
            ((v - self.vmin) / range).clamp(0.0, 1.0)
        };
        // Blue → Cyan → Green → Yellow → Red
        let (r, g, b) = if t < 0.25 {
            let s = t / 0.25;
            (0.0, s, 1.0)
        } else if t < 0.5 {
            let s = (t - 0.25) / 0.25;
            (0.0, 1.0, 1.0 - s)
        } else if t < 0.75 {
            let s = (t - 0.5) / 0.25;
            (s, 1.0, 0.0)
        } else {
            let s = (t - 0.75) / 0.25;
            (1.0, 1.0 - s, 0.0)
        };
        Color::Rgb(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
        )
    }

    /// Map a value to one of the 4 block characters.
    fn value_block(&self, v: f64) -> char {
        if v.is_nan() {
            return ' ';
        }
        let range = self.vmax - self.vmin;
        let t = if range.abs() < 1e-12 {
            0.5
        } else {
            ((v - self.vmin) / range).clamp(0.0, 1.0)
        };
        let idx = ((t * 3.99) as usize).min(3);
        BLOCKS[idx]
    }

    /// Render the heatmap into a buffer area.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        Widget::render(block, area, buf);

        if inner.width < 4 || inner.height < 2 {
            return;
        }

        let nrows = self.data.len();
        let ncols = self.data.first().map_or(0, |r| r.len());
        if nrows == 0 || ncols == 0 {
            return;
        }

        // Reserve 8 cols on the right for the color legend
        let legend_w: u16 = 8;
        let plot_w = inner.width.saturating_sub(legend_w + 1);
        let plot_h = inner.height.saturating_sub(1); // -1 for x-axis label

        // Render heatmap cells (downsample if needed)
        for ty in 0..plot_h.min(nrows as u16) {
            let data_row = (ty as usize * nrows) / plot_h.max(1) as usize;
            let data_row = data_row.min(nrows - 1);
            for tx in 0..plot_w.min(ncols as u16) {
                let data_col = (tx as usize * ncols) / plot_w.max(1) as usize;
                let data_col = data_col.min(ncols - 1);
                let v = self.data[data_row][data_col];
                let ch = self.value_block(v);
                let color = self.value_color(v);
                let cell = buf.cell_mut((inner.x + tx, inner.y + ty));
                if let Some(cell) = cell {
                    cell.set_char(ch);
                    cell.set_fg(color);
                }
            }
        }

        // Render color legend
        if inner.width > legend_w + 2 {
            let lx = inner.x + plot_w + 1;
            let legend_h = plot_h.max(1);
            for ly in 0..legend_h {
                let t = 1.0 - (ly as f64 / (legend_h.max(1) - 1).max(1) as f64);
                let color = self.value_color(self.vmin + t * (self.vmax - self.vmin));
                let ch = BLOCKS[3]; // █
                let cell = buf.cell_mut((lx, inner.y + ly));
                if let Some(cell) = cell {
                    cell.set_char(ch);
                    cell.set_fg(color);
                }
            }
            // Max label
            let max_label = format!("{:.1}", self.vmax);
            let max_area = Rect::new(lx + 1, inner.y, legend_w - 1, 1);
            let max_line = Line::from(Span::styled(
                max_label,
                Style::default().fg(Color::White),
            ));
            Widget::render(max_line, max_area, buf);
            // Min label
            if plot_h > 1 {
                let min_label = format!("{:.1}", self.vmin);
                let min_area = Rect::new(lx + 1, inner.y + plot_h - 1, legend_w - 1, 1);
                let min_line = Line::from(Span::styled(
                    min_label,
                    Style::default().fg(Color::White),
                ));
                Widget::render(min_line, min_area, buf);
            }
        }

        // X-axis label
        if inner.height > 1 {
            let xlabel = if self.col_labels.is_empty() {
                String::new()
            } else {
                format!(
                    "{} ... {}",
                    self.col_labels.first().unwrap(),
                    self.col_labels.last().unwrap()
                )
            };
            let xarea = Rect::new(inner.x, inner.y + plot_h, plot_w, 1);
            let xline = Line::from(Span::styled(xlabel, Style::default().fg(Color::DarkGray)));
            Widget::render(xline, xarea, buf);
        }
    }
}

fn data_range(data: &[Vec<f64>]) -> (f64, f64) {
    let mut vmin = f64::INFINITY;
    let mut vmax = f64::NEG_INFINITY;
    for row in data {
        for &v in row {
            if v.is_finite() {
                vmin = vmin.min(v);
                vmax = vmax.max(v);
            }
        }
    }
    if vmin > vmax {
        (0.0, 1.0)
    } else {
        (vmin, vmax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_range() {
        let data = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let (lo, hi) = data_range(&data);
        assert!((lo - 1.0).abs() < 1e-9);
        assert!((hi - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_data_range_with_nan() {
        let data = vec![vec![f64::NAN, 5.0], vec![2.0, f64::NAN]];
        let (lo, hi) = data_range(&data);
        assert!((lo - 2.0).abs() < 1e-9);
        assert!((hi - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_data_range_empty() {
        let data: Vec<Vec<f64>> = vec![];
        let (lo, hi) = data_range(&data);
        assert!((lo - 0.0).abs() < 1e-9);
        assert!((hi - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_block_char_selection() {
        let hm = HeatmapPanel::new(vec![vec![0.0, 0.5, 1.0]], "test");
        assert_eq!(hm.value_block(0.0), BLOCKS[0]);
        assert_eq!(hm.value_block(1.0), BLOCKS[3]);
        assert_eq!(hm.value_block(f64::NAN), ' ');
    }
}
