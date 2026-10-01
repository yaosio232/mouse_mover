# AZu Sanpo Implementation Plan

**Goal:** Rebuild the four supplied specifications and publish source plus a standalone Windows x64 EXE on GitHub.

**Architecture:** Native WinForms owns UI and timer events. A small state model handles idle/countdown transitions; low-level hooks report physical input; cancellable strategies move the cursor. No external packages or AI decisions.

**Tech Stack:** C#, installed .NET 8 SDK, WinForms, Win32, GitHub CLI.

**Spec:** `01-FUNCTIONAL-SPEC.md`, `02-UI-SPEC.md`, `03-MOVEMENT-SPEC.md`, `04-PLATFORM-QA.md`.

## Constraints

- Preserve 290×138 client area, native controls, F10, three minute idle threshold, 60 second repeat wait, 446 path points, no added UI.
- Use .NET 8 instead of historical .NET 6; self-contained win-x64, single-file, native-library extraction, no trimming, no PDB.
- User clarified the repository must be public; never upload credentials, personal settings or build intermediates.

## Tasks (execute inline in this already isolated new project)

- [x] Initialize Git, commit supplied specs and plan, create `yaosio232/mouse_mover`, push before implementation. Visibility changed to public on user instruction.
- [x] Create `tests/MouseMoverApp.Checks` console checks with no test framework; first run against empty model/path stubs to observe failures. Check 180-second threshold, physical-input cancellation state, countdown completion priority, restart/zero/off, path count/endpoints/centering, rounding/clipping and cancellation.
- [x] Implement `ActivityState.cs`, `AzkiMouseMoveStrategy.cs`, `MouseMoveStrategyContext.cs`, `RandomMouseMoveStrategy.cs`, `IMouseMoveStrategy.cs`; rerun checks with `dotnet run --project tests/MouseMoverApp.Checks`.
- [x] Implement `Program.cs`, `Form1.cs`, `Form1.Designer.cs`, `PhysicalInputMonitor.cs`, `NativeMethods.cs`, `MouseMoverApp.csproj`; connect UI and Windows API lifecycle. Build with `dotnet build -c Release`.
- [x] Create `scripts/publish.ps1`, GitHub build/artifact workflow, README and verification record. Verify real WinForms state transitions, injected input filtering, hotkey, icon/UI bitmap and published EXE launch. Preserve a manual physical-input acceptance checklist. Publish release explicitly from the verified local EXE.
- [ ] Run checks/build/publish, assert the output contains exactly one EXE, compute SHA256, commit and push source; tag `v1.0.0`, upload EXE and checksum with `gh release create`, confirm published assets and clean Git status.
