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
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -W -n io.github.weimanmk.easygot.preview/io.github.weimanmk.easygot.MainActivity
sleep 20
adb shell pidof io.github.weimanmk.easygot.preview
adb exec-out screencap -p > qa-captures/home.png
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
screen paper
adb exec-out screencap -p > qa-captures/paper.png
PDF_PATH=$(adb exec-out run-as io.github.weimanmk.easygot.preview cat files/qa-paper-path.txt)
adb exec-out run-as io.github.weimanmk.easygot.preview cat "$PDF_PATH" > qa-captures/sample-paper.pdf
adb exec-out run-as io.github.weimanmk.easygot.preview cat files/qa-print-spec.json > qa-captures/sample-print-spec.json
adb logcat -d -s AndroidRuntime:E RustStdoutStderr:E > qa-captures/runtime-errors.txt
if rg -q 'FATAL EXCEPTION|panicked at' qa-captures/runtime-errors.txt;then cat qa-captures/runtime-errors.txt;exit 1;fi
