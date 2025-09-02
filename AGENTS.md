# AGENTS.md - Saorsa Robotics Development Guide

## Build/Lint/Test Commands

**Rust (Workspace):**
- Build: `cargo build --workspace --all-targets`
- Format: `cargo fmt --all`
- Lint: `cargo clippy --all-features -- -D clippy::panic -D clippy::unwrap_used -D clippy::expect_used`
- Test all: `cargo test --workspace`
- Test single: `cargo test --package <crate-name> <test_name>`
- All: `make rust-all`

**Python:**
- Lint: `ruff check .`
- Format: `black .`
- Test: `python -m pytest` (if available)

## Code Style Guidelines

**Rust:**
- **Error Handling:** Use `Result<T, E>` with `?` operator. Never use `unwrap()`, `expect()`, or `panic!()` in production code
- **Imports:** Group std, external crates, then local modules. Use explicit imports
- **Naming:** snake_case for functions/variables, PascalCase for types, SCREAMING_SNAKE_CASE for constants
- **Documentation:** Use `//!` for module docs, `///` for public items
- **Safety:** `unsafe` blocks require justification and extensive review

**Python:**
- **Line Length:** 100 characters (ruff/black)
- **Imports:** Standard library first, then third-party, then local. One import per line
- **Naming:** snake_case for functions/variables, PascalCase for classes, UPPER_CASE for constants
- **Types:** Use type hints for all function parameters and return values
- **Error Handling:** Use specific exceptions, avoid bare `except:`
- **Documentation:** Use docstrings for all public functions/classes

**General:**
- **Commits:** Use conventional commits (feat:, fix:, docs:, etc.)
- **Testing:** Write tests for all new functionality. Aim for >85% coverage
- **Security:** Never log secrets, validate all inputs, use secure dependencies