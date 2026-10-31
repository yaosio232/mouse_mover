# Pure Rust Windows migration

The existing C# WinForms program is replaced with Rust and native Win32 controls.
No .NET runtime, WebView, GUI engine, background service or network dependency is
needed at runtime. Keep the title, fixed topmost window, icon, tab navigation,
F10 toggle, physical-input detection, three-minute idle threshold, AZKi path,
click/wait cycle, and independent stop countdown described in the original specs.

Follow-up requirement: add hours/minutes/seconds numeric inputs with the original
three-hour default shown as 3/0/0. Hours allow 0..999; minutes and seconds allow
0..59. Put the duration inputs below the checkbox and increase the client height
while preserving the compact width. Once verified, commit and push to origin/main
as explicitly requested by the user.

The Win32 message loop owns UI and state; timed movement must remain cancellable
by physical input and countdown expiry before any subsequent cursor step/click.
Injected mouse/keyboard events must not reset inactivity. Preserve the corrected
keyboard injected flag (0x10), not the stale bit-0 description in the original spec.

Pure state and path logic have deterministic Rust tests. Native lifecycle and
controls are checked on Windows. Build with size optimization, LTO, one codegen
unit, stripped symbols, and panic abort. Embed the existing icon and native visual
styles. Publish a standalone x64 EXE plus the two existing instruction TXT files.

Compare EXE bytes and warm process-to-visible-window startup on the same machine
against the existing v1.0.1 standalone build. Report measured results and avoid
claiming cold-boot performance from warm launches. Old artifacts remain ignored;
remove obsolete C# source/project/tests from tracked source once parity is verified.
