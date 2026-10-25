# Native font and input acceptance · 0.3.0-alpha.2

final result: passed
isolated-font acceptance: passed
native frontend checks: passed for the scope below
commercial release acceptance: pending
reviewed app sources sha256: 1930d0000ef364d64b44d4bc1cb10cb693a1aa15ace42b07b39980e666f5c573

Scope: this font, input, layout and offline-core correction is eligible for an alpha preview. It is not acceptance of every feature, OEM device or real external service.

Reviewed application sources: `c6afefc7da1888166ef4f7276f7d4ad03661bf64`.
[Full passing run](https://github.com/weimanmk/EasyGot/actions/runs/40147865307).
[Android 10 evidence](https://github.com/weimanmk/EasyGot/tree/qa/run-40147865307-api29/qa-captures).
[Android 15 evidence](https://github.com/weimanmk/EasyGot/tree/qa/run-40147865307-api35/qa-captures).

The user's real-device screenshot invalidated the previous broad font acceptance. Host font injection and an AOSP-only happy path were insufficient. The corrected app embeds two uniquely named complete Noto Sans CJK SC faces, verifies their source hashes and UI/example corpus, and no longer injects external host fonts into rendering.

Both disposable AVDs passed actual touches, objective grading, soft-keyboard entry, force-stop recovery, system Back, tenth-paper scope, direct clipboard paste, the native secret-input button, encrypted key save and cold-start reload. Native password fields report `password=true`; neither UI dump contains the synthetic test key. This test discovered and fixed the real native-dialog plaintext problem rather than weakening its assertion.

For each API, system CJK font directories were bind-masked to Latin-only/empty directories; debug and non-debug APKs were then cold-started. All four font checkpoints passed with an empty failure list. OCR retains every input and output pass, including contrast-normalized actual blue-button crops. Manual inspection of the isolated settings, home and font specimen captures confirmed the previously missing labels and the rare-character corpus. Final 350dp/412dp and 1.0/1.3 layouts, keyboard and assistant captures were also checked. Native font directories were restored after testing.

29 Rust tests, Slint compilation/adapter/format checks, ARM64 packaging and Android lint passed. Both complete font byte arrays were found in the actual ARM64 and emulator libraries; APK signatures were independently checked with apksigner. The preview certificate cache was restored in separate builds and matched the public pin. Older previews had different, lost signing keys; backup-before-migration instructions are mandatory.

The reviewed ARM64 artifact was 54,334,194 bytes, SHA256 `7f7011afec30ea5ecd89653b78afb6039e384273f7843316177918bf30222fee`. Rebuilds publish their own exact hash and font/signature report. The release workflow requires the same reviewed application-source fingerprint, successful build and both emulator jobs before publication. A final evidence-only screenshot addition does not change app sources.

Still pending: the user's VPS (SSH port 22 connection refused before authentication), Android 8 and OEM real-device coverage, actual vision/AI grading and DeepSeek services, two-device WebDAV, complex real documents, Chinese IME compatibility, sustained performance, TalkBack and owner-backed production signing. These remain outside this alpha acceptance.
