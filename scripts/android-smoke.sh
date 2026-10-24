#!/usr/bin/env bash
set -euo pipefail
mkdir -p qa-captures
capture_diagnostics(){
 adb logcat -d > qa-captures/logcat.txt || true
 adb shell dumpsys activity lastanr > qa-captures/last-anr.txt || true
 adb exec-out screencap -p > qa-captures/final-state.png || true
}
trap capture_diagnostics EXIT
adb shell wm size 700x1536
adb shell wm density 320
sleep 20
adb shell input keyevent KEYCODE_WAKEUP
adb shell wm dismiss-keyguard
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -W -n io.github.weimanmk.easygot.preview/io.github.weimanmk.easygot.MainActivity
sleep 20
adb shell ls /system/fonts > qa-captures/system-fonts.txt
adb shell pidof io.github.weimanmk.easygot.preview
python3 scripts/dismiss-launcher-dialog.py
adb exec-out screencap -p > qa-captures/home.png
screen_ball(){ python3 - <<'PYBALL'
import json,subprocess,time
command=['adb','exec-out','run-as','io.github.weimanmk.easygot.preview','cat','files/qa-ball.json']
def read():
 try:return json.loads(subprocess.check_output(command,stderr=subprocess.DEVNULL))
 except (subprocess.CalledProcessError,ValueError):return {}
previous=read().get('sampledAt')
subprocess.run(['adb','shell','am','start','-W','-n','io.github.weimanmk.easygot.preview/io.github.weimanmk.easygot.MainActivity','--es','qaScreen','check-ball'],check=True,stdout=subprocess.DEVNULL)
for _ in range(20):
 value=read()
 if value.get('sampledAt') is not None and value['sampledAt']!=previous:
  print(json.dumps(value));break
 time.sleep(1)
else:raise RuntimeError('App did not acknowledge the floating-ball checkpoint')
PYBALL
}
# Density/font-display changes must not detach the Java event bridge from the Rust app.
adb shell wm density 321
sleep 2
adb shell wm density 320
sleep 2
python3 scripts/dismiss-launcher-dialog.py
screen_ball > qa-captures/ball-before.json
python3 - <<'PYSWIPE'
import json,subprocess
p=json.load(open('qa-captures/ball-before.json'));x,y=int((p['x']+28)*2),int((p['y']+28)*2)
subprocess.run(['adb','shell','input','swipe',str(x),str(y),str(x-80),str(y+140),'600'],check=True)
PYSWIPE
screen_ball > qa-captures/ball-after.json
python3 - <<'PYCHECK'
import json
before=json.load(open('qa-captures/ball-before.json'));after=json.load(open('qa-captures/ball-after.json'))
assert before['x']-after['x']>15 and after['y']-before['y']>30,(before,after)
PYCHECK
adb exec-out screencap -p > qa-captures/ball-dragged.png
screen() { adb shell am start -W -n io.github.weimanmk.easygot.preview/io.github.weimanmk.easygot.MainActivity --es qaScreen "$1";sleep 3; }
screen settings
adb exec-out screencap -p > qa-captures/settings.png
screen native-paste
adb shell input keyevent 279
sleep 2
adb shell uiautomator dump /sdcard/easygot-ui.xml
adb pull /sdcard/easygot-ui.xml qa-captures/native-dialog.xml
python3 - <<'PY'
import xml.etree.ElementTree as E,re,subprocess
root=E.parse('qa-captures/native-dialog.xml')
button=next(n for n in root.iter('node')if n.attrib.get('resource-id')=='android:id/button1')
x1,y1,x2,y2=map(int,re.findall(r'\d+',button.attrib['bounds']))
subprocess.run(['adb','shell','input','tap',str((x1+x2)//2),str((y1+y2)//2)],check=True)
PY
sleep 2
screen check-paste
adb exec-out run-as io.github.weimanmk.easygot.preview cat files/qa-paste-result.json > qa-captures/paste-result.json
python3 -c 'import json;assert json.load(open("qa-captures/paste-result.json"))["passed"],"Native password paste failed"'
screen home
screen assistant
adb exec-out screencap -p > qa-captures/assistant.png
screen directory
adb exec-out screencap -p > qa-captures/directory.png
adb shell input swipe 350 1150 350 450 700
sleep 2
adb exec-out screencap -p > qa-captures/directory-scrolled.png
screen question
adb exec-out screencap -p > qa-captures/question.png
screen paper
adb exec-out screencap -p > qa-captures/paper.png
PDF_PATH=$(adb exec-out run-as io.github.weimanmk.easygot.preview cat files/qa-paper-path.txt)
adb exec-out run-as io.github.weimanmk.easygot.preview cat "$PDF_PATH" > qa-captures/sample-paper.pdf
adb exec-out run-as io.github.weimanmk.easygot.preview cat files/qa-print-spec.json > qa-captures/sample-print-spec.json
adb logcat -d -s AndroidRuntime:E RustStdoutStderr:E > qa-captures/runtime-errors.txt
if grep -Eq 'FATAL EXCEPTION|panicked at' qa-captures/runtime-errors.txt;then cat qa-captures/runtime-errors.txt;exit 1;fi
