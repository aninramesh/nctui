pub mod heatmap;
pub mod histogram;
pub mod search;
pub mod slice_picker;
pub mod stats;
pub mod table_preview;
pub mod tree;

#[cfg(feature = "netcdf-backend")]
pub mod app;
#[cfg(feature = "netcdf-backend")]
pub mod backend;
#[cfg(feature = "netcdf-backend")]
pub mod ui;
