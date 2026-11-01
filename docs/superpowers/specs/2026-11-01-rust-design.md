# Pure Rust Windows migration

The existing C# WinForms program is replaced with Rust and native Win32 controls.
No .NET runtime, WebView, GUI engine, background service or network dependency is
needed at runtime. Keep the title, fixed topmost window, icon, tab navigation,
F10 toggle, physical-input detection, three-minute idle threshold, AZKi path,
click/wait cycle, and independent stop countdown described in the original specs.

Provide hours/minutes/seconds numeric inputs with the original
three-hour default shown as 3/0/0. Hours allow 0..999; minutes and seconds allow
0..59. Put the duration inputs beside the checkbox in the compact width.
Once verified, commit and push to origin/main
as explicitly requested by the user.

Use a clean light appearance with a blue primary
button. Replace the stock bitmap-looking font with DPI-scaled Segoe UI, give
status, action and countdown clear spacing, align the three duration inputs,
and paint labels on a consistent light background. Keep real native controls,
keyboard focus feedback and existing accessible labels. Use GDI for the blue
button and status indicator; add no GUI framework or runtime dependency.

The window must be small and tightly arranged in at
most five rows, only slightly taller than the original 290×138 client. Target
290×148: Start/Stop (40 px high); status and idle information on one line in
different colors; countdown checkbox with h/m/s fields; countdown result.
Use four rows, within the five-row maximum. Remove the large in-window title, subtitle
and divider. Keep Segoe UI and the blue primary action.
Use 10pt body/status/field text and 11pt button text after the user's request for
larger lettering. Label the compact checkbox "Stop after" to fit without clipping.

Mouse input must update inactivity/cancellation immediately without forcing a
paint for every event. Refresh text only when it changes, redraw indicators only
on state transitions, and keep parent background erasure away from child controls.
Add a regression covering repeated input/refreshes and native control update counts.

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
