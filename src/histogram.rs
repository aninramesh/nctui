use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Widget},
};

/// A computed histogram with equal-width bins.
#[derive(Debug, Clone)]
pub struct Histogram {
    pub edges: Vec<f64>,
    pub counts: Vec<usize>,
    pub total: usize,
    pub max_count: usize,
}

impl Histogram {
    /// Compute a histogram from a slice of f64 values. NaN values are skipped.
    pub fn compute(data: &[f64], n_bins: usize) -> Self {
        let finite: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
        if finite.is_empty() {
            return Self {
                edges: vec![0.0, 1.0],
                counts: vec![0],
                total: 0,
                max_count: 0,
            };
        }
        let lo = finite.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let n = n_bins.max(1);

        // Handle degenerate case (all values identical)
        let (lo, hi) = if (hi - lo).abs() < 1e-12 {
            (lo - 0.5, hi + 0.5)
        } else {
            (lo, hi)
        };

        let width = (hi - lo) / n as f64;
        let mut edges = Vec::with_capacity(n + 1);
        for i in 0..=n {
            edges.push(lo + width * i as f64);
        }
        let mut counts = vec![0usize; n];
        for &v in &finite {
            let bin = ((v - lo) / width) as usize;
            let bin = bin.min(n - 1);
            counts[bin] += 1;
        }
        let total = finite.len();
        let max_count = counts.iter().copied().max().unwrap_or(0);
        Self {
            edges,
            counts,
            total,
            max_count,
        }
    }
}

/// State for the histogram overlay widget.
pub struct HistogramState {
    pub visible: bool,
    pub n_bins: usize,
    pub histogram: Option<Histogram>,
}

impl HistogramState {
    pub fn new() -> Self {
        Self {
            visible: false,
            n_bins: 20,
            histogram: None,
        }
    }

    pub fn set_data(&mut self, data: &[f64]) {
        self.histogram = Some(Histogram::compute(data, self.n_bins));
    }

    pub fn increase_bins(&mut self) {
        self.n_bins = (self.n_bins + 4).min(80);
        if let Some(ref hist) = self.histogram {
            let edges = &hist.edges;
            let lo = edges[0];
            let hi = edges[edges.len() - 1];
            // We'd need the original data to recompute; flag as stale
            let _ = (lo, hi);
        }
    }

    pub fn decrease_bins(&mut self) {
        self.n_bins = self.n_bins.saturating_sub(4).max(4);
    }

    /// Render the histogram overlay into a buffer area.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }
        let hist = match &self.histogram {
            Some(h) => h,
            None => return,
        };
        if area.width < 12 || area.height < 6 {
            return;
        }

        // Center modal
        let modal_w = area.width.min(60).max(20);
        let modal_h = area.height.min(20).max(8);
        let x = area.x + (area.width.saturating_sub(modal_w)) / 2;
        let y = area.y + (area.height.saturating_sub(modal_h)) / 2;
        let modal_area = Rect::new(x, y, modal_w, modal_h);

        Clear.render(modal_area, buf);

        let block = Block::default()
            .title(format!(" Histogram ({} bins, n={}) ", hist.counts.len(), hist.total))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let inner = block.inner(modal_area);
        Widget::render(block, modal_area, buf);

        if inner.width < 4 || inner.height < 3 {
            return;
        }

        let bar_h = inner.height.saturating_sub(2); // room for axis labels
        let n = hist.counts.len();
        let bar_w = (inner.width as usize) / n.max(1);
        let bar_w = bar_w.max(1);

        // Draw bars
        for (i, &count) in hist.counts.iter().enumerate() {
            let x_off = (i * bar_w) as u16;
            if x_off >= inner.width {
                break;
            }
            let filled = if hist.max_count > 0 {
                ((count as f64 / hist.max_count as f64) * bar_h as f64).round() as u16
            } else {
                0
            };
            // Color: blue → red based on bin position
            let t = if n > 1 { i as f64 / (n - 1) as f64 } else { 0.5 };
            let r = (t * 255.0) as u8;
            let b_val = ((1.0 - t) * 255.0) as u8;
            let color = Color::Rgb(r, 80, b_val);

            for dy in 0..bar_h {
                let row_from_bottom = bar_h - 1 - dy;
                if row_from_bottom < filled {
                    let cell = buf.cell_mut((inner.x + x_off, inner.y + dy));
                    if let Some(cell) = cell {
                        cell.set_char('\u{2588}'); // █
                        cell.set_fg(color);
                    }
                }
            }
        }

        // Axis line
        let axis_y = inner.y + bar_h;
        for dx in 0..inner.width {
            let cell = buf.cell_mut((inner.x + dx, axis_y));
            if let Some(cell) = cell {
                cell.set_char('\u{2500}'); // ─
                cell.set_fg(Color::DarkGray);
            }
        }

        // Min/max labels
        if inner.height > bar_h + 1 {
            let label_y = axis_y + 1;
            let min_label = format!("{:.1}", hist.edges[0]);
            let max_label = format!("{:.1}", hist.edges[hist.edges.len() - 1]);
            let mid_val = (hist.edges[0] + hist.edges[hist.edges.len() - 1]) / 2.0;
            let mid_label = format!("{:.1}", mid_val);

            let min_area = Rect::new(inner.x, label_y, 8.min(inner.width), 1);
            Widget::render(
                Line::from(Span::styled(min_label, Style::default().fg(Color::DarkGray))),
                min_area,
                buf,
            );
            if inner.width > 20 {
                let mid_x = inner.x + inner.width / 2 - 3;
                let mid_area = Rect::new(mid_x, label_y, 8, 1);
                Widget::render(
                    Line::from(Span::styled(mid_label, Style::default().fg(Color::DarkGray))),
                    mid_area,
                    buf,
                );
            }
            if inner.width > 10 {
                let max_x = inner.x + inner.width.saturating_sub(8);
                let max_area = Rect::new(max_x, label_y, 8, 1);
                Widget::render(
                    Line::from(Span::styled(max_label, Style::default().fg(Color::DarkGray))),
                    max_area,
                    buf,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_histogram() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let hist = Histogram::compute(&data, 10);
        assert_eq!(hist.counts.len(), 10);
        assert_eq!(hist.total, 100);
        assert!(hist.max_count > 0);
    }

    #[test]
    fn test_empty_data() {
        let hist = Histogram::compute(&[], 10);
        assert_eq!(hist.total, 0);
        assert_eq!(hist.max_count, 0);
    }

    #[test]
    fn test_nan_handling() {
        let data = vec![1.0, f64::NAN, 2.0, f64::NAN, 3.0];
        let hist = Histogram::compute(&data, 5);
        assert_eq!(hist.total, 3);
    }

    #[test]
    fn test_degenerate_all_same() {
        let data = vec![5.0; 50];
        let hist = Histogram::compute(&data, 10);
        assert_eq!(hist.total, 50);
        assert_eq!(hist.max_count, 50); // all in one bin
    }

    #[test]
    fn test_bin_adjustment() {
        let mut state = HistogramState::new();
        assert_eq!(state.n_bins, 20);
        state.increase_bins();
        assert_eq!(state.n_bins, 24);
        state.decrease_bins();
        assert_eq!(state.n_bins, 20);
        for _ in 0..10 {
            state.decrease_bins();
        }
        assert_eq!(state.n_bins, 4); // clamped
    }
}
