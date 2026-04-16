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

### Development (dynamic, default)

```
cargo build --release
```

The binary is written to `target/release/nctui`. It links dynamically against
the system C library (glibc on most Linux distros) and is the recommended path
for local development and testing.

### Static release (musl)

Fully-static binaries that run on any Linux without runtime dependencies:

```bash
# x86_64
rustup target add x86_64-unknown-linux-musl
sudo apt-get install musl-tools          # Debian/Ubuntu
cargo build --release --target x86_64-unknown-linux-musl

# aarch64 (cross-compile from x86_64 host)
rustup target add aarch64-unknown-linux-musl
sudo apt-get install gcc-aarch64-linux-gnu
cargo build --release --target aarch64-unknown-linux-musl
```

A convenience script builds one or both targets:

```bash
./scripts/build-static.sh x86_64    # just x86_64
./scripts/build-static.sh aarch64   # just aarch64
./scripts/build-static.sh all       # both
```

Static binaries land in `target/<triple>/release/nctui`.

## Releases

Pre-built static Linux binaries (x86_64 and aarch64) are published
automatically when a version tag is pushed:

```
git tag v0.5.4
git push origin v0.5.4
```

The [Release workflow](.github/workflows/release.yml) builds both architectures
via musl, creates a GitHub Release, and uploads `nctui-linux-x86_64.tar.gz` and
`nctui-linux-aarch64.tar.gz` with SHA-256 checksums.

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
