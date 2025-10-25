# Ether

Ether is a research-grade, high-performance interpreted language implementation focused on language features for AI-oriented programming. It provides a compact lexer, parser, compiler, and a small virtual machine (VM) for executing a bytecode-like instruction set. The repository currently contains a prototype interpreter/compiler written in Rust.

Authors
- Richard Ore
- Ether Language Foundation

Status
- Prototype / experimental. The repo contains a single Rust crate that implements the core language pieces (lexer, parser, AST, compiler, VM) and a demo `main` function that runs several example programs.

Goals & Vision
- Build a fast, expressive language primarily aimed at AI researchers and systems programmers.
- Provide first-class tensor primitives and convenient high-level constructs (pattern matching, exceptions, ranges, concise syntax).
- Evolve into a modular toolchain: language spec -> compiler -> optimizer -> native backends.

Key Features (implemented / in-progress)
- Lexer and parser for a Python/JS-like surface syntax
- AST representation and a minimal static type representation
- Bytecode compiler and a small VM with error handling (try/catch/finally)
- Built-in operations for tensors and basic neural primitives (matmul, relu, softmax)
- Lists, maps, slicing, ranges, for-in loops, and match expressions

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
- Module & import system (allow code to be split across files; `import` keyword)
- Add CLI to run a file or REPL mode (currently main runs embedded examples)
- Classes with multiple inheritance and visibility rules (private members using leading `_`)
- Enumerations (enums) and improved pattern matching
- Improved type system and optional type-checker pass
- Tests, CI, and packaging

Short-term TODOs (extracted from `TODO.md`)
- Module and import support; allow `main` to accept a filepath and run a file
- Classes with multiple inheritance and private/public member rules
- Enumerations

Contributing
- Issues and PRs are welcome. For significant changes, open an issue to discuss the design first.
- Code style: use `rustfmt` (cargo fmt) and prefer idiomatic Rust.

License
- This project is released under the BSD 2-Clause "Simplified" License — see `LICENSE`.

Contact
- Ether Language Foundation
