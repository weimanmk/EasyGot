# EasyGot 0.2 visual verification

final result: passed

Native Rust/Android implementation, as explicitly requested by the user. Screenshots are captured from the app on an Android emulator; no browser prototype substitutes for the native app.

## Evidence and normalization

- Source visual truth: user attachment `01-1000083136.jpg`, 698 x 1536 pixels; existing-app baseline `02-1000083135.jpg`. The source is a proxy dashboard; learning content intentionally replaces proxy data. The user subsequently requested smaller type.
- Final source: `ed99dfd234705379108af89b0101eac558effe36`.
- [Final native captures and runtime results](https://github.com/weimanmk/EasyGot/tree/qa/run-40074429762/qa-captures): home, settings, assistant, directory and scrolled directory, question, paper, preview-home, native dialog and JSON checks.
- Viewport: Android 35 AOSP Pixel 5, 700 x 1536 pixels, density 320; 350 x 768 logical points. Source width is normalized from 698 to 700 pixels. Both sides are compared at equal pixel dimensions. Android status and navigation bars are real; the emulator uses three-button navigation while the source uses gestures.
- Full comparison: `../verification/accepted-home-comparison.png`. Focused typography/header: `../verification/accepted-header-comparison.png`. Focused shortcut/navigation: `../verification/accepted-shortcuts-nav-comparison.png`. These comparisons include the supplied reference and remain in this conversation workspace; the user's reference screenshot is not copied into the public repository.
- State: empty learning dashboard and initial ball position; separate preview upgrade capture retains 24 fixture questions. Other captures exercise selected paper, OCR/print entry, settings and assistant states. Live-model responses are outside this visual comparison.

## Comparison history

1. `0e73f08`: first emulator cold start stalled; treated as blocked, with no valid design comparison. Added startup diagnostics and settling time.
2. `e107679`, [captures](https://github.com/weimanmk/EasyGot/tree/qa/run-40063404224/qa-captures): native paste and A4 PDF worked. P1 ball covered content at the left edge. P2 nav/sheet width overflow and excessive card spacing. P2 primary CJK font affected Latin/digit metrics. Fixed placement and actual drag, width calculations, system font fallback and type scale.
3. `610d5d1`, [captures](https://github.com/weimanmk/EasyGot/tree/qa/run-40069372974/qa-captures): correct nav/sheet bounds, smaller type, native paste and actual drag. P2 oversized horizontal section rows and low-visibility choice controls. Reduced header-row sizing and restored control outlines.
4. `da374fd` and `0e3fe79`: invalid interaction captures due to stock launcher ANR/window relaunch during emulator configuration. Retained diagnostics, moved visual testing to Ubuntu 22/AOSP, waited for display changes, preserved NativeActivity across density/font configuration changes and required timestamped app acknowledgments.
5. `365c28a`, [captures](https://github.com/weimanmk/EasyGot/tree/qa/run-40070932287/qa-captures): full debug flow worked. Corrected an assertion that mistook adb exec-out's transport status for successful debug access. Runtime checks use grep where ripgrep is absent.
6. `036227a`, [captures](https://github.com/weimanmk/EasyGot/tree/qa/run-40071646822/qa-captures): complete debug/preview smoke passed. P2 mixed-font shortcut layout clipped a label and default ball covered a shortcut. Centered the real-font icon/label within standard buttons and moved the initial ball to the unused status-card corner.
7. `aa01016`: corrected areas rendered, but Quickstep's system dialog obscured input. Added a helper that only recognizes and closes Quickstep/Pixel Launcher ANR dialogs; an EasyGot ANR still fails.
8. `ed99dfd`, final captures linked above: complete labels, all three home modules, initial ball clear of primary controls, no nav/sheet overflow, visible radio/checkbox boundaries and readable directory rows. Debug and published-preview checks passed. No actionable P0/P1/P2 differences remain.

## Required fidelity surfaces

- Typography: Android Roboto for Latin/numerals and Simplified Chinese CJK face, separate heading fallback and real Material Symbols icon font. Body/buttons 14, supporting labels 12-13, section headings 17, screen headings 23. Chinese and mixed text remain legible, with expected wrapping. The smaller scale is the user's explicit preference.
- Layout rhythm: 28-point white cards, grey inset statistics, compact card headers, blue/grey pill actions and floating bottom navigation. Three core home modules fit above the app navigation. Insets account for real Android system bars. Assistant hides its ball while the sheet is open, and may be dragged elsewhere.
- Colors: grey-lilac background, white cards, blue actions, green local-ready badge and black/grey text. Borders make input and choice controls visible. This is not a formal contrast certification.
- Assets: image-generated blue paper/check launcher icon and multi-size ICO; launcher mask-safe padding. Library icon font rather than drawn placeholder symbols. No rasterized system chrome. PDF uses Android's Chinese-capable text renderer.
- Copy: learning-specific content, linked sources, named paper directories and explicit OCR review. App-only information replaces the reference's proxy-specific labels; proxy graphics/brand are intentionally not copied.

## Interaction and runtime verification

16 Rust tests pass, including legacy JSON, ordered ten-paper import/export round trips, invalid directory references, bank-material binding, paper numbering/answer exclusion, timing, scoring and photo asset retention. ARM64 preview build and Android lint pass.

Android 35 exercised actual ball drag (274,90 to 235.47,157.43), density 320 to 321 to 320 with fresh app acknowledgments, synthetic password paste, directory import/selection state, question view, assistant sheet and native A4 PDF generation. Published preview installed successfully, retained fixture data, disabled debug access and ignored test-only intents. Runtime error captures contain no application crash; no app ANR was observed in the accepted run. Emulator-launcher recovery is kept separate from application failures.

A4 sample PDF (595 x 842 points) was rendered and visually inspected: Chinese text, stable question numbers, footer, answer lines and no reference answers in the student paper. Sample answer mapping and timing are covered by core tests.

## Remaining limits and follow-up polish

- Real-phone IME/file providers, real vision/grading/search services and two-device WebDAV are still integration checks requiring configured services/devices; emulator success does not establish model accuracy.
- P3: system font weight varies when a device lacks a CJK bold face. Complex formulas are plain text; OCR diagrams require draft review. A dragged overlay can cover content until moved.
- Reference-specific organic hero decoration and network graphs are intentionally replaced by learning statistics; not fabricated from empty study history.

Implementation checklist: typography, bounds, source links, directory behavior, preview installation and recorded review completed.
