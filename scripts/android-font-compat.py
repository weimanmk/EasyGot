#!/usr/bin/env python3
"""Run the installed APK with CJK system font files hidden on our disposable AVD.
Does not modify a user's phone. Restore all bind mounts even after test failure.
"""
import json,subprocess,time,os
from pathlib import Path
from ocr_chinese import ocr_capture
OUT=Path('qa-captures');OUT.mkdir(exist_ok=True)
PKG='io.github.weimanmk.easygot.preview';COMP=PKG+'/io.github.weimanmk.easygot.MainActivity'
def adb(*a,check=True):return subprocess.run(['adb',*a],check=check,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True).stdout
def capture(name):
 with (OUT/(name+'.png')).open('wb')as f:subprocess.run(['adb','exec-out','screencap','-p'],stdout=f,check=True)
def launch(screen=None):
 adb('shell','am','force-stop',PKG);args=['shell','am','start','-W','-n',COMP]
 adb(*args);time.sleep(5)
 if screen:adb(*args,'--es','qaScreen',screen);time.sleep(2)
failures=[]
def ocr(name,required):
 try:return ocr_capture(OUT/(name+'.png'),required)
 except AssertionError as error:
  failures.append(str(error));return False
mounted=[];checks={}
try:
 (OUT/'font-isolation-root.txt').write_text(adb('root'));adb('wait-for-device');time.sleep(2)
 assert adb('shell','id','-u').strip()=='0','Font isolation requires the rooted disposable AVD'
 adb('shell','setenforce','0')
 (OUT/'system-fonts-before.txt').write_text(adb('shell','ls','/system/fonts'))
 for index,path in enumerate(['/system/fonts','/product/fonts','/system_ext/fonts','/vendor/fonts']):
  if 'present' not in adb('shell',f'test -d {path} && echo present',check=False):continue
  target=f'/data/local/tmp/easygot-latin-fonts-{index}';adb('shell',f'mkdir -p {target}')
  # Keep Latin faces so default UI fallback remains usable, without any CJK files.
  if index==0:adb('shell',f'cp /system/fonts/Roboto*.ttf {target}/',check=False)
  adb('shell',f'mount --bind {target} {path}');mounted.append(path)
  names=adb('shell','ls',path);assert not any(s in names for s in ['CJK','SansSC','DroidSansFallback']),names
 (OUT/'system-fonts-isolated.txt').write_text(adb('shell','ls','/system/fonts'))
 checks['systemCjkFontFilesHidden']=True
 # Reinstall debug, then cold start. No cached font collection from the happy-path run survives.
 adb('install','-r','app/build/outputs/apk/debug/app-debug.apk')
 launch('settings');capture('font-isolated-debug-settings')
 checks['debugSettingsLabels']=ocr('font-isolated-debug-settings',['设置','保存连接配置','模型与搜索','粘贴','系统输入','长按粘贴','模型名称','学习','题库','资料'])
 launch('font-check');capture('font-isolated-specimen')
 checks['specimenLabels']=ocr('font-isolated-specimen',['中文字体兼容性','设置','连接','粘贴','系统输入','模型名称','资料','错题'])
 # Exercise the actual non-debug APK too. Do not rely only on a debug font-probe screen.
 adb('install','-r','app/build/outputs/apk/preview/app-preview.apk');launch()
 capture('font-isolated-preview-home')
 checks['previewHomeLabels']=ocr('font-isolated-preview-home',['学习','学习状态','题库','资料','设置'])
 adb('shell','input','tap','612','1368');time.sleep(2);capture('font-isolated-preview-settings')
 checks['previewSettingsLabels']=ocr('font-isolated-preview-settings',['设置','保存连接配置','模型与搜索','粘贴','模型名称'])
 checks['sdk']=adb('shell','getprop','ro.build.version.sdk').strip()
 checks['failures']=failures
 (OUT/'font-compat-checks.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2))
 assert not failures,failures
finally:
 for path in reversed(mounted):adb('shell','umount',path,check=False)
 (OUT/'font-isolation-restored.txt').write_text(adb('shell','ls','/system/fonts',check=False))
 adb('shell','setenforce','1',check=False)
