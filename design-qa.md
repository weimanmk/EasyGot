# Font compatibility correction

final result: pending
native frontend checks: previous happy-path coverage only

The user supplied a real-device screenshot with missing glyphs in 设置、保存连接配置、粘贴、系统输入、长按粘贴、模型名称 and navigation labels. This invalidates the earlier broad font acceptance. The AOSP emulator and host font injection masked OEM font coverage differences.

Correction: embed uniquely named, complete Noto Sans CJK SC Regular/Bold faces; pin/download/check/rename at build time; use them as the default UI font; remove external font injection from host captures.

Acceptance requires fresh Android 29 and 35 captures, cold-starting both debug and shipping non-debug APKs while system CJK font files are hidden on disposable rooted AVDs, glyph-map checks, OCR of actual labels, and manual inspection. Results remain pending until the exact APK/commit passes. Actual service, device and performance acceptance is separate.
