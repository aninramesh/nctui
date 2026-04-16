#[cfg(not(feature = "netcdf-backend"))]
fn main() {
    eprintln!(
        "nctui v{} — built without NetCDF backend.\n\
         Rebuild with: cargo build --features netcdf-backend",
        env!("CARGO_PKG_VERSION")
    );
    std::process::exit(1);
}

#[cfg(feature = "netcdf-backend")]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 || args[1] == "-h" || args[1] == "--help" {
        eprintln!("nctui v{} — Terminal UI viewer for NetCDF4 / HDF5 datasets", env!("CARGO_PKG_VERSION"));
        eprintln!("Usage: nctui <file.nc>");
        std::process::exit(if args.len() == 1 { 1 } else { 0 });
    }

    let path = std::path::Path::new(&args[1]);
    let (file, info) = match nctui::backend::open_dataset(path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    // Summarize the dataset to stdout (interactive TUI loop to be added).
    println!("nctui v{}", env!("CARGO_PKG_VERSION"));
    println!("File: {}", path.display());
    println!("Groups: {}", info.groups.len());
    if !info.coord_vars.is_empty() {
        println!(
            "Coordinate variables: {}",
            info.coord_vars.keys().cloned().collect::<Vec<_>>().join(", ")
        );
    }
    for (gname, vars) in &info.groups {
        println!("  {gname}/ ({} variables)", vars.len());
        for vname in vars {
            if let Some(meta) = info.var_meta.get(vname) {
                let dims: Vec<String> = meta
                    .dim_names
                    .iter()
                    .zip(&meta.dim_sizes)
                    .map(|(n, s)| format!("{n}={s}"))
                    .collect();
                let coord_marker = if info.coord_vars.contains_key(vname) {
                    " (coord)"
                } else {
                    ""
                };
                println!("    {vname}  [{}]{coord_marker}", dims.join(", "));
            }
        }
    }
}
