# Rust migration verification — 2026-11-01

The application is now Rust with native Win32 controls, hooks and input APIs.
The C# project, implementation and test runner have been removed. The release
does not need .NET, a WebView, or third-party runtime DLLs.

## Measured delivery and startup

| Measurement | C# v1.0.1 | Rust |
| --- | ---: | ---: |
| EXE bytes | 71,855,237 | 345,088 |
| Warm startup median | 224.17 ms | 23.18 ms |

EXE size fell **99.52%**. Warm startup was **9.7× faster** on this machine.
These are warm process launches, not cold boot or first-download measurements:
one warmup per executable, then ten launches each in alternating order, measuring
process start until its main window is visible. Measurements include PowerShell
`Start-Process` overhead and can vary with Windows, hardware and antivirus state.

Raw samples and hashes: `artifacts/startup-benchmark.json`.
The final Rust EXE is `artifacts/rust-release/MouseMoverApp.exe`.
SHA-256: `661ef82e5c03cd921a017d7a8ec1780ac35cab6766dee83f1c92a92b29456ed5`.

The light UI uses DPI-scaled Segoe UI, a blue native owner-drawn action button,
a status dot with text, and aligned hours/minutes/seconds fields. Preview:

![Light native Windows interface](ui-rust-light.png)

## Verification

- `cargo fmt --check`: passed.
- `cargo test --locked`: ten passed, zero failed. Includes the complete 446-point
  path fixture generated from the old C# release implementation, idle/countdown
  transitions, hours/minutes/seconds conversion, injected-input flag filtering and
  worker cancellation replacement.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `scripts/publish.ps1 -OutputDirectory artifacts/rust-release`: passed; standalone
  EXE, two UTF-8 instruction TXT files, and SHA-256 output produced.
- `scripts/check-ui.ps1 -Executable artifacts/rust-release/MouseMoverApp.exe`:
  passed. Checks topmost fixed window, 3/0/0 defaults, all three spinner buddies/ranges,
  Start/Stop, hours affecting countdown, countdown before Start, restart, zero
  duration and global F10.
- Native render captured and visually inspected for readable text, unclipped
  bounds, consistent light backgrounds and aligned duration fields.
- PE import inspection: x64 Windows GUI subsystem; only Windows system DLLs
  (`kernel32`, `msvcrt`, `ntdll`, synchronization API set, `comctl32`, `gdi32`, `user32`).

Build environment: Rust 1.99.1, x86_64-pc-windows-gnu, compatible x64 MinGW-w64.
The native smoke test does not wait three real minutes or generate a full cursor
trace/click cycle on the desktop; those timing/state/path rules are covered by
deterministic tests and source review. Remote CI results are available in GitHub
Actions for the uploaded commit.

## Reproduce

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
./scripts/publish.ps1 -OutputDirectory artifacts/new-release
./scripts/check-ui.ps1 -Executable artifacts/new-release/MouseMoverApp.exe
./scripts/benchmark.ps1 -Baseline artifacts/v1.0.1/release/MouseMoverApp.exe -Candidate artifacts/new-release/MouseMoverApp.exe -Runs 10
```

Publishing intentionally requires an empty output directory. The benchmark needs
the prior release EXE and should run while no other instance owns global F10.
Use `-Screenshot artifacts/ui-rust-light.png` with `check-ui.ps1` to save the
application's own native window render for visual inspection.
