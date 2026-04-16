# nctui

Terminal UI viewer for NetCDF4 / HDF5 datasets, built with Rust and [ratatui](https://github.com/ratatui/ratatui).

## Features

- **Tree navigator** -- browse groups and variables with expand/collapse; coordinate variables are auto-detected and marked
- **2D heatmap** -- color-mapped visualization using Unicode block characters (░▒▓█) with a blue-to-red palette and automatic downsampling
- **Histogram overlay** -- equal-width bin distribution plot with adjustable bin count (4--80)
- **Dimension slicer** -- interactively pick a 2D slice from an nD variable by assigning X/Y axes and stepping through fixed dimensions

All rendering is terminal-native -- no GPU, no graphics protocol, just Unicode and ANSI colors.

## Requirements

- Rust 1.70+ (edition 2021)

## Build

```
cargo build --release
```

The binary is written to `target/release/nctui`.

## Usage

```
nctui <file.nc>
```

## Tests

The test suite includes per-module unit tests and 16 snapshot tests (via [insta](https://insta.rs/)) that render each widget into a buffer and compare against golden files.

```
cargo test                   # run all tests
cargo insta test --review    # review snapshot changes after code edits
```

## License

See repository for license details.
