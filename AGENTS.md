# Repository Guidelines

## Project Structure & Module Organization
- `crates/*`: Rust libraries (e.g., `can-transport`, `device-registry`, `vision-stereo`).
- `apps/*`: Rust binaries (e.g., `sr-cli`, `brain-daemon`, `kyutai-stt-app`).
- `examples/*`: Small runnable demos; `configs/*`: runtime settings; `docs/*`: design notes; `rl/*`: training scripts; `archive/*`: legacy.
- Tests: unit tests in `src` modules; integration tests in `crates/<name>/tests`.

## Build, Test, and Development Commands
- Build (workspace): `cargo build --workspace --all-targets`.
- Format: `cargo fmt --all`.
- Lint (panic-free policy): `cargo clippy --all-features -- -D clippy::panic -D clippy::unwrap_used -D clippy::expect_used -W clippy::pedantic`.
- Test: `cargo test --workspace` (single: `cargo test -p <crate> <name>`).
- Run an app: `cargo run -p sr-cli -- <args>`.
- Convenience: `make rust-all` (fmt + build + clippy).
- Python tooling (where present): `ruff check .`, `black .`, `pytest`.
- Tip: structural code search with `sg -lang rust -p '<pattern>'`.

## Coding Style & Naming Conventions
- Rust: use `Result` + `?`; forbid `unwrap/expect/panic` outside tests; workspace forbids `unsafe`. Prefer `thiserror` in libs and `tracing` for structured logs. Docs with `//!` (modules) and `///` (items). Names: `snake_case` funcs/vars, `PascalCase` types, `SCREAMING_SNAKE_CASE` consts.
- Python: line length 100 (ruff/black); one import per line; type hints for public APIs.

## Testing Guidelines
- Aim ≥85% coverage for new/changed code. Keep tests deterministic; gate hardware/IO behind features.
- Rust integration test example: `crates/can-transport/tests/can_roundtrip.rs`.
- Run all tests: `cargo test --workspace`; single crate: `cargo test -p <crate>`.

## Commit & Pull Request Guidelines
- Commits: Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`). Small, focused, with rationale in the body.
- PRs: include a clear description, linked issues, reproduction/test steps, and logs/screenshots if relevant. CI must pass (fmt, clippy, tests).

## Security & Configuration Tips
- Do not commit secrets. Copy `.env.example` → `.env` for local runs.
- Validate inputs and return precise errors; prefer `anyhow` in bins and `thiserror` in libraries.
