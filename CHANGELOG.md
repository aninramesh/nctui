# Changelog

## v0.8.0

### New features

- **Interactive TUI application** — `nctui <file.nc>` now launches a full
  interactive terminal UI instead of printing a text summary. The application
  composes all existing widgets (tree navigator, heatmap, stats panel,
  histogram, table preview, search, dimension slicer) into a coherent
  keyboard-driven interface.

- **Application shell** — new `App` state struct (`src/app.rs`) that owns
  every widget and manages focus, modal state, and data loading. Selecting a
  variable in the tree automatically reads its data from the NetCDF file,
  populates the heatmap, stats panel, and histogram, and handles the nD
  slice picker flow for variables with more than 2 dimensions.

- **Layout engine** — new `ui` module (`src/ui.rs`) providing a composable
  panel layout: search bar + tree on the left, heatmap + stats on the right,
  status bar at the bottom, and modal overlays (histogram, table, slicer,
  help) drawn on top.

- **Keyboard navigation** — vim-style keybindings throughout: `j`/`k` for
  tree navigation, `/` for fuzzy search, `Enter` to load variables, `h` for
  histogram, `t` for table preview, `s` for the dimension slicer, `?` for
  a help overlay, and `q` to quit. Each modal has its own keybinding layer.

- **Help overlay** — press `?` to see all keybindings in a centered modal.

### Architecture

- `src/app.rs` — central `App` struct composing `TreeNavigator`,
  `HeatmapPanel`, `StatsPanel`, `HistogramState`, `TablePreview`,
  `SearchState`, and `SlicePicker`, with `Focus` and `Modal` enums for
  input routing.
- `src/ui.rs` — `draw()` function that renders the full layout into a
  ratatui `Buffer`, with focus-aware borders and a file/variable status bar.
- `src/main.rs` — replaced the summary-printing CLI with a crossterm
  alternate-screen event loop that calls `ui::draw` and routes keyboard
  events through `handle_key`.
- `src/lib.rs` — exports `app` and `ui` modules (gated behind
  `netcdf-backend` since they depend on the backend).

### Tests

- 5 new unit tests for `App` creation, focus toggling, modal state, search
  catalog population, and tree expansion.
- 1 new unit test for help modal rendering.
- 4 new snapshot tests for the full-app UI layout: initial state, search
  active, help modal, and loaded variable with heatmap + stats.

### Other

- Version bumped to 0.8.0.
- Updated README with interactive usage instructions and full keybinding
  reference tables.

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
