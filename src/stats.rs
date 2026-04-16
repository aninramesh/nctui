use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

/// Summary statistics computed from a data slice.
#[derive(Debug, Clone)]
pub struct Stats {
    pub count: usize,
    pub valid: usize,
    pub nan_count: usize,
    pub inf_count: usize,
    pub valid_fraction: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub p25: f64,
    pub p75: f64,
    pub p05: f64,
    pub p95: f64,
}

impl Stats {
    /// Compute statistics from a flat slice of f64 data.
    pub fn compute(data: &[f64]) -> Self {
        let count = data.len();
        let mut finite: Vec<f64> = Vec::with_capacity(count);
        let mut nan_count = 0usize;
        let mut inf_count = 0usize;

        for &v in data {
            if v.is_nan() {
                nan_count += 1;
            } else if v.is_infinite() {
                inf_count += 1;
            } else {
                finite.push(v);
            }
        }

        let valid = finite.len();
        let valid_fraction = if count > 0 {
            valid as f64 / count as f64
        } else {
            0.0
        };

        if finite.is_empty() {
            return Self {
                count,
                valid: 0,
                nan_count,
                inf_count,
                valid_fraction,
                min: f64::NAN,
                max: f64::NAN,
                mean: f64::NAN,
                median: f64::NAN,
                std_dev: f64::NAN,
                p25: f64::NAN,
                p75: f64::NAN,
                p05: f64::NAN,
                p95: f64::NAN,
            };
        }

        finite.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min = finite[0];
        let max = finite[finite.len() - 1];
        let sum: f64 = finite.iter().sum();
        let mean = sum / valid as f64;

        let variance = if valid > 1 {
            finite.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (valid - 1) as f64
        } else {
            0.0
        };
        let std_dev = variance.sqrt();

        let median = percentile_sorted(&finite, 50.0);
        let p25 = percentile_sorted(&finite, 25.0);
        let p75 = percentile_sorted(&finite, 75.0);
        let p05 = percentile_sorted(&finite, 5.0);
        let p95 = percentile_sorted(&finite, 95.0);

        Self {
            count,
            valid,
            nan_count,
            inf_count,
            valid_fraction,
            min,
            max,
            mean,
            median,
            std_dev,
            p25,
            p75,
            p05,
            p95,
        }
    }
}

/// Linear interpolation percentile on a pre-sorted slice.
fn percentile_sorted(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = (pct / 100.0) * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - lo as f64;
    if lo == hi {
        sorted[lo]
    } else {
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}

/// Stats panel widget state.
pub struct StatsPanel {
    pub stats: Option<Stats>,
    pub var_name: String,
}

impl StatsPanel {
    pub fn new() -> Self {
        Self {
            stats: None,
            var_name: String::new(),
        }
    }

    pub fn set_data(&mut self, var_name: &str, data: &[f64]) {
        self.var_name = var_name.to_string();
        self.stats = Some(Stats::compute(data));
    }

    pub fn clear(&mut self) {
        self.stats = None;
        self.var_name.clear();
    }

    /// Render the stats panel into the given area.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let title = if self.var_name.is_empty() {
            " Stats ".to_string()
        } else {
            format!(" Stats: {} ", self.var_name)
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));
        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let stats = match &self.stats {
            Some(s) => s,
            None => {
                if inner.width >= 12 && inner.height >= 1 {
                    let msg = Line::from(Span::styled(
                        "Select a variable",
                        Style::default().fg(Color::DarkGray),
                    ));
                    Widget::render(msg, inner, buf);
                }
                return;
            }
        };

        let ls = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let vs = Style::default().fg(Color::White);
        let ds = Style::default().fg(Color::DarkGray);
        let warn = Style::default().fg(Color::Yellow);

        let nan_style = if stats.nan_count > 0 { warn } else { ds };
        let inf_style = if stats.inf_count > 0 { warn } else { ds };

        let lines: Vec<Line> = vec![
            stat_line("Count", &stats.count.to_string(), ls, vs),
            stat_line("Valid", &format!("{} ({:.1}%)", stats.valid, stats.valid_fraction * 100.0), ls, vs),
            stat_line("NaN", &stats.nan_count.to_string(), ls, nan_style),
            stat_line("Inf", &stats.inf_count.to_string(), ls, inf_style),
            separator_line(ds),
            stat_line("Min", &fmt_val(stats.min), ls, vs),
            stat_line("p5", &fmt_val(stats.p05), ls, ds),
            stat_line("p25", &fmt_val(stats.p25), ls, vs),
            stat_line("Median", &fmt_val(stats.median), ls, vs),
            stat_line("p75", &fmt_val(stats.p75), ls, vs),
            stat_line("p95", &fmt_val(stats.p95), ls, ds),
            stat_line("Max", &fmt_val(stats.max), ls, vs),
            separator_line(ds),
            stat_line("Mean", &fmt_val(stats.mean), ls, vs),
            stat_line("Std Dev", &fmt_val(stats.std_dev), ls, vs),
        ];

        for (i, line) in lines.iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }
            let line_area = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
            Widget::render(line.clone(), line_area, buf);
        }
    }
}

fn stat_line(label: &str, value: &str, ls: Style, vs: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<8}", label), ls),
        Span::styled(value.to_string(), vs),
    ])
}

fn separator_line(style: Style) -> Line<'static> {
    Line::from(Span::styled(
        "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string(),
        style,
    ))
}

fn fmt_val(v: f64) -> String {
    if v.is_nan() {
        "\u{2014}".to_string() // em-dash
    } else if v.abs() >= 1e6 || (v != 0.0 && v.abs() < 1e-3) {
        format!("{:.4e}", v)
    } else {
        format!("{:.4}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_stats() {
        let data: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let s = Stats::compute(&data);
        assert_eq!(s.count, 100);
        assert_eq!(s.valid, 100);
        assert_eq!(s.nan_count, 0);
        assert!((s.min - 1.0).abs() < 1e-9);
        assert!((s.max - 100.0).abs() < 1e-9);
        assert!((s.mean - 50.5).abs() < 1e-9);
        assert!((s.median - 50.5).abs() < 1e-9);
    }

    #[test]
    fn test_stats_with_nan() {
        let data = vec![1.0, f64::NAN, 3.0, f64::NAN, 5.0];
        let s = Stats::compute(&data);
        assert_eq!(s.count, 5);
        assert_eq!(s.valid, 3);
        assert_eq!(s.nan_count, 2);
        assert!((s.valid_fraction - 0.6).abs() < 1e-9);
        assert!((s.min - 1.0).abs() < 1e-9);
        assert!((s.max - 5.0).abs() < 1e-9);
        assert!((s.median - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_stats_with_inf() {
        let data = vec![1.0, f64::INFINITY, 3.0, f64::NEG_INFINITY];
        let s = Stats::compute(&data);
        assert_eq!(s.inf_count, 2);
        assert_eq!(s.valid, 2);
        assert!((s.mean - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_stats_empty() {
        let s = Stats::compute(&[]);
        assert_eq!(s.count, 0);
        assert_eq!(s.valid, 0);
        assert!(s.mean.is_nan());
        assert!(s.median.is_nan());
    }

    #[test]
    fn test_stats_all_nan() {
        let data = vec![f64::NAN; 10];
        let s = Stats::compute(&data);
        assert_eq!(s.count, 10);
        assert_eq!(s.nan_count, 10);
        assert_eq!(s.valid, 0);
        assert!(s.min.is_nan());
    }

    #[test]
    fn test_percentiles() {
        let data: Vec<f64> = (0..=100).map(|i| i as f64).collect();
        let s = Stats::compute(&data);
        assert!((s.p05 - 5.0).abs() < 1e-9);
        assert!((s.p25 - 25.0).abs() < 1e-9);
        assert!((s.p75 - 75.0).abs() < 1e-9);
        assert!((s.p95 - 95.0).abs() < 1e-9);
    }

    #[test]
    fn test_single_value() {
        let s = Stats::compute(&[42.0]);
        assert_eq!(s.valid, 1);
        assert!((s.min - 42.0).abs() < 1e-9);
        assert!((s.max - 42.0).abs() < 1e-9);
        assert!((s.median - 42.0).abs() < 1e-9);
        assert!((s.std_dev - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_std_dev() {
        // Known: std dev of [2, 4, 4, 4, 5, 5, 7, 9] = ~2.138
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let s = Stats::compute(&data);
        assert!((s.std_dev - 2.1380899).abs() < 0.001);
    }

    #[test]
    fn test_fmt_val() {
        assert_eq!(fmt_val(42.0), "42.0000");
        assert_eq!(fmt_val(f64::NAN), "\u{2014}");
        assert!(fmt_val(1e8).contains('e'));
        assert!(fmt_val(1e-5).contains('e'));
    }
}
