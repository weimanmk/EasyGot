#!/usr/bin/env python3
"""Fetch pinned OFL fonts at BUILD time and give the bundled family a unique name.
No runtime download and no OEM font dependency for Chinese UI/body text.
"""
import hashlib,json,sys,urllib.request
from pathlib import Path
from fontTools.ttLib import TTFont
ROOT=Path(__file__).resolve().parents[1]
COMMIT='165c01b46ea533872e002e0785ff17e44f6d97d8'
FONTS={
 'Regular':('2c76254f6fc379fddfce0a7e84fb5385bb135d3e399294f6eeb6680d0365b74b',400),
 'Bold':('b5f0d1a190a7f9b43c310a8850630af12553df32c4c050543f9059732d9b4c0a',700),
}
def sha(data):return hashlib.sha256(data).hexdigest()
def prepare():
 target=ROOT/'design/fonts';cache=target/'upstream';cache.mkdir(parents=True,exist_ok=True);report=[]
 for style,(expected,weight) in FONTS.items():
  name=f'NotoSansCJKsc-{style}.otf';source=cache/name
  if not source.exists() or sha(source.read_bytes())!=expected:
   url=f'https://raw.githubusercontent.com/notofonts/noto-cjk/{COMMIT}/Sans/OTF/SimplifiedChinese/{name}'
   data=urllib.request.urlopen(url,timeout=90).read()
   if sha(data)!=expected:raise ValueError(f'{name}: upstream SHA256 mismatch')
   source.write_bytes(data)
  font=TTFont(source,recalcTimestamp=False)
  replacements={1:'EasyGot Sans SC',16:'EasyGot Sans SC',4:f'EasyGot Sans SC {style}',6:f'EasyGotSansSC-{style}',3:f'EasyGotSansSC;2.004;{style}'}
  for record in font['name'].names:
   if record.nameID in replacements:record.string=replacements[record.nameID].encode(record.getEncoding())
  output=target/f'EasyGotSansSC-{style}.otf';font.save(output)
  assert font['OS/2'].usWeightClass==weight
  cmap=font.getBestCmap();required=set()
  for path in [*ROOT.glob('ui/**/*.slint'),*ROOT.glob('rust/src/**/*.rs'),*ROOT.glob('examples/*.json')]:
   required.update(c for c in path.read_text() if 0x3400<=ord(c)<=0x9fff or 0x20000<=ord(c)<=0x2fa1f)
  required.update('设置保存连接配置粘贴系统输入长按模型名称学习题库资料简答判断多选错题𠮷龘')
  missing=sorted(c for c in required if ord(c) not in cmap or cmap[ord(c)]=='.notdef')
  if missing:raise ValueError(f'{style}: missing characters: {missing}')
  report.append({'style':style,'family':font['name'].getDebugName(1),'weight':weight,'glyphCodepoints':len(cmap),'uiAndExamplesCharactersChecked':len(required),'missing':missing,'sourceSha256':expected,'outputSha256':sha(output.read_bytes()),'bytes':output.stat().st_size})
 (target/'font-manifest.json').write_text(json.dumps({'upstreamCommit':COMMIT,'fonts':report},ensure_ascii=False,indent=2)+'\n')
 print(json.dumps(report,ensure_ascii=False))
if __name__=='__main__':prepare()
