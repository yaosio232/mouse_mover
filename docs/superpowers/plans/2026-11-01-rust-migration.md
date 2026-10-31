# Rust Migration Implementation Plan

**Goal:** Preserve the current mouse mover while reducing startup delay and delivery size.

**Architecture:** Rust state/path logic plus native Win32 controls and input hooks.
The UI event loop coordinates cancellation and countdowns without a GUI framework.

**Tech stack:** Rust stable, Windows x64, Windows API bindings.

**Spec:** ../specs/2026-11-01-rust-design.md

## Work

- [x] Port existing state/path regression checks to Rust and implement the native app.
  Files: Cargo.toml, Cargo.lock, build.rs, src/ and Rust tests.
  Verify: cargo test, cargo fmt --check, cargo clippy --all-targets -- -D warnings.
- [x] Replace .NET build/publish/CI instructions with Cargo and standalone packaging.
  Files: scripts/publish.ps1, .github/workflows/, README.md, docs/README_*.txt, .gitignore.
  Verify: release build and publish script, delivery file list and checksum.
- [x] Check native lifecycle, controls, F10, countdown and input handling; compare
  startup/file size against the existing release. Inspect the actual diff and
  remove obsolete C# source after parity checks. Record reproducible measurements.

Use the existing checkout. The initial diff was clean. JEV routing selected HIGH
(Luna high) with confidence 0.99; delegate implementation accordingly, and reserve
JEV for bounded workflow decisions after deterministic checks.
