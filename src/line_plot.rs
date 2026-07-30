use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Widget},
};

/// A 1D line plot panel rendering a series against index or coordinate X.
///
/// Used as the primary visualization for 1D NetCDF variables. When
/// `x_coords` is provided the X-axis uses real coordinate values; otherwise
/// it uses 0-based sample indices.
pub struct LinePlotPanel {
    pub data: Vec<f64>,
    /// Optional X coordinates (must match `data.len()` when used).
    pub x_coords: Option<Vec<f64>>,
    pub title: String,
    pub dim_name: String,
    pub ymin: f64,
    pub ymax: f64,
    pub xmin: f64,
    pub xmax: f64,
}

impl LinePlotPanel {
    /// Create a line plot from a 1D data vector.
    pub fn new(data: &[f64], title: &str, dim_name: &str, x_coords: Option<&[f64]>) -> Self {
        let (ymin, ymax) = y_range(data);
        let coords = x_coords
            .filter(|c| c.len() == data.len())
            .map(|c| c.to_vec());
        let (xmin, xmax) = match &coords {
            Some(c) => x_range(c),
            None => {
                if data.is_empty() {
                    (0.0, 1.0)
                } else {
                    (0.0, (data.len().saturating_sub(1)) as f64)
                }
            }
        };
        Self {
            data: data.to_vec(),
            x_coords: coords,
            title: title.to_string(),
            dim_name: dim_name.to_string(),
            ymin,
            ymax,
            xmin,
            xmax,
        }
    }

    /// Build `(x, y)` points, skipping non-finite Y values.
    fn points(&self) -> Vec<(f64, f64)> {
        self.data
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| {
                if !v.is_finite() {
                    return None;
                }
                let x = self
                    .x_coords
                    .as_ref()
                    .and_then(|c| c.get(i).copied())
                    .filter(|x| x.is_finite())
                    .unwrap_or(i as f64);
                Some((x, v))
            })
            .collect()
    }

    /// Render the line plot into a buffer area.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let title = format!(" {} [{}] ", self.title, self.dim_name);
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));

        if self.data.is_empty() || area.width < 8 || area.height < 4 {
            let inner = block.inner(area);
            Widget::render(block, area, buf);
            if inner.width >= 10 && inner.height >= 1 {
                let msg = Line::from(Span::styled(
                    "No data to plot",
                    Style::default().fg(Color::DarkGray),
                ));
                Widget::render(msg, inner, buf);
            }
            return;
        }

        let points = self.points();
        if points.is_empty() {
            let inner = block.inner(area);
            Widget::render(block, area, buf);
            if inner.width >= 14 && inner.height >= 1 {
                let msg = Line::from(Span::styled(
                    "No finite values",
                    Style::default().fg(Color::DarkGray),
                ));
                Widget::render(msg, inner, buf);
            }
            return;
        }

        // Chart needs a slight Y pad so points aren't clipped at the border
        let y_span = (self.ymax - self.ymin).abs().max(1e-12);
        let y_pad = y_span * 0.05;
        let y_bounds = [self.ymin - y_pad, self.ymax + y_pad];

        let x_span = (self.xmax - self.xmin).abs().max(1e-12);
        let x_pad = if self.x_coords.is_some() {
            x_span * 0.02
        } else {
            0.0
        };
        let x_bounds = [self.xmin - x_pad, self.xmax + x_pad];

        let datasets = vec![Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&points)];

        let x_labels = [
            format_axis(self.xmin),
            format_axis((self.xmin + self.xmax) / 2.0),
            format_axis(self.xmax),
        ];
        let y_labels = [
            format_axis(self.ymin),
            format_axis((self.ymin + self.ymax) / 2.0),
            format_axis(self.ymax),
        ];

        let chart = Chart::new(datasets)
            .block(block)
            .x_axis(
                Axis::default()
                    .title(self.dim_name.as_str())
                    .style(Style::default().fg(Color::DarkGray))
                    .bounds(x_bounds)
                    .labels(x_labels),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(Color::DarkGray))
                    .bounds(y_bounds)
                    .labels(y_labels),
            );

        Widget::render(chart, area, buf);
    }
}

fn y_range(data: &[f64]) -> (f64, f64) {
    let mut ymin = f64::INFINITY;
    let mut ymax = f64::NEG_INFINITY;
    for &v in data {
        if v.is_finite() {
            ymin = ymin.min(v);
            ymax = ymax.max(v);
        }
    }
    if ymin > ymax {
        (0.0, 1.0)
    } else if (ymax - ymin).abs() < 1e-12 {
        // Degenerate flat series: give the axis a small span
        (ymin - 0.5, ymax + 0.5)
    } else {
        (ymin, ymax)
    }
}

fn x_range(coords: &[f64]) -> (f64, f64) {
    let mut xmin = f64::INFINITY;
    let mut xmax = f64::NEG_INFINITY;
    for &v in coords {
        if v.is_finite() {
            xmin = xmin.min(v);
            xmax = xmax.max(v);
        }
    }
    if xmin > xmax {
        (0.0, 1.0)
    } else if (xmax - xmin).abs() < 1e-12 {
        (xmin - 0.5, xmax + 0.5)
    } else {
        (xmin, xmax)
    }
}

/// Compact axis label: integers without decimals when possible.
fn format_axis(v: f64) -> String {
    if !v.is_finite() {
        return "NaN".to_string();
    }
    if v == v.trunc() && v.abs() < 1e9 {
        format!("{}", v as i64)
    } else if v.abs() >= 1000.0 || (v.abs() > 0.0 && v.abs() < 0.01) {
        format!("{:.2e}", v)
    } else {
        format!("{:.2}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let plot = LinePlotPanel::new(&data, "temp", "time", None);
        assert_eq!(plot.data.len(), 4);
        assert_eq!(plot.title, "temp");
        assert_eq!(plot.dim_name, "time");
        assert!((plot.ymin - 1.0).abs() < 1e-9);
        assert!((plot.ymax - 4.0).abs() < 1e-9);
        assert!((plot.xmin - 0.0).abs() < 1e-9);
        assert!((plot.xmax - 3.0).abs() < 1e-9);
        assert!(plot.x_coords.is_none());
    }

    #[test]
    fn test_new_with_coords() {
        let data = vec![10.0, 20.0, 30.0];
        let coords = vec![-90.0, 0.0, 90.0];
        let plot = LinePlotPanel::new(&data, "lat_val", "lat", Some(&coords));
        assert_eq!(plot.x_coords.as_ref().unwrap().len(), 3);
        assert!((plot.xmin - (-90.0)).abs() < 1e-9);
        assert!((plot.xmax - 90.0).abs() < 1e-9);
        assert!((plot.ymin - 10.0).abs() < 1e-9);
        assert!((plot.ymax - 30.0).abs() < 1e-9);
    }

    #[test]
    fn test_coords_length_mismatch_falls_back() {
        let data = vec![1.0, 2.0, 3.0];
        let coords = vec![0.0, 1.0]; // wrong length
        let plot = LinePlotPanel::new(&data, "v", "x", Some(&coords));
        assert!(plot.x_coords.is_none());
        assert!((plot.xmax - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_nan_skipped_in_range() {
        let data = vec![f64::NAN, 5.0, f64::NAN, 15.0];
        let plot = LinePlotPanel::new(&data, "v", "i", None);
        assert!((plot.ymin - 5.0).abs() < 1e-9);
        assert!((plot.ymax - 15.0).abs() < 1e-9);
        assert_eq!(plot.points().len(), 2);
    }

    #[test]
    fn test_empty_data() {
        let plot = LinePlotPanel::new(&[], "empty", "x", None);
        assert!(plot.data.is_empty());
        assert!((plot.ymin - 0.0).abs() < 1e-9);
        assert!((plot.ymax - 1.0).abs() < 1e-9);
        assert!(plot.points().is_empty());
    }

    #[test]
    fn test_degenerate_flat() {
        let data = vec![3.0; 10];
        let plot = LinePlotPanel::new(&data, "flat", "t", None);
        assert!(plot.ymax > plot.ymin);
        assert!((plot.ymin - 2.5).abs() < 1e-9);
        assert!((plot.ymax - 3.5).abs() < 1e-9);
    }

    #[test]
    fn test_all_nan() {
        let data = vec![f64::NAN; 5];
        let plot = LinePlotPanel::new(&data, "nan", "t", None);
        assert!(plot.points().is_empty());
        // y_range falls back to 0..1
        assert!((plot.ymin - 0.0).abs() < 1e-9);
        assert!((plot.ymax - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_format_axis() {
        assert_eq!(format_axis(42.0), "42");
        assert_eq!(format_axis(-90.0), "-90");
        assert_eq!(format_axis(1.5), "1.50");
        assert_eq!(format_axis(f64::NAN), "NaN");
    }

    #[test]
    fn test_render_does_not_panic() {
        let data: Vec<f64> = (0..50).map(|i| (i as f64 * 0.2).sin()).collect();
        let plot = LinePlotPanel::new(&data, "signal", "time", None);
        let area = Rect::new(0, 0, 60, 20);
        let mut buf = Buffer::empty(area);
        plot.render(area, &mut buf);
    }

    #[test]
    fn test_render_empty_does_not_panic() {
        let plot = LinePlotPanel::new(&[], "empty", "x", None);
        let area = Rect::new(0, 0, 40, 12);
        let mut buf = Buffer::empty(area);
        plot.render(area, &mut buf);
    }
}
