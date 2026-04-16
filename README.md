# nctui

Terminal UI viewer for NetCDF4 / HDF5 datasets, built with Rust and [ratatui](https://github.com/ratatui/ratatui).

## Features

- **Interactive TUI** -- full terminal application with keyboard navigation, modal overlays, and a composable panel layout
- **Tree navigator** -- browse groups and variables with expand/collapse; coordinate variables are auto-detected and marked
- **2D heatmap** -- color-mapped visualization using Unicode block characters (░▒▓█) with a blue-to-red palette and automatic downsampling
- **Coordinate-aware axes** -- plots display real coordinate values (e.g. latitude/longitude) when coordinate variables are available, instead of raw indices
- **Stats panel** -- summary statistics including count, min/max, mean, median, standard deviation, percentiles (p5/p25/p75/p95), NaN/Inf counts, and valid-data fraction
- **Table preview** -- inspect exact numeric values for 1D variables or small 2D slices in a scrollable table overlay
- **Histogram overlay** -- equal-width bin distribution plot with adjustable bin count (4--80)
- **Dimension slicer** -- interactively pick a 2D slice from an nD variable by assigning X/Y axes and stepping through fixed dimensions
- **Search & filter** -- fuzzy variable name matching, group/dimension name search, and metadata filters (dimension name, dimensionality)
- **NetCDF/HDF5 backend** -- reads real NetCDF4 and HDF5 files via the [netcdf](https://crates.io/crates/netcdf) crate; the C libraries can be bundled from source for a fully self-contained static binary

All rendering is terminal-native -- no GPU, no graphics protocol, just Unicode and ANSI colors.

## Requirements

- Rust 1.77+ (edition 2021)
- For the default build: system `libnetcdf-dev` / `libhdf5-dev`
- For static builds: `cmake`, `g++`, `m4`, `musl-tools`

## Cargo features

| Feature | Default | Description |
|---------|---------|-------------|
| `netcdf-backend` | yes | Enables the real NetCDF/HDF5 file reading backend |
| `static` | no | Compiles `libnetcdf` + `libhdf5` from source for fully self-contained static binaries (implies `netcdf-backend`) |

Build without the backend (TUI widget library only):

```bash
cargo build --release --no-default-features
```

## Build

### Development (dynamic, default)

```bash
sudo apt-get install libnetcdf-dev libhdf5-dev   # Debian/Ubuntu
cargo build --release
```

The binary links dynamically against system `libnetcdf` / `libhdf5` and is the
recommended path for local development and testing.

### Static release (musl, bundled NetCDF/HDF5)

Fully-static binaries that bundle `libnetcdf` and `libhdf5` compiled from
source, producing a single binary with zero runtime dependencies:

```bash
# x86_64
rustup target add x86_64-unknown-linux-musl
sudo apt-get install musl-tools cmake g++ m4
cargo build --release --target x86_64-unknown-linux-musl --features static

# aarch64 (cross-compile from x86_64 host)
rustup target add aarch64-unknown-linux-musl
sudo apt-get install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu cmake m4
cargo build --release --target aarch64-unknown-linux-musl --features static
```

A convenience script builds one or both targets:

```bash
./scripts/build-static.sh x86_64    # just x86_64
./scripts/build-static.sh aarch64   # just aarch64
./scripts/build-static.sh all       # both
```

Static binaries land in `target/<triple>/release/nctui`.

## Releases

Pre-built static Linux binaries (x86_64 and aarch64) with bundled
NetCDF/HDF5 are published automatically when a version tag is pushed:

```
git tag v0.8.0
git push origin v0.8.0
```

The [Release workflow](.github/workflows/release.yml) builds both architectures
via musl with `--features static`, creates a GitHub Release, and uploads
`nctui-linux-x86_64.tar.gz` and `nctui-linux-aarch64.tar.gz` with SHA-256
checksums.

## Usage

```
nctui <file.nc>
```

Opens an interactive terminal UI with the dataset's variables displayed in a
tree on the left and a heatmap/stats panel on the right.

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` / `↓` | Navigate the variable tree |
| `Enter` / `Space` | Select variable (load heatmap + stats) or expand/collapse group |
| `g` / `G` | Jump to top / bottom of tree |
| `/` | Open fuzzy search bar |
| `Esc` | Close modal / cancel search / clear filter |
| `h` | Toggle histogram overlay |
| `t` | Toggle table preview |
| `s` | Open dimension slicer (for variables with >2 dimensions) |
| `?` | Toggle help overlay |
| `q` / `Ctrl+C` | Quit |

**Inside histogram overlay:**

| Key | Action |
|-----|--------|
| `+` / `=` | Increase bin count |
| `-` | Decrease bin count |
| `Esc` / `h` | Close |

**Inside table preview:**

| Key | Action |
|-----|--------|
| `↑` / `↓` / `j` / `k` | Scroll rows |
| `←` / `→` / `h` / `l` | Scroll columns |
| `PgUp` / `PgDn` | Scroll 20 rows |
| `Esc` / `t` | Close |

**Inside dimension slicer:**

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` / `↓` | Select dimension row |
| `x` / `y` / `f` | Assign X-axis / Y-axis / Fixed role |
| `h` / `l` or `←` / `→` | Decrement / increment fixed index |
| `Enter` | Confirm slice |
| `Esc` | Cancel |

## Tests

The test suite includes per-module unit tests and snapshot tests (via [insta](https://insta.rs/)) that render each widget into a buffer and compare against golden files.

```
cargo test                   # run all tests (needs libnetcdf-dev)
cargo test --no-default-features   # run TUI widget tests only
cargo insta test --review    # review snapshot changes after code edits
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release history.

## License

See repository for license details.
