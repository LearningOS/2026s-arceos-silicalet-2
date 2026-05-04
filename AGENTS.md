# Repository Guidelines

## Project Structure & Module Organization
This repository is a training-oriented ArceOS fork.

- `arceos/`: main Rust workspace (kernel modules, APIs, user libs, `tour/`, `exercises/`, build scripts).
- `scripts/`: root-level grading and smoke-test entry points (for example `total-test.sh`).
- `crates/kernel_guard/`: locally patched dependency wired through `[patch.crates-io]`.
- `challenges/`, `course/`, `tour_books/`: challenge statements and teaching materials (not core runtime code).

Most code changes happen under `arceos/` and occasionally `scripts/`.

## Build, Test, and Development Commands
Run kernel/app build commands in `arceos/` unless noted.

- `make A=tour/u_1_0`: build a target app with default `riscv64-qemu-virt`.
- `make run A=exercises/print_with_color ARCH=riscv64 LOG=info`: build and boot in QEMU.
- `make fmt` and `make clippy`: Rust formatting and lint checks.
- `make unittest`: runs `cargo test -p axfs --features myfs` and workspace tests.
- `./scripts/total-test.sh` (repo root): runs all stage exercise checks and prints the total score.
- `./scripts/tour_test.sh`: regression-style run across `tour/*` apps.

## Coding Style & Naming Conventions
- Rust: follow `rustfmt` defaults; keep `clippy` warnings addressed before PR.
- C (`arceos/ulib/axlibc`): format with `make fmt_c` (`.clang-format` uses 4-space indent, 100-column limit).
- Use idiomatic Rust naming (`snake_case` functions/modules, `CamelCase` types).
- Keep naming consistent with existing paths, e.g. `scripts/test-<feature>.sh`, `tour/u_*`, `tour/m_*`, `tour/h_*`.

## Testing Guidelines
- Prefer fast local validation first: `make unittest`, then target-specific `make run A=...`.
- For graded exercises, ensure `./scripts/total-test.sh` passes locally before submitting.
- New behavior should include at least one automated check (unit test or script assertion) and a clear failure signal (`exit 1` on failure).

## Commit & Pull Request Guidelines
- Commit style in history is short, imperative, and scoped when useful (example: `Fix CI: ...`).
- Recommended format: `<Scope>: <imperative summary>` or direct imperative summary.
- PRs should include: purpose, key changed paths, verification commands run, and relevant logs/output snippets.
- Keep PRs focused; avoid mixing challenge-material edits with kernel/runtime changes in one PR.
