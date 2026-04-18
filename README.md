# ipx

ipx is a keyboard-first macOS network operations TUI built in Rust.

## Current state

The project is in active v1 development.

Current foundations in the repo:
- Rust application structure
- modular TUI architecture
- macOS-oriented interface discovery with fallback sample data
- command palette shell
- safe v1 action framework with confirmation gates for risky network operations
- event log and inspector layout
- test coverage for app behavior and parsing

## Repository layout

- `src/main.rs` - application entry point
- `src/lib.rs` - crate surface
- `src/app.rs` - app state and interaction model
- `src/network.rs` - network data model and macOS discovery
- `src/tui.rs` - rendering and event loop
- `tests/` - behavior and parsing tests
- `docs/` - user-facing Mintlify documentation

## Local development

### Run the app

```bash
cargo run
```

### Quality checks

```bash
cargo fmt
cargo test
cargo build
```

## Product constraints

For v1, ipx should:
- stay keyboard-first
- remain inspect-first before mutation-heavy
- make risky actions explicit
- feel calm and operator-grade
- support a polished macOS-native workflow for network inspection

The Actions tab now exposes a small safe-action catalog:
- read-only actions execute immediately
- risky actions require an explicit confirmation gate
- mutating network changes remain blocked in v1 even after confirmation

## CI/CD note

GitHub Actions workflow files are currently blocked from push because the active GitHub auth does not have `workflow` scope.
