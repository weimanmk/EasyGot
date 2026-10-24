"""Internal development helper: bounded GitHub tool payloads, not shipped in the APK."""
import sys,json,subprocess,base64,hashlib,os
from pathlib import Path
manifest=Path('../verification/upload-manifest.json')
if sys.argv[1]=='prepare':
    files=subprocess.check_output(['git','ls-files','-z']).decode().strip('\0').split('\0')
    entries=[]
    for path in files:
        raw=Path(path).read_bytes()
        try:raw.decode('utf-8');binary=False
        except UnicodeDecodeError:binary=True
        entries.append(dict(path=path,binary=binary,size=len(raw),hash=hashlib.sha256(raw).hexdigest(),mode='100755' if os.stat(path).st_mode&0o111 else '100644'))
    manifest.write_text(json.dumps(entries));print(json.dumps(entries))
elif sys.argv[1]=='binary':
    entries=json.loads(manifest.read_text());raw=base64.b64encode(Path(entries[int(sys.argv[2])]['path']).read_bytes()).decode();offset=int(sys.argv[3]);print(raw[offset:offset+24000])
elif sys.argv[1]=='texts':
    entries=json.loads(manifest.read_text());start=int(sys.argv[2]);selected=[];size=0;end=start
    for i in range(start,len(entries)):
        e=entries[i]
        if e['binary']:end=i+1;continue
        content=Path(e['path']).read_text()
        if selected and size+len(content)>35000:break
        selected.append(dict(path=e['path'],mode=e['mode'],type='blob',content=content));size+=len(content);end=i+1
    print(json.dumps(dict(entries=selected,end=end),ensure_ascii=False))
