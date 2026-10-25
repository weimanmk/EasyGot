#!/usr/bin/env python3
"""Real touch/navigation regression against the Slint Android APK.
Debug-only snapshots report widget rectangles and synthetic learning fixtures, never credentials.
"""
import json, subprocess, time, re
import xml.etree.ElementTree as ET
from pathlib import Path

import atexit
atexit.register(lambda: capture('flow-final-state'))

PACKAGE='io.github.weimanmk.easygot.preview'
COMPONENT=PACKAGE+'/io.github.weimanmk.easygot.MainActivity'
OUT=Path('qa-captures'); OUT.mkdir(exist_ok=True)
def adb(*args): return subprocess.check_output(['adb',*args],text=True,stderr=subprocess.DEVNULL)
def command(*args): subprocess.run(['adb',*args],check=True,stdout=subprocess.DEVNULL)
def route(name):
    command('shell','am','start','-W','-n',COMPONENT,'--es','qaScreen',name); time.sleep(1)
def read():
    try: return json.loads(adb('exec-out','run-as',PACKAGE,'cat','files/qa-state.json'))
    except (subprocess.CalledProcessError,ValueError): return {}
def snapshot():
    previous=read().get('sampledAt'); route('snapshot')
    for _ in range(30):
        value=read()
        if value.get('sampledAt') is not None and value['sampledAt']!=previous:
            (OUT/'state-latest.json').write_text(json.dumps(value,ensure_ascii=False,indent=2));return value
        time.sleep(.2)
    raise RuntimeError('Slint snapshot did not arrive')
def capture(name):
    with (OUT/(name+'.png')).open('wb') as file:subprocess.run(['adb','exec-out','screencap','-p'],stdout=file,check=True)
def tap(key):
    for _ in range(14):
        value=snapshot(); box=value['bounds'].get(key)
        assert box, (key,'control missing',list(value['bounds']))
        scale=value.get('scale',2); height=value['window']['height']/scale
        x=box['x']+box['width']/2; y=box['y']+box['height']/2
        top=value.get('viewport',{}).get('top',100);bottom=value.get('viewport',{}).get('bottom',height-130)
        if top+5 < box['y'] and box['y']+box['height'] < bottom-5:
            command('shell','input','tap',str(round(x*scale)),str(round(y*scale)));time.sleep(.5);return
        direction=1 if box['y']+box['height']>=bottom-5 else -1
        distance=min(200,max(60,(box['y']+box['height']-bottom+30) if direction==1 else (top-box['y']+30)))
        start=bottom-40 if direction==1 else top+40
        command('shell','input','swipe',str(round(175*scale)),str(round(start*scale)),str(round(175*scale)),str(round((start-direction*distance)*scale)),'800')
        time.sleep(1)
    raise AssertionError((key,'could not bring control into visible area'))

route('question'); value=snapshot(); session=value['session']; question=session['questions'][0]
answer=question['answer'][0];tap('answer-option:'+answer)
value=snapshot();assert value['session']['answers'][question['id']]['selected']==[answer]
tap('submit-one');value=snapshot();assert value['route']=='result',value
assert value['result']['score']==question['maxScore'],value['result']
capture('flow-result');tap('continue');tap('next');value=snapshot();session_id=value['session']['id'];assert value['session']['index']==1
command('shell','am','force-stop',PACKAGE);command('shell','am','start','-W','-n',COMPONENT);time.sleep(4)
tap('resume:'+session_id);value=snapshot();assert value['session']['index']==1
assert value['session']['answers'][question['id']]['selected']==[answer];capture('flow-restored')
command('shell','input','keyevent','KEYCODE_BACK');time.sleep(1);value=snapshot();assert value['route']=='',value['route']
command('shell','settings','put','secure','show_ime_with_hard_keyboard','1')
route('question')
for _ in range(3):tap('next')
tap('answer');command('shell','input','text','keyboard-answer');time.sleep(1)
value=snapshot();question_id=value['session']['questions'][3]['id']
assert value['session']['answers'][question_id]['text']=='keyboard-answer',value
capture('flow-answer-keyboard')
command('shell','input','keyevent','KEYCODE_BACK');time.sleep(1)
value=snapshot();assert value['route']=='question',('Back should dismiss IME first',value)
route('directory');tap('directory:paper-10');value=snapshot();assert value['selectedDirectory']=='paper-10'
capture('flow-selected-paper');tap('practice');value=snapshot();assert len(value['session']['questions'])==2,value
assert value['session']['questions'][0]['id']=='paper-10-q1';capture('flow-paper-practice')

# Test the actual settings buttons, not only the debug intent that prepares
# a synthetic clipboard. Never copy real credentials into QA snapshots/logs.
route('settings');tap('aiKey')
command('shell','input','keyevent','KEYCODE_MOVE_END')
command('shell','input','keyevent',*['KEYCODE_DEL']*16)
command('shell','input','keyevent','KEYCODE_BACK');time.sleep(1)
route('check-paste')
assert not json.loads(adb('exec-out','run-as',PACKAGE,'cat','files/qa-paste-result.json'))['passed'],'Test did not clear the existing synthetic key'
tap('paste:aiKey');route('check-paste')
assert json.loads(adb('exec-out','run-as',PACKAGE,'cat','files/qa-paste-result.json'))['passed'],'Actual paste button did not read the clipboard'
tap('native:aiKey');command('shell','input','keyevent','279');time.sleep(1)
command('shell','uiautomator','dump','/sdcard/easygot-flow-secret.xml')
command('pull','/sdcard/easygot-flow-secret.xml',str(OUT/'flow-secret-dialog.xml'))
dialog=ET.parse(OUT/'flow-secret-dialog.xml')
assert any(n.attrib.get('class')=='android.widget.EditText' and n.attrib.get('password')=='true' for n in dialog.iter('node'))
assert 'easygot-test-key' not in (OUT/'flow-secret-dialog.xml').read_text(),'Secret text exposed to accessibility/UI automation'
button=next(n for n in dialog.iter('node') if n.attrib.get('resource-id')=='android:id/button1')
x1,y1,x2,y2=map(int,re.findall(r'\d+',button.attrib['bounds']))
command('shell','input','tap',str((x1+x2)//2),str((y1+y2)//2));time.sleep(1)
route('check-paste')
assert json.loads(adb('exec-out','run-as',PACKAGE,'cat','files/qa-paste-result.json'))['passed'],'Actual native-input button did not confirm a pasted key'
tap('save-config');capture('flow-config-saved')
command('shell','am','force-stop',PACKAGE);command('shell','am','start','-W','-n',COMPONENT);time.sleep(4)
route('check-paste')
assert json.loads(adb('exec-out','run-as',PACKAGE,'cat','files/qa-paste-result.json'))['passed'],'Saved key did not survive cold start and Android Keystore decryption'

route('home');command('shell','settings','put','system','font_scale','1.3');time.sleep(2);capture('home-font130')
command('shell','wm','size','824x1832');time.sleep(3);value=snapshot();capture('home-412dp-font130')
assert value['window']=={'width':824,'height':1832},('Native surface did not resize',value['window'])
assert value['viewport']['bottom']>760,('Layout did not fill 412dp surface',value['viewport'])
command('shell','settings','put','system','font_scale','1.0');command('shell','wm','size','700x1536')
json.dump({'actualOptionTap':True,'objectiveScoreCorrect':True,'restartRestoredAnswers':True,'systemBackNavigated':True,'paper10SelectedByTouch':True,'paper10ScopeCorrect':True,'fontAndWidthCaptures':True,'subjectiveKeyboardInput':True,'actualClipboardButton':True,'actualNativeInputButton':True,'encryptedKeyRestoredAfterColdStart':True},(OUT/'flow-checks.json').open('w'),indent=2)

