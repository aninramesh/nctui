//! NetCDF/HDF5 file reading backend.
//!
//! Translates the on-disk structure into the abstract data types consumed
//! by the TUI widgets (tree of groups/variables, dimension metadata, and
//! flat f64 data slices).

use indexmap::IndexMap;
use std::path::Path;

/// Metadata about a single variable as needed by the TUI.
#[derive(Debug, Clone)]
pub struct VarMeta {
    pub name: String,
    pub dim_names: Vec<String>,
    pub dim_sizes: Vec<usize>,
}

/// Everything the TUI needs from a NetCDF / HDF5 file.
#[derive(Debug)]
pub struct DatasetInfo {
    /// Group name → ordered list of variable names contained in it.
    pub groups: IndexMap<String, Vec<String>>,
    /// Variable name → ordered list of dimension names.
    pub variables: IndexMap<String, Vec<String>>,
    /// Variable name → full metadata (dims + sizes).
    pub var_meta: IndexMap<String, VarMeta>,
}

/// Open a NetCDF / HDF5 file and collect its structure.
pub fn open_dataset(path: &Path) -> Result<(netcdf::File, DatasetInfo), String> {
    let file = netcdf::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;

    let mut groups: IndexMap<String, Vec<String>> = IndexMap::new();
    let mut variables: IndexMap<String, Vec<String>> = IndexMap::new();
    let mut var_meta: IndexMap<String, VarMeta> = IndexMap::new();

    // Collect root-level variables into a synthetic "/" group.
    let root_vars: Vec<String> = file.variables().map(|v| v.name()).collect();
    if !root_vars.is_empty() {
        groups.insert("/".to_string(), root_vars);
        for v in file.variables() {
            let name = v.name();
            let dim_names: Vec<String> = v.dimensions().iter().map(|d| d.name()).collect();
            let dim_sizes: Vec<usize> = v.dimensions().iter().map(|d| d.len()).collect();
            variables.insert(name.clone(), dim_names.clone());
            var_meta.insert(
                name.clone(),
                VarMeta {
                    name,
                    dim_names,
                    dim_sizes,
                },
            );
        }
    }

    // Collect named groups (File::groups returns Result<Iterator>).
    if let Ok(grp_iter) = file.groups() {
        for grp in grp_iter {
            let gname = grp.name();
            let gvars: Vec<String> = grp.variables().map(|v| v.name()).collect();
            if !gvars.is_empty() {
                groups.insert(gname.clone(), gvars);
            }
            for v in grp.variables() {
                let name = v.name();
                let dim_names: Vec<String> =
                    v.dimensions().iter().map(|d| d.name()).collect();
                let dim_sizes: Vec<usize> = v.dimensions().iter().map(|d| d.len()).collect();
                variables.insert(name.clone(), dim_names.clone());
                var_meta.insert(
                    name.clone(),
                    VarMeta {
                        name,
                        dim_names,
                        dim_sizes,
                    },
                );
            }
        }
    }

    let info = DatasetInfo {
        groups,
        variables,
        var_meta,
    };
    Ok((file, info))
}

/// Read *all* values of a variable as a flat `Vec<f64>`.
///
/// The variable is located either at the root or inside a group based on
/// the path (e.g. "/temperature" for root, "/ocean/salinity" for a group).
pub fn read_var_f64(file: &netcdf::File, var_path: &str) -> Result<Vec<f64>, String> {
    let parts: Vec<&str> = var_path
        .trim_start_matches('/')
        .splitn(2, '/')
        .collect();

    if parts.len() == 2 {
        // group/var — File::group returns Result<Option<Group>>
        let grp = file
            .group(parts[0])
            .map_err(|e| format!("group lookup error: {e}"))?
            .ok_or_else(|| format!("group '{}' not found", parts[0]))?;
        let var = grp
            .variable(parts[1])
            .ok_or_else(|| format!("variable '{}' not found in group '{}'", parts[1], parts[0]))?;
        read_variable_data(&var)
    } else {
        // root-level var
        let vname = parts[0];
        let var = file
            .variable(vname)
            .ok_or_else(|| format!("variable '{vname}' not found at root"))?;
        read_variable_data(&var)
    }
}

fn read_variable_data(var: &netcdf::Variable) -> Result<Vec<f64>, String> {
    let total: usize = var.dimensions().iter().map(|d| d.len()).product();
    if total == 0 {
        return Ok(Vec::new());
    }
    let mut data = vec![0.0f64; total];
    var.get_values_into(&mut data, ..)
        .map_err(|e| format!("read error: {e}"))?;
    Ok(data)
}
