#!/usr/bin/env python3
"""Real touch/navigation regression against the Slint Android APK.
Debug-only snapshots report widget rectangles and synthetic learning fixtures, never credentials.
"""
import json, subprocess, time
from pathlib import Path

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
        if value.get('sampledAt') is not None and value['sampledAt']!=previous:return value
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
        if 100 < y < height-110:
            command('shell','input','tap',str(round(x*scale)),str(round(y*scale)));time.sleep(.5);return
        direction=1 if y>=height-110 else -1
        command('shell','input','swipe',str(round(175*scale)),str(round((height-180 if direction==1 else 180)*scale)),str(round(175*scale)),str(round((180 if direction==1 else height-180)*scale)),'400')
        time.sleep(.3)
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
route('directory');tap('directory:paper-10');value=snapshot();assert value['selectedDirectory']=='paper-10'
capture('flow-selected-paper');tap('practice');value=snapshot();assert len(value['session']['questions'])==2
assert value['session']['questions'][0]['id']=='paper-10-q1';capture('flow-paper-practice')
route('home');command('shell','settings','put','system','font_scale','1.3');time.sleep(2);capture('home-font130')
command('shell','wm','size','824x1832');time.sleep(2);capture('home-412dp-font130')
command('shell','settings','put','system','font_scale','1.0');command('shell','wm','size','700x1536')
json.dump({'actualOptionTap':True,'objectiveScoreCorrect':True,'restartRestoredAnswers':True,'systemBackNavigated':True,'paper10SelectedByTouch':True,'paper10ScopeCorrect':True,'fontAndWidthCaptures':True},(OUT/'flow-checks.json').open('w'),indent=2)
