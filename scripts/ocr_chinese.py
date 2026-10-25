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
    variants = [('original-sparse', path, '11'), ('gray3-block', enlarged, '6'),
                ('gray3-sparse', enlarged, '11')]
    # Older distribution OCR models may treat a blue button as an illustration.
    # Find blue horizontal controls by pixels and invert their actual screenshots,
    # so white glyphs become black ink. Do not inject expected text into images.
    with Image.open(path) as source:
        source = source.convert('RGB')
        def is_blue(pixel):
            return pixel[0] < 80 and 70 < pixel[1] < 190 and pixel[2] > 210
        rows = []
        for y in range(source.height):
            blue = sum(1 for x in range(0, source.width, 4)
                       if is_blue(source.getpixel((x, y))))
            if blue > source.width / 4 * .28:
                rows.append(y)
        bands = []
        for y in rows:
            if not bands or y > bands[-1][-1] + 1:
                bands.append([])
            bands[-1].append(y)
        for index, band in enumerate(bands):
            if len(band) < 15:
                continue
            blue_x = [x for x in range(source.width) if is_blue(source.getpixel((x, band[len(band) // 2])))]
            inset_y = max(3, int(len(band) * .18))
            crop = source.crop((min(blue_x) + 24, band[0] + inset_y,
                                max(blue_x) - 24, band[-1] - inset_y))
            crop = ImageOps.invert(ImageOps.grayscale(crop)).point(lambda v: 0 if v < 100 else 255)
            crop = crop.resize((crop.width * 3, crop.height * 3), Image.Resampling.LANCZOS)
            image = path.with_name(path.stem + f'-ocr-blue-{index}.png')
            crop.save(image)
            variants.append((f'blue-{index}-block', image, '6'))
            variants.append((f'blue-{index}-line', image, '7'))
    for label, image, mode in variants:
        result = subprocess.run(['tesseract', str(image), 'stdout', '-l', 'chi_sim+eng', '--psm', mode],
                                check=True, capture_output=True, text=True)
        path.with_name(path.stem + '-' + label + '.txt').write_text(result.stdout)
        passes.append((label, ''.join(result.stdout.split())))
    matches = {phrase: [label for label, text in passes if phrase in text] for phrase in required}
    path.with_suffix('.ocr.json').write_text(json.dumps(matches, ensure_ascii=False, indent=2) + '\n')
    missing = [phrase for phrase, labels in matches.items() if not labels]
    assert not missing, (path.name, 'OCR could not confirm required Chinese labels', missing, passes)
    return required
