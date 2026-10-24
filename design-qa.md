# EasyGot 0.2 design verification

final result: blocked

Native Android implementation. The user explicitly requested changes to the existing Rust Android app, so emulator screenshots are used instead of a browser prototype.

Source visual truth: user attachment `01-1000083136.jpg` (698 x 1536). Existing-app baseline: `02-1000083135.jpg`. The reference is a proxy dashboard; module content is intentionally replaced by learning data, sources, and paper workflows. The user subsequently requested smaller type.

Viewport: Android 35 Pixel 5 emulator, 700 x 1536 pixels, density 320 (350 x 768 logical points). Source is normalized horizontally from 698 to 700 pixels for comparison; implementation captures remain native 2x. OS status/navigation bars are real Android surfaces.

Full-view comparison evidence (local): `../verification/home-comparison-before.png` and `../verification/home-comparison-fonts.png`. Focused typography/hero evidence: `../verification/header-hero-comparison.png`. Supplied reference screenshots are not copied into the public repository.

## Iteration history

- Initial build `0e73f08`: emulator cold start stalled; no valid UI comparison. Treated as blocked. Added startup logging and diagnostic capture, increased emulator settling time. Subsequent runs have no ANR.
- `e107679`, evidence [qa/run-40063404224](https://github.com/weimanmk/EasyGot/tree/qa/run-40063404224/qa-captures): native paste passed and A4 PDF rendered correctly. P1: assistant ball started on the left, covering content. P2: nav/sheet overflow and oversized card spacing. P2: CJK face selected as the primary Latin/digit font, poor typography balance. Fixed placement/drag handling, width calculations, Android font fallback and smaller type scale.
- `610d5d1`, evidence [qa/run-40069372974](https://github.com/weimanmk/EasyGot/tree/qa/run-40069372974/qa-captures): paste passed; ball moved from (274,552) to (237.80,488.65); nav and sheet fit. Body text 14, support 12-13, headings 23. P2: header rows still too tall; inactive choice controls need visible outlines. Fixes are in the next build. Directory screen must also show the actual paper rows after scrolling.

- `da374fd`: build and Rust tests passed. Pixel Launcher produced a system ANR dialog during the emulator density change and obscured the drag test. The app was rendered behind it. Treat this capture as invalid; stop the unused launcher while exercising EasyGot, then rerun. Bank directory controls are now consolidated above the paper list.

- `0e3fe79`: native home rendered with compact typography and no app ANR, but the emulator failed to deliver the first QA intent after its launcher/window relaunch. Replacing the Google-service image with AOSP and the visual runner with Ubuntu 22.04, allowing density changes to settle before launch, and requiring timestamped acknowledgments for checkpoints. Final capture remains pending.

## Fidelity surfaces

- Typography: Android Roboto Latin/numerals and Simplified Chinese CJK face; icon font isolated from text. Smaller type requested by user. Pending final screen capture for question wrapping and headers.
- Rhythm: 28-point card corners, grey insets, pill navigation. Reduced horizontal section-row height. Pending final comparison.
- Tokens: grey-lilac background, white cards, blue primary actions, green local-ready badge; black/grey text. Choice outlines restored for contrast.
- Assets: image-generated blue paper/check launcher icon, multi-size ICO, real Material Symbols font. Adaptive icon receives mask-safe padding. PDF uses native Chinese-capable Android text rendering.
- Copy: learning-specific content replaces proxy details. Sources are linked per bank; multi-paper directory labels and editable OCR review are explicit.

## Primary interactions and runtime evidence

16 Rust tests and ARM64 build/lint passed on `610d5d1`. Android 35 tested native password paste with a synthetic key, actual ball drag, contextual assistant, multi-paper import, question screen and PDF generation. A4 sample PDF: 595 x 842 points, Chinese text and answer lines inspected; standard answers excluded. Logs contain no app crash or ANR in successful runs.

Latest successful debug flow: [qa/run-40070932287](https://github.com/weimanmk/EasyGot/tree/qa/run-40070932287/qa-captures), including directory rows and visible radio outlines. The preview variant correctly reported `package not debuggable`; the original harness treated adb exec-out status 0 as access success, so its assertion has been corrected to inspect the actual diagnostic. Ubuntu 22 lacks ripgrep, so runtime crash checks now use grep. Final density-change and preview-launch checks remain pending.

Remaining blocker: compare the final compact home action pills and header spacing after the last adjustment; finish the published-preview smoke test. Real phone/IME, real AI recognition accuracy, DeepSeek service and two-device WebDAV are not validated by these emulator runs.
