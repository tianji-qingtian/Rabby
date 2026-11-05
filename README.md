# Rabby

Rabby is a Rust rewrite of Tabby: a terminal, SSH and serial client for macOS,
Windows and Linux. The plan lives in .scratch/rust-rewrite/; implementation
starts at M0 in crates/.

## Status

- Blueprint: complete (map 00, 17/17 tickets; specs in .scratch/rust-rewrite/specs/).
- M0: workspace + crate skeleton + local PTY vertical slice (GPUI Kit window,
  wezterm-term screen model, portable-pty shell).
- M1: multi-tab workspace + nested PaneTree splits + rename/close/maximize
  (macOS dogfood; see .scratch/implementation/m1.md).

## Build

The workspace pins a nightly toolchain because GPUI Kit currently depends on
GPUI pre-release snapshots. Locally:

    cargo build
    cargo test --workspace
    cargo run -p rabby-app

The prototype/ directory is throwaway evidence from ticket 09 and is excluded
from the workspace.

## Layout

- crates/rabby-core: domain types
- crates/rabby-config: config.toml/state.json (skeleton)
- crates/rabby-secrets: keychain/vault (skeleton)
- crates/rabby-import-tabby: Tabby importer (skeleton)
- crates/rabby-terminal: TerminalCore over wezterm-term
- crates/rabby-pty: portable-pty local session
- crates/rabby-ssh: SSH transport (skeleton)
- crates/rabby-session: session orchestration (skeleton)
- crates/rabby-platform: shell catalog/platform services
- crates/rabby-terminal-view: TerminalElement / TerminalPane
- crates/rabby-ui: Workspace / AppShell
- crates/rabby-ext-api: extension ABI (skeleton)
- crates/rabby-ext-host: extension host (skeleton, v1.x)
- crates/rabby-app: application entry point
- crates/rabby-test-support: fixtures/mocks
- prototype/: 09 throwaway prototype (excluded)
- .scratch/rust-rewrite/: map, research, specs

## Docs

- Blueprint index: .scratch/rust-rewrite/specs/README.md
- Map: .scratch/rust-rewrite/map.md
