# Bundled typography

Slint UI uses the unique `EasyGot Sans SC` family for Chinese, Latin text and numbers. Both Regular (400) and Bold (700) are embedded in the Rust library. System CJK fonts are not required for UI or reading common CJK content. The full upstream Noto Sans CJK SC faces each map 44,810 codepoints, including characters outside the basic plane. This is not a claim to cover every Unicode script.

Run `python3 -m pip install fonttools==4.61.1` then `python3 scripts/prepare_fonts.py` before Cargo builds. The script downloads pinned upstream files, validates SHA256, renames the family to avoid collision with OEM Noto subsets, verifies UI/example/corpus glyphs, and produces the two OTF assets plus a build manifest. Fonts are fetched only while building; the installed app reads embedded bytes offline.

Source: notofonts/noto-cjk commit `165c01b46ea533872e002e0785ff17e44f6d97d8`, `Sans/OTF/SimplifiedChinese/NotoSansCJKsc-{Regular,Bold}.otf`.
License: `OFL-Noto-CJK.txt`. Original copyright and license metadata are retained in the renamed faces. Material Symbols' separate Apache-2.0 LICENSE is unchanged; SVG navigation icons do not depend on font glyphs.

Validation includes Android 29 and 35, plus rooted disposable AVDs with CJK system font directories bind-masked and cold-started debug AND non-debug APKs. OCR and manual screenshots cover the actual labels reported missing by the user. A host preview with preinstalled CJK fonts alone is insufficient evidence.
