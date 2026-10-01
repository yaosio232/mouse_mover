Mouse Mover App

A native Windows Forms mouse movement tool, rebuilt from the four specifications in this repository.

Download and usage

Download MouseMoverApp.exe from https://github.com/yaosio232/mouse_mover/releases and run it directly. Supports Windows x64.

- Press Start (F10) or the global F10 hotkey to start monitoring physical mouse and keyboard input.
- After three minutes of inactivity, the cursor starts moving. When movement finishes, the app clicks the left mouse button once, waits 60 seconds, and repeats.
- Keyboard input or mouse movement cancels automatic movement until the next three-minute idle period.
- Press Stop or F10 to stop completely.

Additional features

- Stop countdown is an independent stop timer. Enabling it or changing the duration restarts the countdown immediately; pressing Start also restarts it. When it expires, the app displays Done and stops completely. It is disabled by default, with a preset of 180 minutes / 0 seconds.
- Closing the window stops the app. There is no background service, saved settings, or network functionality.

Build and publish

Requires Windows and the .NET 8 SDK:

dotnet build -c Release
dotnet run --project tests/MouseMoverApp.Checks -c Release
./scripts/publish.ps1

The publish directory artifacts/release contains MouseMoverApp.exe, README_EN.txt, and README_ZH-TW.txt.
