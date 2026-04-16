# Changelog

## v0.7.0

### New features

- **Coordinate-aware axes** — heatmap plots display real coordinate values
  (e.g. latitude, longitude, depth) when matching coordinate variables are
  found in the dataset, instead of showing only integer indices.
  `HeatmapPanel::with_coords()` accepts optional `Vec<f64>` coordinate
  arrays for both axes.

- **Stats panel** — new `StatsPanel` widget showing comprehensive summary
  statistics for the selected variable: count, valid count, NaN count,
  Inf count, valid-data fraction, min, max, mean, median, standard
  deviation, and percentiles (p5, p25, p75, p95).

- **Table preview** — new `TablePreview` modal overlay for inspecting exact
  numeric values of 1D variables or small 2D slices. Supports scrollable
  rows and columns, coordinate-aware headers, and NaN/Inf formatting.

- **Search & filter** — new `SearchState` widget providing:
  - Fuzzy variable name matching (subsequence matching)
  - Group name and dimension name substring search
  - Dimension filters: filter by dimension name (`HasDim`) or
    dimensionality count (`NDim`)
  - Combined text + dimension filters
  - Inline search bar with result count and filter tags

### Backend changes

- `DatasetInfo` now includes a `coord_vars` field that maps dimension
  names to their matching coordinate variable names (CF convention:
  1D variable whose name matches its sole dimension).
- New `read_coord_var()` function reads coordinate data for a given
  dimension name.
- Main CLI output now shows detected coordinate variables and marks them
  in the variable listing.

### Tests

- 8 new unit tests for `Stats::compute()` covering NaN, Inf, empty data,
  percentiles, single values, and standard deviation.
- 8 new unit tests for `TablePreview` covering 1D/2D construction,
  coordinate headers, scrolling, formatting, and hidden state.
- 13 new unit tests for `SearchState` and fuzzy matching covering
  substring, fuzzy, group, dimension name search, combined filters,
  and scoring.
- 4 new unit tests for coordinate-aware `HeatmapPanel` covering
  `with_coords`, fractional values, mismatched lengths, and `format_coord`.

### Other

- Version bumped to 0.7.0.
- Updated README with new feature descriptions.
- Added this CHANGELOG.

## v0.6.0

Initial release with tree navigator, 2D heatmap, histogram overlay,
dimension slicer, and NetCDF/HDF5 backend.
