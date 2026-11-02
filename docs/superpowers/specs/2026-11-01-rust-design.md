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

The compact client is 340×130 and contains three rows, following the user's
reference: a 40px-high blue gradient Start/Stop button with a background icon;
a round lamp and state value without the Status prefix, IDLE HH:MM:SS and In MM:SS;
then the stop checkbox and h/m/s inputs. In begins at 03:00 and clamps at zero.
There is no separate title, subtitle or fourth countdown row.

Use 10pt body/status/IDLE text, 11pt duration fields, and 12pt button text.
The stop checkbox has a 20px glyph and 12pt normal-weight caption, while remaining
a real BS_AUTOCHECKBOX with native keyboard and accessibility behavior. Its caption
shows Stop after, Stop HH:MM:SS, Stop ready, Set > 00:00 or Stop done as appropriate.
Read the checked state from BM_GETCHECK when painting; do not infer it from custom
draw notification flags.

A keyboard-accessible checkerboard button switches background transparency using
WS_EX_LAYERED and a per-pixel alpha surface. Transparent mode removes
text, badge and checkbox-caption backplates; outlined floating text remains
readable over light and dark surfaces. Buttons, checkbox glyph and inputs remain
opaque and clickable. Almost invisible nonzero alpha inside interactive control
rectangles preserves their full pointer targets; empty background stays alpha zero.
A checkbox subclass extends native activation to its full rectangle while keeping
native checked-state, keyboard and accessibility behavior. Verify actual pointer
targets and mouse actions, including the main button's rounded corners and the
checkbox's blank caption area. Changing the background must not change monitoring,
countdown or numeric values. The normal background is restored on the next click.
After verification, replace the existing v1.0.2 tag and release assets as explicitly
requested, retaining the same public download URLs.

Verify actual desktop composition over controlled black/white fixture windows,
not just layered-style flags. No user desktop content belongs in repository images.

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
