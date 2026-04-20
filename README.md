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

## Install with Homebrew

ipx is distributed through the `tonyassistant/homebrew-tap` tap:

```bash
brew tap tonyassistant/homebrew-tap
brew install ipx
```

You can also install directly from the tap in one command:

```bash
brew install tonyassistant/homebrew-tap/ipx
```

For maintainers, the release pipeline publishes macOS archives to GitHub Releases and updates the `tonyassistant/homebrew-tap` formula from those release artifacts.

## Local development

### Run the app

```bash
cargo run
```

The current TUI flow is:
- `j` / `k` or arrow keys move through visible interfaces
- `v` cycles interface visibility between all, grouped inactive, active, and inactive
- `[` / `]` or `Shift+Tab` / `Tab` switch inspector views
- `a` / `s` move through actions in the Actions view
- `p` or `:` opens the command palette, with forgiving suggestion matching for abbreviated commands
- `Enter` runs an action or confirms a gated action
- `Esc` cancels an open confirmation or closes the palette

### Quality checks

```bash
cargo fmt
cargo test
cargo build
```

## Documentation

User-facing docs live in `docs/` and currently cover:
- product overview
- quickstart
- interface navigation
- actions and confirmation behavior

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

## Releases

Homebrew delivery is configured through `cargo-dist`.

Expected maintainer flow:
- add a repository secret named `HOMEBREW_TAP_TOKEN` with push access to `tonyassistant/homebrew-tap`
- create and push a version tag like `v0.1.0`
- GitHub Actions builds macOS release archives and attaches them to the GitHub release
- GitHub Actions updates the Homebrew formula in `tonyassistant/homebrew-tap`

A tagged release and published macOS archive are required before `brew install` will succeed.
