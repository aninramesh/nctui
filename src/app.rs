//! Application state for the interactive TUI.
//!
//! `App` composes every widget (tree, heatmap, stats, histogram, table,
//! search, slice picker) into a single struct and routes keyboard events
//! to the currently focused component.

use indexmap::IndexMap;

use crate::heatmap::HeatmapPanel;
use crate::histogram::HistogramState;
use crate::search::{SearchState, VarInfo};
use crate::slice_picker::{DimRole, SlicePicker, SliceSpec};
use crate::stats::StatsPanel;
use crate::table_preview::TablePreview;
use crate::tree::{RowKind, TreeNavigator};

/// Which panel has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tree,
    Search,
}

/// Which modal overlay is active (at most one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modal {
    None,
    Histogram,
    Table,
    SlicePicker,
    Help,
}

/// Central application state.
pub struct App {
    // -- widgets --
    pub tree: TreeNavigator,
    pub heatmap: Option<HeatmapPanel>,
    pub stats: StatsPanel,
    pub histogram: HistogramState,
    pub table: Option<TablePreview>,
    pub search: SearchState,
    pub slice_picker: Option<SlicePicker>,

    // -- interaction state --
    pub focus: Focus,
    pub modal: Modal,
    pub running: bool,
    pub status_msg: String,

    // -- dataset metadata --
    pub file_path: String,
    pub var_meta: IndexMap<String, crate::backend::VarMeta>,
    pub coord_vars: IndexMap<String, String>,

    // -- currently loaded variable data --
    pub current_var: Option<String>,
    pub current_data: Vec<f64>,
}

impl App {
    /// Create a new App from an opened dataset.
    pub fn new(
        file_path: String,
        info: &crate::backend::DatasetInfo,
    ) -> Self {
        let tree = TreeNavigator::new(info.groups.clone(), info.variables.clone());

        let mut search = SearchState::new();
        let catalog: Vec<VarInfo> = info
            .var_meta
            .iter()
            .map(|(name, meta)| {
                let group = info
                    .groups
                    .iter()
                    .find(|(_, vars)| vars.contains(name))
                    .map(|(g, _)| g.clone())
                    .unwrap_or_else(|| "/".to_string());
                VarInfo {
                    name: name.clone(),
                    group,
                    dim_names: meta.dim_names.clone(),
                    dim_sizes: meta.dim_sizes.clone(),
                    is_coord: info.coord_vars.contains_key(name),
                }
            })
            .collect();
        search.set_catalog(catalog);

        Self {
            tree,
            heatmap: None,
            stats: StatsPanel::new(),
            histogram: HistogramState::new(),
            table: None,
            search,
            slice_picker: None,
            focus: Focus::Tree,
            modal: Modal::None,
            running: true,
            status_msg: String::new(),
            file_path,
            var_meta: info.var_meta.clone(),
            coord_vars: info.coord_vars.clone(),
            current_var: None,
            current_data: Vec::new(),
        }
    }

    /// Load data for the currently selected variable in the tree.
    ///
    /// For 2D variables, loads into heatmap + stats. For 1D, loads stats
    /// only. For nD (>2) variables, opens the slice picker.
    pub fn load_selected_variable(
        &mut self,
        file: &netcdf::File,
        info: &crate::backend::DatasetInfo,
    ) {
        let row = match self.tree.rows.get(self.tree.selected) {
            Some(r) => r.clone(),
            None => return,
        };

        // Only load variables, not groups
        if matches!(row.kind, RowKind::Group { .. }) {
            self.tree.toggle_expand();
            return;
        }

        let var_name = &row.label;
        let var_path = &row.path;

        let meta = match self.var_meta.get(var_name) {
            Some(m) => m.clone(),
            None => {
                self.status_msg = format!("No metadata for {var_name}");
                return;
            }
        };

        let ndim = meta.dim_names.len();

        if ndim > 2 {
            // Open slice picker for nD variables
            let spec = SliceSpec::default_for(var_name, &meta.dim_names, &meta.dim_sizes);
            self.slice_picker = Some(SlicePicker::new(spec));
            self.modal = Modal::SlicePicker;
            self.status_msg = format!("{var_name}: pick a 2D slice");
            return;
        }

        // Read the full variable
        let data = match crate::backend::read_var_f64(file, var_path) {
            Ok(d) => d,
            Err(e) => {
                self.status_msg = format!("Error reading {var_name}: {e}");
                return;
            }
        };

        self.current_var = Some(var_name.clone());
        self.current_data = data.clone();

        // Stats
        self.stats.set_data(var_name, &data);

        // Histogram
        self.histogram.set_data(&data);

        if ndim == 0 {
            // Scalar
            self.heatmap = None;
            self.table = None;
            self.status_msg = format!("{var_name}: scalar value");
        } else if ndim == 1 {
            // 1D variable: show as single-row heatmap + prepare table
            let dim = &meta.dim_names[0];
            let coords = crate::backend::read_coord_var(file, dim, info);
            let hm_data = vec![data.clone()];
            self.heatmap = Some(HeatmapPanel::with_coords(
                hm_data,
                var_name,
                None,
                coords.clone(),
            ));
            self.table = Some(TablePreview::from_1d(
                &data,
                var_name,
                dim,
                coords.as_deref(),
            ));
            self.table.as_mut().unwrap().visible = false;
            self.status_msg = format!(
                "{var_name} [{dim}={}]",
                meta.dim_sizes[0]
            );
        } else {
            // 2D variable
            let nrows = meta.dim_sizes[0];
            let ncols = meta.dim_sizes[1];
            let data_2d: Vec<Vec<f64>> = data.chunks(ncols).map(|c| c.to_vec()).collect();

            let row_dim = &meta.dim_names[0];
            let col_dim = &meta.dim_names[1];
            let row_coords = crate::backend::read_coord_var(file, row_dim, info);
            let col_coords = crate::backend::read_coord_var(file, col_dim, info);

            self.heatmap = Some(HeatmapPanel::with_coords(
                data_2d.clone(),
                var_name,
                row_coords.clone(),
                col_coords.clone(),
            ));
            self.table = Some(TablePreview::from_2d(
                &data_2d,
                var_name,
                row_dim,
                col_dim,
                row_coords.as_deref(),
                col_coords.as_deref(),
            ));
            self.table.as_mut().unwrap().visible = false;
            self.status_msg = format!(
                "{var_name} [{row_dim}={nrows}, {col_dim}={ncols}]",
            );
        }
    }

    /// Apply the confirmed slice spec to load a 2D slice from an nD variable.
    pub fn apply_slice(
        &mut self,
        file: &netcdf::File,
        info: &crate::backend::DatasetInfo,
    ) {
        let picker = match &self.slice_picker {
            Some(p) => p,
            None => return,
        };
        let spec = &picker.spec;
        let var_name = &spec.var_name;

        let meta = match self.var_meta.get(var_name) {
            Some(m) => m.clone(),
            None => return,
        };

        // Build the var path
        let var_path = info
            .groups
            .iter()
            .find(|(_, vars)| vars.contains(var_name))
            .map(|(g, _)| {
                if g == "/" {
                    format!("/{var_name}")
                } else {
                    format!("/{g}/{var_name}")
                }
            })
            .unwrap_or_else(|| format!("/{var_name}"));

        // Read full data
        let all_data = match crate::backend::read_var_f64(file, &var_path) {
            Ok(d) => d,
            Err(e) => {
                self.status_msg = format!("Error reading {var_name}: {e}");
                return;
            }
        };

        // Extract the 2D slice
        let (y_idx, x_idx) = match spec.xy_axes() {
            Some(pair) => pair,
            None => {
                self.status_msg = "Need exactly 2 free axes".to_string();
                return;
            }
        };

        let y_size = meta.dim_sizes[y_idx];
        let x_size = meta.dim_sizes[x_idx];

        // Build strides for indexing into the flat array
        let mut strides = vec![1usize; meta.dim_sizes.len()];
        for i in (0..meta.dim_sizes.len() - 1).rev() {
            strides[i] = strides[i + 1] * meta.dim_sizes[i + 1];
        }

        // Compute base offset from fixed dimensions
        let mut base_offset = 0usize;
        for (i, dim) in spec.dims.iter().enumerate() {
            if let DimRole::Fixed(idx) = dim.role {
                base_offset += idx * strides[i];
            }
        }

        // Extract 2D slice
        let mut data_2d = Vec::with_capacity(y_size);
        for yr in 0..y_size {
            let mut row = Vec::with_capacity(x_size);
            for xc in 0..x_size {
                let offset = base_offset + yr * strides[y_idx] + xc * strides[x_idx];
                let val = all_data.get(offset).copied().unwrap_or(f64::NAN);
                row.push(val);
            }
            data_2d.push(row);
        }

        let flat_data: Vec<f64> = data_2d.iter().flat_map(|r| r.iter().copied()).collect();

        let row_dim = &meta.dim_names[y_idx];
        let col_dim = &meta.dim_names[x_idx];
        let row_coords = crate::backend::read_coord_var(file, row_dim, info);
        let col_coords = crate::backend::read_coord_var(file, col_dim, info);

        self.current_var = Some(var_name.clone());
        self.current_data = flat_data.clone();
        self.stats.set_data(var_name, &flat_data);
        self.histogram.set_data(&flat_data);
        self.heatmap = Some(HeatmapPanel::with_coords(
            data_2d.clone(),
            var_name,
            row_coords.clone(),
            col_coords.clone(),
        ));
        self.table = Some(TablePreview::from_2d(
            &data_2d,
            var_name,
            row_dim,
            col_dim,
            row_coords.as_deref(),
            col_coords.as_deref(),
        ));
        self.table.as_mut().unwrap().visible = false;

        // Build a description of fixed dims
        let fixed_desc: Vec<String> = spec
            .dims
            .iter()
            .filter_map(|d| {
                if let DimRole::Fixed(idx) = d.role {
                    Some(format!("{}={}", d.name, idx))
                } else {
                    None
                }
            })
            .collect();
        let fixed_str = if fixed_desc.is_empty() {
            String::new()
        } else {
            format!(" ({})", fixed_desc.join(", "))
        };

        self.status_msg = format!(
            "{var_name} [{row_dim}={y_size}, {col_dim}={x_size}]{fixed_str}",
        );

        self.modal = Modal::None;
        self.slice_picker = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_info() -> crate::backend::DatasetInfo {
        let mut groups = IndexMap::new();
        groups.insert(
            "/".to_string(),
            vec!["temperature".to_string(), "lat".to_string()],
        );

        let mut variables = IndexMap::new();
        variables.insert(
            "temperature".to_string(),
            vec!["lat".to_string(), "lon".to_string()],
        );
        variables.insert("lat".to_string(), vec!["lat".to_string()]);

        let mut var_meta = IndexMap::new();
        var_meta.insert(
            "temperature".to_string(),
            crate::backend::VarMeta {
                name: "temperature".to_string(),
                dim_names: vec!["lat".to_string(), "lon".to_string()],
                dim_sizes: vec![4, 6],
            },
        );
        var_meta.insert(
            "lat".to_string(),
            crate::backend::VarMeta {
                name: "lat".to_string(),
                dim_names: vec!["lat".to_string()],
                dim_sizes: vec![4],
            },
        );

        let mut coord_vars = IndexMap::new();
        coord_vars.insert("lat".to_string(), "lat".to_string());

        crate::backend::DatasetInfo {
            groups,
            variables,
            var_meta,
            coord_vars,
        }
    }

    #[test]
    fn test_app_creation() {
        let info = stub_info();
        let app = App::new("test.nc".to_string(), &info);
        assert!(app.running);
        assert_eq!(app.focus, Focus::Tree);
        assert_eq!(app.modal, Modal::None);
        assert!(app.heatmap.is_none());
        assert!(app.current_var.is_none());
        assert_eq!(app.tree.rows.len(), 1); // one group collapsed
    }

    #[test]
    fn test_search_catalog_populated() {
        let info = stub_info();
        let app = App::new("test.nc".to_string(), &info);
        assert_eq!(app.search.catalog().len(), 2);
    }

    #[test]
    fn test_focus_toggle() {
        let info = stub_info();
        let mut app = App::new("test.nc".to_string(), &info);
        assert_eq!(app.focus, Focus::Tree);
        app.focus = Focus::Search;
        app.search.active = true;
        assert_eq!(app.focus, Focus::Search);
        assert!(app.search.active);
    }

    #[test]
    fn test_modal_state() {
        let info = stub_info();
        let mut app = App::new("test.nc".to_string(), &info);
        app.modal = Modal::Histogram;
        app.histogram.visible = true;
        assert_eq!(app.modal, Modal::Histogram);
        app.modal = Modal::None;
        app.histogram.visible = false;
        assert_eq!(app.modal, Modal::None);
    }

    #[test]
    fn test_tree_expand_on_group_select() {
        let info = stub_info();
        let mut app = App::new("test.nc".to_string(), &info);
        // Selected row 0 is the "/" group
        assert_eq!(app.tree.rows.len(), 1);
        app.tree.toggle_expand();
        assert!(app.tree.rows.len() > 1);
    }
}
