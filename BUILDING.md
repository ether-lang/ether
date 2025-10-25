# Building Ether

This document provides step-by-step instructions for building and running the Ether Rust crate on a typical Linux development machine.

Prerequisites
- Rust toolchain (recommended via rustup). Minimum recommended toolchain: recent stable (2024+). Install: https://rustup.rs
- Basic build tools (make, gcc) are useful but not strictly required for pure Rust builds.

Quick build
1. Clone the repository (if not already):

    git clone <repo-url>
    cd ether

2. Build (debug):

    cargo build

3. Run the demo program (the repository's `main` runs embedded examples):

    cargo run

4. Build (release optimized):

    cargo build --release

Binary location
- After `cargo build --release`, the optimized binary is in `target/release/ether`.

Running only the library API
- The crate exposes `compile_and_run(source: &str) -> Result<(), String>` in `src/main.rs`. To invoke the compiler/VM programmatically, add this crate as a dependency in another Rust project or call into the function from a small runner (see `src/main.rs` for examples).

Formatting and linting
- Format the code with rustfmt:

    cargo fmt

- Run Clippy for linting (optional):

    cargo clippy -- -D warnings

Testing
- There are currently no formal unit tests committed. To add tests, create files under `tests/` or add `#[cfg(test)]` modules in source files, then run:

    cargo test

Troubleshooting
- Rust toolchain errors: make sure `rustup` is installed and the default toolchain is set:

    rustup default stable

- Edition mismatch: `Cargo.toml` uses `edition = "2024"` which requires a recent Rust toolchain. If your toolchain is older, update with `rustup update`.

Next steps (developer tasks)
- Add a CLI entrypoint to accept a source file path or REPL mode.
- Add integration tests and CI to validate cross-platform builds.
