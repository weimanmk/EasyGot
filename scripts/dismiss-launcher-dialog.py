"""Clear only the stock emulator launcher's ANR, never application failures."""
import json
import re
import subprocess
import time
from pathlib import Path
import xml.etree.ElementTree as ET

out = Path('qa-captures')
out.mkdir(exist_ok=True)
dismissed = []
for attempt in range(3):
    remote = f'/sdcard/easygot-startup-{attempt}.xml'
    subprocess.run(['adb', 'shell', 'uiautomator', 'dump', remote], check=True, stdout=subprocess.DEVNULL)
    local = out / f'startup-dialog-{attempt}.xml'
    subprocess.run(['adb', 'pull', remote, str(local)], check=True, stdout=subprocess.DEVNULL)
    nodes = list(ET.parse(local).iter('node'))
    titles = [n.attrib.get('text', '') for n in nodes if n.attrib.get('resource-id') == 'android:id/alertTitle']
    launcher = next((title for title in titles if ('Quickstep' in title or 'Pixel Launcher' in title) and 'responding' in title), None)
    if launcher is None:
        assert not any('EasyGot' in title and 'responding' in title for title in titles), titles
        break
    button = next(n for n in nodes if n.attrib.get('resource-id') == 'android:id/aerr_close')
    x1, y1, x2, y2 = map(int, re.findall(r'\d+', button.attrib['bounds']))
    subprocess.run(['adb', 'shell', 'input', 'tap', str((x1+x2)//2), str((y1+y2)//2)], check=True)
    dismissed.append(launcher)
    time.sleep(2)
else:
    raise RuntimeError('Emulator launcher error did not close')
(out / 'launcher-dialog-recovery.json').write_text(json.dumps({'dismissed': dismissed}))
