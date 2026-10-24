# Typography

Body text uses Android Roboto Regular for Latin text and numbers, followed by the Simplified Chinese face (index 2) of NotoSansCJK-Regular.ttc. NotoSansSC-Regular.otf and DroidSansFallback.ttf support older/alternative Android builds. The default egui font chain follows as the final fallback.

Headings use Roboto Medium and the system CJK bold font when present, with regular CJK fallback. Body and button text is 14 logical points, supporting labels 12-13, section headings 17, and screen headings 23. Dynamic labels are wrapped within the phone width. Monospace editors preserve Latin monospace and add Chinese fallback. No additional text font packages are installed.

Material Symbols Rounded is a separate icon-only font; see LICENSE (Apache 2.0). It is never used for text.
