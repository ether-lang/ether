# Ether

Ether is a research-grade, high-performance interpreted language implementation focused on language features for machine learning and AI-oriented programming. It provides a compact lexer, parser, compiler, and a small virtual machine (VM) for executing a bytecode-like instruction set. The repository currently contains a prototype interpreter/compiler written in Rust.

Authors
- Richard Ore
- Ether Language Foundation

Status
- Prototype / experimental.

Goals & Vision
- Build a fast, expressive language primarily aimed at AI researchers and systems programmers.
- Provide first-class tensor primitives and convenient high-level constructs (pattern matching, errors, ranges, concise syntax).
- Evolve into a modular toolchain: language spec -> compiler -> optimizer -> native backends.

Key Features (implemented / in-progress)
- Python/JS-like syntax
- Optional type inference
- Bytecode compiler and a small but fast VM 
- Built-in operations for tensors and basic neural primitives (matmul, relu, softmax etc)
- Lists, maps, slicing, ranges, for-in loops, and match expressions etc

Quick start
1. Install Rust (rustup) if you don't already have it: https://rustup.rs
2. From the project root, build and run the demo:

    cargo run

This will build the crate and run the demo `main` in `src/main.rs`, which demonstrates core language features.

Building the release binary:

    cargo build --release

Running as a library
- The crate exposes `compile_and_run(source: &str) -> Result<(), String>` which can be used from other Rust code. See `src/main.rs` for example usage.

Roadmap (high-level)
- [ ] Module & import system (allow code to be split across files; `import` keyword)
- [ ] Add CLI to run a file and REPL mode (currently main runs embedded examples)
- [ ] Classes with multiple inheritance and visibility rules (private members using leading `_`)
- [ ] Enumerations (enums) and improved pattern matching
- [ ] Improved type system and optional type-checker pass
- [ ] Improved AI primitives support
- [ ] Language level multi-model support
- [ ] Tests, CI, and packaging

Contributing
- Issues and PRs are welcome. For significant changes, open an issue to discuss the design first.
- Code style: use `rustfmt` (cargo fmt) and prefer idiomatic Rust.

License
- This project is released under the BSD 2-Clause "Simplified" License — see `LICENSE`.

Contact
- Ether Language Foundation
