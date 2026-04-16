//! Snapshot tests for nctui TUI rendering.
//!
//! These tests render widgets into a fixed-size ratatui Buffer, convert the
//! buffer to a plain-text string (with trailing whitespace trimmed per line),
//! and compare against golden snapshots managed by the `insta` crate.
//!
//! The buffer-to-string conversion strips trailing spaces so that snapshots
//! are stable across minor terminal-size or padding changes.

use indexmap::IndexMap;
use ratatui::{buffer::Buffer, layout::Rect};

use nctui::heatmap::HeatmapPanel;
use nctui::histogram::{Histogram, HistogramState};
use nctui::search::{SearchState, VarInfo};
use nctui::slice_picker::{SlicePicker, SliceSpec};
use nctui::stats::StatsPanel;
use nctui::table_preview::TablePreview;
use nctui::tree::TreeNavigator;

/// Render a buffer to a trimmed string for snapshot comparison.
/// Each line has trailing whitespace removed to avoid brittle diffs.
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
    // Trim trailing empty lines
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn stub_tree() -> TreeNavigator {
    let mut groups = IndexMap::new();
    groups.insert(
        "atmosphere".to_string(),
        vec![
            "temperature".to_string(),
            "pressure".to_string(),
            "lat".to_string(),
        ],
    );
    groups.insert("ocean".to_string(), vec!["salinity".to_string()]);

    let mut vars = IndexMap::new();
    vars.insert(
        "temperature".to_string(),
        vec!["lat".to_string(), "lon".to_string(), "time".to_string()],
    );
    vars.insert(
        "pressure".to_string(),
        vec!["lat".to_string(), "lon".to_string()],
    );
    vars.insert("lat".to_string(), vec!["lat".to_string()]);
    vars.insert("salinity".to_string(), vec!["depth".to_string()]);

    TreeNavigator::new(groups, vars)
}

fn make_heatmap_data(rows: usize, cols: usize) -> Vec<Vec<f64>> {
    (0..rows)
        .map(|r| {
            (0..cols)
                .map(|c| (r * cols + c) as f64)
                .collect()
        })
        .collect()
}

fn sample_catalog() -> Vec<VarInfo> {
    vec![
        VarInfo {
            name: "temperature".into(),
            group: "atmosphere".into(),
            dim_names: vec!["lat".into(), "lon".into(), "time".into()],
            dim_sizes: vec![180, 360, 12],
            is_coord: false,
        },
        VarInfo {
            name: "pressure".into(),
            group: "atmosphere".into(),
            dim_names: vec!["lat".into(), "lon".into()],
            dim_sizes: vec![180, 360],
            is_coord: false,
        },
        VarInfo {
            name: "lat".into(),
            group: "atmosphere".into(),
            dim_names: vec!["lat".into()],
            dim_sizes: vec![180],
            is_coord: true,
        },
        VarInfo {
            name: "salinity".into(),
            group: "ocean".into(),
            dim_names: vec!["depth".into()],
            dim_sizes: vec![50],
            is_coord: false,
        },
    ]
}

// ---------------------------------------------------------------------------
// Tree Navigation Snapshots
// ---------------------------------------------------------------------------

#[test]
fn snapshot_tree_collapsed() {
    let tree = stub_tree();
    let area = Rect::new(0, 0, 40, 10);
    let mut buf = Buffer::empty(area);
    tree.render(area, &mut buf);
    insta::assert_snapshot!("tree_collapsed", buffer_to_string(&buf));
}

#[test]
fn snapshot_tree_expanded_atmosphere() {
    let mut tree = stub_tree();
    tree.toggle_expand(); // expand atmosphere (selected=0)
    let area = Rect::new(0, 0, 40, 12);
    let mut buf = Buffer::empty(area);
    tree.render(area, &mut buf);
    insta::assert_snapshot!("tree_expanded_atmosphere", buffer_to_string(&buf));
}

#[test]
fn snapshot_tree_selection_on_variable() {
    let mut tree = stub_tree();
    tree.toggle_expand(); // expand atmosphere
    tree.move_down(); // select temperature
    let area = Rect::new(0, 0, 40, 12);
    let mut buf = Buffer::empty(area);
    tree.render(area, &mut buf);
    insta::assert_snapshot!("tree_selection_on_variable", buffer_to_string(&buf));
}

#[test]
fn snapshot_tree_both_groups_expanded() {
    let mut tree = stub_tree();
    tree.toggle_expand(); // expand atmosphere
    tree.jump_bottom(); // go to ocean
    tree.toggle_expand(); // expand ocean
    let area = Rect::new(0, 0, 40, 14);
    let mut buf = Buffer::empty(area);
    tree.render(area, &mut buf);
    insta::assert_snapshot!("tree_both_expanded", buffer_to_string(&buf));
}

// ---------------------------------------------------------------------------
// Slice Modal Snapshots
// ---------------------------------------------------------------------------

#[test]
fn snapshot_slice_picker_3d() {
    let spec = SliceSpec::default_for(
        "temperature",
        &["time".into(), "lat".into(), "lon".into()],
        &[12, 180, 360],
    );
    let picker = SlicePicker::new(spec);
    let area = Rect::new(0, 0, 60, 20);
    let mut buf = Buffer::empty(area);
    picker.render(area, &mut buf);
    insta::assert_snapshot!("slice_picker_3d", buffer_to_string(&buf));
}

#[test]
fn snapshot_slice_picker_4d() {
    let spec = SliceSpec::default_for(
        "temperature",
        &[
            "time".into(),
            "level".into(),
            "lat".into(),
            "lon".into(),
        ],
        &[12, 10, 180, 360],
    );
    let picker = SlicePicker::new(spec);
    let area = Rect::new(0, 0, 60, 22);
    let mut buf = Buffer::empty(area);
    picker.render(area, &mut buf);
    insta::assert_snapshot!("slice_picker_4d", buffer_to_string(&buf));
}

#[test]
fn snapshot_slice_picker_selection_moved() {
    let spec = SliceSpec::default_for(
        "temperature",
        &["time".into(), "lat".into(), "lon".into()],
        &[12, 180, 360],
    );
    let mut picker = SlicePicker::new(spec);
    picker.selected = 2; // select lon row
    let area = Rect::new(0, 0, 60, 20);
    let mut buf = Buffer::empty(area);
    picker.render(area, &mut buf);
    insta::assert_snapshot!("slice_picker_selection_moved", buffer_to_string(&buf));
}

// ---------------------------------------------------------------------------
// Heatmap Snapshots
// ---------------------------------------------------------------------------

#[test]
fn snapshot_heatmap_small() {
    let data = make_heatmap_data(4, 6);
    let panel = HeatmapPanel::new(data, "SST");
    let area = Rect::new(0, 0, 30, 10);
    let mut buf = Buffer::empty(area);
    panel.render(area, &mut buf);
    insta::assert_snapshot!("heatmap_small", buffer_to_string(&buf));
}

#[test]
fn snapshot_heatmap_with_nan() {
    let mut data = make_heatmap_data(4, 6);
    data[1][2] = f64::NAN;
    data[2][4] = f64::NAN;
    let panel = HeatmapPanel::new(data, "SST (with NaN)");
    let area = Rect::new(0, 0, 30, 10);
    let mut buf = Buffer::empty(area);
    panel.render(area, &mut buf);
    insta::assert_snapshot!("heatmap_with_nan", buffer_to_string(&buf));
}

#[test]
fn snapshot_heatmap_single_row() {
    let data = vec![vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]];
    let panel = HeatmapPanel::new(data, "1D profile");
    let area = Rect::new(0, 0, 30, 8);
    let mut buf = Buffer::empty(area);
    panel.render(area, &mut buf);
    insta::assert_snapshot!("heatmap_single_row", buffer_to_string(&buf));
}

#[test]
fn snapshot_heatmap_uniform_data() {
    let data = vec![vec![42.0; 6]; 4];
    let panel = HeatmapPanel::new(data, "Uniform");
    let area = Rect::new(0, 0, 30, 10);
    let mut buf = Buffer::empty(area);
    panel.render(area, &mut buf);
    insta::assert_snapshot!("heatmap_uniform", buffer_to_string(&buf));
}

#[test]
fn snapshot_heatmap_with_coords() {
    let data = make_heatmap_data(4, 6);
    let row_coords = vec![-90.0, -30.0, 30.0, 90.0];
    let col_coords = vec![0.0, 60.0, 120.0, 180.0, 240.0, 300.0];
    let panel = HeatmapPanel::with_coords(
        data,
        "SST (coords)",
        Some(row_coords),
        Some(col_coords),
    );
    let area = Rect::new(0, 0, 30, 10);
    let mut buf = Buffer::empty(area);
    panel.render(area, &mut buf);
    insta::assert_snapshot!("heatmap_with_coords", buffer_to_string(&buf));
}

// ---------------------------------------------------------------------------
// Histogram Snapshots
// ---------------------------------------------------------------------------

#[test]
fn snapshot_histogram_normal() {
    let data: Vec<f64> = (0..200).map(|i| (i as f64 / 10.0).sin() * 100.0).collect();
    let mut state = HistogramState::new();
    state.visible = true;
    state.n_bins = 12;
    state.histogram = Some(Histogram::compute(&data, 12));
    let area = Rect::new(0, 0, 60, 20);
    let mut buf = Buffer::empty(area);
    state.render(area, &mut buf);
    insta::assert_snapshot!("histogram_normal", buffer_to_string(&buf));
}

#[test]
fn snapshot_histogram_few_bins() {
    let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
    let mut state = HistogramState::new();
    state.visible = true;
    state.n_bins = 4;
    state.histogram = Some(Histogram::compute(&data, 4));
    let area = Rect::new(0, 0, 50, 16);
    let mut buf = Buffer::empty(area);
    state.render(area, &mut buf);
    insta::assert_snapshot!("histogram_few_bins", buffer_to_string(&buf));
}

#[test]
fn snapshot_histogram_degenerate() {
    let data = vec![7.0; 80];
    let mut state = HistogramState::new();
    state.visible = true;
    state.n_bins = 10;
    state.histogram = Some(Histogram::compute(&data, 10));
    let area = Rect::new(0, 0, 50, 16);
    let mut buf = Buffer::empty(area);
    state.render(area, &mut buf);
    insta::assert_snapshot!("histogram_degenerate", buffer_to_string(&buf));
}

#[test]
fn snapshot_histogram_hidden() {
    let state = HistogramState::new(); // visible=false
    let area = Rect::new(0, 0, 50, 16);
    let mut buf = Buffer::empty(area);
    state.render(area, &mut buf);
    // Should be empty — all spaces trimmed
    let output = buffer_to_string(&buf);
    insta::assert_snapshot!("histogram_hidden", output);
}

// ---------------------------------------------------------------------------
// Stats Panel Snapshots
// ---------------------------------------------------------------------------

#[test]
fn snapshot_stats_panel_basic() {
    let mut panel = StatsPanel::new();
    let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
    panel.set_data("temperature", &data);
    let area = Rect::new(0, 0, 30, 20);
    let mut buf = Buffer::empty(area);
    panel.render(area, &mut buf);
    insta::assert_snapshot!("stats_panel_basic", buffer_to_string(&buf));
}

#[test]
fn snapshot_stats_panel_with_nan() {
    let mut panel = StatsPanel::new();
    let mut data: Vec<f64> = (0..50).map(|i| i as f64).collect();
    data.extend(vec![f64::NAN; 10]);
    panel.set_data("pressure", &data);
    let area = Rect::new(0, 0, 30, 20);
    let mut buf = Buffer::empty(area);
    panel.render(area, &mut buf);
    insta::assert_snapshot!("stats_panel_with_nan", buffer_to_string(&buf));
}

#[test]
fn snapshot_stats_panel_empty() {
    let panel = StatsPanel::new();
    let area = Rect::new(0, 0, 30, 6);
    let mut buf = Buffer::empty(area);
    panel.render(area, &mut buf);
    insta::assert_snapshot!("stats_panel_empty", buffer_to_string(&buf));
}

// ---------------------------------------------------------------------------
// Table Preview Snapshots
// ---------------------------------------------------------------------------

#[test]
fn snapshot_table_1d() {
    let data = vec![10.0, 20.5, 30.0, f64::NAN, 50.0];
    let tp = TablePreview::from_1d(&data, "lat", "lat", None);
    let area = Rect::new(0, 0, 40, 14);
    let mut buf = Buffer::empty(area);
    tp.render(area, &mut buf);
    insta::assert_snapshot!("table_preview_1d", buffer_to_string(&buf));
}

#[test]
fn snapshot_table_2d() {
    let data = vec![
        vec![1.0, 2.0, 3.0],
        vec![4.0, 5.0, 6.0],
    ];
    let tp = TablePreview::from_2d(&data, "sst", "lat", "lon", None, None);
    let area = Rect::new(0, 0, 60, 14);
    let mut buf = Buffer::empty(area);
    tp.render(area, &mut buf);
    insta::assert_snapshot!("table_preview_2d", buffer_to_string(&buf));
}

#[test]
fn snapshot_table_with_coords() {
    let data = vec![100.0, 200.0, 300.0];
    let coords = vec![-90.0, 0.0, 90.0];
    let tp = TablePreview::from_1d(&data, "temperature", "lat", Some(&coords));
    let area = Rect::new(0, 0, 40, 14);
    let mut buf = Buffer::empty(area);
    tp.render(area, &mut buf);
    insta::assert_snapshot!("table_preview_with_coords", buffer_to_string(&buf));
}

// ---------------------------------------------------------------------------
// Search Bar Snapshots
// ---------------------------------------------------------------------------

#[test]
fn snapshot_search_bar_inactive() {
    let ss = SearchState::new();
    let area = Rect::new(0, 0, 40, 3);
    let mut buf = Buffer::empty(area);
    ss.render_bar(area, &mut buf);
    insta::assert_snapshot!("search_bar_inactive", buffer_to_string(&buf));
}

#[test]
fn snapshot_search_bar_active_with_query() {
    let mut ss = SearchState::new();
    ss.set_catalog(sample_catalog());
    ss.active = true;
    for ch in "temp".chars() {
        ss.push_char(ch);
    }
    let area = Rect::new(0, 0, 40, 3);
    let mut buf = Buffer::empty(area);
    ss.render_bar(area, &mut buf);
    insta::assert_snapshot!("search_bar_active_query", buffer_to_string(&buf));
}

#[test]
fn snapshot_search_results() {
    let mut ss = SearchState::new();
    ss.set_catalog(sample_catalog());
    for ch in "lat".chars() {
        ss.push_char(ch);
    }
    let area = Rect::new(0, 0, 50, 8);
    let mut buf = Buffer::empty(area);
    ss.render_results(area, &mut buf);
    insta::assert_snapshot!("search_results", buffer_to_string(&buf));
}
