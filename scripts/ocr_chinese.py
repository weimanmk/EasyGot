"""Screenshot OCR evidence, retaining the input and every recognition pass.

Sparse-page OCR alone skips white text on blue controls and missegments small
navigation captions. Use a grayscale enlarged page with both block and sparse
segmentation as well. A required phrase must be intact in an actual OCR result.
"""
import json
import subprocess
from pathlib import Path
from PIL import Image, ImageOps


def ocr_capture(path: Path, required):
    with Image.open(path) as source:
        gray = ImageOps.grayscale(source)
        gray = gray.resize((gray.width * 3, gray.height * 3), Image.Resampling.LANCZOS)
    enlarged = path.with_name(path.stem + '-ocr-gray3.png')
    gray.save(enlarged)
    passes = []
    for label, image, mode in [('original-sparse', path, '11'),
                                ('gray3-block', enlarged, '6'),
                                ('gray3-sparse', enlarged, '11')]:
        result = subprocess.run(['tesseract', str(image), 'stdout', '-l', 'chi_sim+eng', '--psm', mode],
                                check=True, capture_output=True, text=True)
        path.with_name(path.stem + '-' + label + '.txt').write_text(result.stdout)
        passes.append((label, ''.join(result.stdout.split())))
    matches = {phrase: [label for label, text in passes if phrase in text] for phrase in required}
    path.with_suffix('.ocr.json').write_text(json.dumps(matches, ensure_ascii=False, indent=2) + '\n')
    missing = [phrase for phrase, labels in matches.items() if not labels]
    assert not missing, (path.name, 'OCR could not confirm required Chinese labels', missing, passes)
    return required
