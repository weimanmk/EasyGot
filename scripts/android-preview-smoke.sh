#!/usr/bin/env bash
set -euo pipefail
mkdir -p qa-captures
trap 'adb logcat -d > qa-captures/preview-logcat.txt || true' EXIT
adb logcat -c
adb install -r app/build/outputs/apk/preview/app-preview.apk
if adb exec-out run-as io.github.weimanmk.easygot.preview pwd > qa-captures/preview-run-as.txt 2>&1; then
  echo 'Published preview unexpectedly permits debug access'
  exit 1
fi
adb shell am start -W -n io.github.weimanmk.easygot.preview/io.github.weimanmk.easygot.MainActivity
sleep 20
adb shell pidof io.github.weimanmk.easygot.preview
adb exec-out screencap -p > qa-captures/preview-home.png
adb shell am start -W -n io.github.weimanmk.easygot.preview/io.github.weimanmk.easygot.MainActivity --es qaScreen native-paste
sleep 3
adb shell uiautomator dump /sdcard/preview-ui.xml
adb pull /sdcard/preview-ui.xml qa-captures/preview-ui.xml
python3 - <<'PY'
import xml.etree.ElementTree as E,json
root=E.parse('qa-captures/preview-ui.xml')
assert not any(n.attrib.get('class')=='android.widget.EditText' for n in root.iter('node')),'Test-only secret input was exposed in published preview'
json.dump({'debugAccessDisabled':True,'testIntentsDisabled':True},open('qa-captures/preview-checks.json','w'))
PY
adb logcat -d -s AndroidRuntime:E RustStdoutStderr:E > qa-captures/preview-runtime-errors.txt
if rg -q 'FATAL EXCEPTION|panicked at' qa-captures/preview-runtime-errors.txt;then cat qa-captures/preview-runtime-errors.txt;exit 1;fi
