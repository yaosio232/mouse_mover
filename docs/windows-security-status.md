# Windows security checks — 2026-11-03

The report identified Trend Micro Apex One and the detection name
`Troj.Win32.TRX.XXPE50FFF110`. Issue #1 records local reproduction with the
original release build and an eight-minute observation without the detection
after restoring Cargo's default release profile. The vendor's exact detection
basis remains unknown. This release must not be described as antivirus-approved
or as a confirmed false-positive fix.

Local checks found:

- The former C# release and current Rust release are both unsigned.
- Both implementations install the same low-level mouse and keyboard hooks for
  physical-input activity. The Rust port did not introduce keyboard recording.
- The Rust executable lacked the PE product/version information present in C#.
  The build now embeds a product name, description, version and original filename.
  This is standard release metadata, not a digital signature or an antivirus fix.
- Microsoft Defender was enabled locally, with signatures `1.463.93.0`.
  Explicit custom scans completed for both the prior Rust executable and the
  metadata-updated executable, with no threat records matching either test path.
  This does not establish the result on another computer or security product.

SmartScreen file reputation is separate from antivirus malware detection. Each
new unsigned file hash must establish its own reputation; changing the language
or keeping the same tag/download URL does not preserve that file identity.
See Microsoft's [SmartScreen reputation guidance](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation).

Trusted Authenticode signing requires a validated publisher identity and a
signing certificate/service. No such credential was supplied, so the executable
remains unsigned. Signing helps identify the publisher but does not guarantee
that every security product will accept the application. For a malware detection,
use the affected vendor's analysis process; Microsoft's [developer FAQ](https://learn.microsoft.com/en-us/defender-xdr/developer-faq)
and [submission portal](https://www.microsoft.com/en-us/wdsi/filesubmission) describe
its process. No samples were uploaded to a third-party analysis service here.

Updated build: `artifacts/version-info-release/MouseMoverApp.exe`.
SHA-256: `b25464746075431955d66f70086c8f693e685a0425ab8b6aa365fa84018214a0`.

Build verification: formatting, 15 tests, Clippy and the release build pass.
The local UI check reached its global-F10 step and failed because an existing
copy of the preceding release owned F10. That user instance was left running;
the clean Windows CI job provides isolated hotkey verification.
