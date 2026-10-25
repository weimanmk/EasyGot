#!/usr/bin/env python3
"""Fail packaging if either complete font face is absent from the actual APK.

Checks bytes in every shipped Rust library, not a host renderer or source tree.
Writes an audit record beside each APK for the downloadable build artifact.
"""
import hashlib
import json
import os
import struct
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def certificate_sha256(apk: Path):
    data = apk.read_bytes()
    end = data.rfind(b'PK\x05\x06')
    assert end >= 0, 'APK ZIP end record missing'
    central = struct.unpack_from('<I', data, end + 16)[0]
    assert data[central - 16:central] == b'APK Sig Block 42', 'APK signing block missing'
    size = struct.unpack_from('<Q', data, central - 24)[0]
    cursor = central - size

    def length_prefixed(block, offset=0):
        length = struct.unpack_from('<I', block, offset)[0]
        assert offset + 4 + length <= len(block)
        return block[offset + 4:offset + 4 + length], offset + 4 + length

    while cursor < central - 24:
        length = struct.unpack_from('<Q', data, cursor)[0]
        kind = struct.unpack_from('<I', data, cursor + 8)[0]
        block = data[cursor + 12:cursor + 8 + length]
        cursor += 8 + length
        if kind == 0x7109871a:  # Android APK signature scheme v2
            signers, _ = length_prefixed(block)
            signer, _ = length_prefixed(signers)
            signed, _ = length_prefixed(signer)
            _, offset = length_prefixed(signed)
            certificates, _ = length_prefixed(signed, offset)
            certificate, _ = length_prefixed(certificates)
            return hashlib.sha256(certificate).hexdigest()
    raise AssertionError('APK v2 certificate missing')


def check(apk: Path):
    manifest = json.loads((ROOT / 'design/fonts/font-manifest.json').read_text())
    fonts = []
    for face in manifest['fonts']:
        data = (ROOT / f"design/fonts/EasyGotSansSC-{face['style']}.otf").read_bytes()
        assert hashlib.sha256(data).hexdigest() == face['outputSha256']
        fonts.append((face, data))
    libraries = []
    with zipfile.ZipFile(apk) as archive:
        for name in archive.namelist():
            if name.startswith('lib/') and name.endswith('/libeasygot.so'):
                library = archive.read(name)
                for face, data in fonts:
                    assert data in library, f"{apk}: {face['style']} font not embedded in {name}"
                libraries.append(name)
    assert libraries, f'{apk}: no Rust Android library found'
    certificate = certificate_sha256(apk)
    pin = ROOT / 'docs/preview-certificate-sha256.txt'
    if os.environ.get('EASYGOT_PREVIEW_KEYSTORE') and pin.exists():
        assert certificate == pin.read_text().strip(), 'Published preview signing identity changed'
    result = {
        'apk': apk.name,
        'apkSha256': hashlib.sha256(apk.read_bytes()).hexdigest(),
        'bytes': apk.stat().st_size,
        'libraries': libraries,
        'certificateSha256': certificate,
        'completeFontBytesEmbedded': True,
        'fontManifest': manifest,
    }
    apk.with_suffix('.font-check.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({key: value for key, value in result.items() if key != 'fontManifest'}))


if __name__ == '__main__':
    assert len(sys.argv) > 1, 'Usage: check_apk_fonts.py actual.apk [another.apk]'
    for name in sys.argv[1:]:
        check(Path(name))
