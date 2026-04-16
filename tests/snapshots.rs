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
use nctui::slice_picker::{SlicePicker, SliceSpec};
use nctui::tree::{TreeNavigator};

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
