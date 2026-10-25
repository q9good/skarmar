"""Lesson UI/partition and retry tests against a disposable localhost-only PRD DB."""
import os, re
from pathlib import Path
from uuid import uuid4
from urllib.parse import urlparse
from playwright.sync_api import expect, sync_playwright
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/prd-lessons-ui-screenshots');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
    browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
    first=browser.new_context(viewport={'width':360,'height':740});second=browser.new_context(viewport={'width':430,'height':932})
    a,b=first.new_page(),second.new_page();errors=[]
    for page in [a,b]:page.on('pageerror',lambda e:errors.append(str(e)))
    def button(page,name):return page.get_by_role('button',name=name,exact=True)
    def login(page,user,password):
        page.goto(BASE);page.get_by_role('textbox',name='账号',exact=True).fill(user);page.get_by_label('密码',exact=True).fill(password);button(page,'登录').click();expect(button(page,'记录上课')).to_be_visible()
    def call(method,path,body=None,status=200):
        r=first.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'});assert r.status==status,r.text();return r.json()
    def state():return call('GET','/prd/state')
    def nav(page,name):button(page,name).click()
    def dismiss(page,title):button(page,'关闭'+title).click();expect(page.get_by_role('dialog')).to_have_count(0)
    def lesson_detail(page):button(page,re.compile(r'^查看课记：')).first.click();expect(page.get_by_role('dialog')).to_have_count(1)
    def submit(page,title='提交记录'):
        button(page,title).click();expect(page.get_by_role('dialog')).to_have_count(0)
        if title=='提交记录':expect(button(page,'检查连接并继续上传')).to_have_count(0,timeout=20000)
    login(a,'owner','synthetic-prd-ui-password');assert not state()['lessons'],'Use a fresh disposable database'
    member=call('POST','/auth/users',{'username':'family','display_name':'测试家人','password':'s','role':'member'})
    # Seed only unrelated library objects through the real authenticated contract.
    ids={}
    for kind,title in [('goal','测试自己穿鞋'),('method','测试等待示范')]:
        id=str(uuid4());ids[kind]=id
        value={'title':title,'level':'short','meaning':'','criteria':'','template':'text','parent_id':None,'state':'active','archived':False,'frequency':None,'attention_start':None} if kind=='goal' else {'title':title,'goal_ids':[ids['goal']],'description':'','steps':'','guidance':'','state':'active','pinned':False,'needs_attention':False,'attention_reason':''}
        call('PUT','/prd/records/'+id,{'id':id,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value}})
    a.reload();expect(button(a,'记录上课')).to_be_visible()
    nav(a,'记录上课');expect(a.get_by_role('textbox',name='今天课上怎么样？',exact=True)).to_have_count(0)
    nav(a,'课程名称（必填）：请选择课程');a.get_by_role('textbox',name='新增课程名称',exact=True).fill('测试语言课');nav(a,'保存并选用')
    expect(a.get_by_role('dialog')).to_have_count(1);expect(button(a,'授课老师（必填）：请选择老师')).to_be_visible()
    nav(a,'授课老师（必填）：请选择老师');a.get_by_role('textbox',name='新增老师名称',exact=True).fill('测试王老师');nav(a,'保存并选用')
    expect(a.get_by_role('dialog')).to_have_count(1)
    a.screenshot(path=str(OUT/'basic-lesson-mobile.png'));submit(a)
    assert len(state()['lessons'])==1;l=state()['lessons'][0];lesson_id=l['id'];today=l['observation']['business_date']
    nav(a,'补充课堂观察');field=a.get_by_role('textbox',name='今天课上怎么样？',exact=True);field.fill('测试：玩轮流游戏')
    nav(a,'老师的做法和建议');a.get_by_role('textbox',name='老师的做法和建议',exact=True).fill('测试：老师建议先等待，再示范')
    nav(a,'老师的做法和建议 · 已写');expect(a.get_by_role('textbox',name='老师的做法和建议',exact=True)).to_have_count(0)
    nav(a,'老师的做法和建议 · 已写');expect(a.get_by_role('textbox',name='老师的做法和建议',exact=True)).to_have_value('测试：老师建议先等待，再示范')
    a.get_by_role('checkbox',name='请爸爸妈妈看看',exact=True).click();submit(a)
    assert len(state()['lessons'])==1 and state()['lessons'][0]['observation_version']==2 and not state()['actions']
    print('PASS basic record, shared options created and independently selected; optional observations preserve text and do not add lessons/actions',flush=True)
    # Nested selector Back closes the selector only and preserves the observation draft.
    lesson_detail(a);nav(a,'补充／修改课堂观察');nav(a,'课程名称（必填）：测试语言课');a.go_back();expect(a.get_by_role('dialog')).to_have_count(1)
    expect(a.get_by_role('textbox',name='今天课上怎么样？',exact=True)).to_have_value('测试：玩轮流游戏');dismiss(a,'记录上课')
    login(b,'family','s')
    lesson_detail(b);nav(b,'整理这条记录');b.get_by_role('checkbox',name='测试自己穿鞋',exact=True).click();b.get_by_role('checkbox',name='实际方法：测试等待示范',exact=True).click()
    # Cross-partition update: observation changes while another member is organizing old observation.
    lesson_detail(a);nav(a,'补充／修改课堂观察');a.get_by_role('textbox',name='今天课上怎么样？',exact=True).fill('测试：后来补充观察');submit(a)
    nav(b,'记一条困难');b.get_by_role('textbox',name='困难内容 1',exact=True).fill('测试：等待时需要支持')
    nav(b,'记一条经验');b.get_by_role('textbox',name='经验内容 2',exact=True).fill('测试：轮流前先示范');b.get_by_role('combobox',name='经验阶段 2',exact=True).select_option('try')
    nav(b,'写下下一步');b.get_by_role('textbox',name='下一步 1',exact=True).fill('测试：下次请老师示范')
    nav(b,'调整下一步关联 1');b.go_back();expect(b.get_by_role('dialog')).to_have_count(1)
    b.get_by_role('combobox',name='这条记录如何处理',exact=True).select_option('complete');submit(b,'保存整理')
    s=state();assert len(s['knowledge'])==2 and len(s['actions'])==1
    assert all(k['source_observation_version']==2 and k['source_observation']['notes']=='测试：玩轮流游戏' for k in s['knowledge'])
    assert s['lessons'][0]['observation']['notes']=='测试：后来补充观察'
    print('PASS independent observation/organization saves, explicit linked experience/difficulty/action and immutable source observation',flush=True)
    nav(b,'目标');expect(b.get_by_text('关联困难：测试：等待时需要支持',exact=True)).to_be_visible();expect(b.get_by_text('关联经验：测试：轮流前先示范',exact=True)).to_be_visible()
    nav(b,'方法');expect(b.get_by_text('关联困难：测试：等待时需要支持',exact=True)).to_be_visible()
    nav(b,'跟进');nav(b,'经验库与困难');expect(b.get_by_text('测试：等待时需要支持',exact=True)).to_be_visible()
    nav(b,'今天');lesson_detail(b);nav(b,'查看关联目标：测试自己穿鞋');expect(b.get_by_role('dialog')).to_have_count(1);expect(b.get_by_role('dialog').get_by_text('测试自己穿鞋',exact=True).first).to_be_visible()
    b.go_back();expect(b.get_by_role('dialog')).to_have_count(0)
    print('PASS goal/method/followup links and replacement detail Back behavior',flush=True)
    # Same-partition concurrency preserves unsaved input and requires explicit reconciliation.
    a.reload();lesson_detail(a);nav(a,'补充／修改课堂观察');a.get_by_role('textbox',name='今天课上怎么样？',exact=True).fill('测试：未提交的观察')
    lesson_detail(b);nav(b,'补充／修改课堂观察');b.get_by_role('textbox',name='今天课上怎么样？',exact=True).fill('测试：家人新观察');submit(b)
    a.evaluate("window.dispatchEvent(new Event('focus'))")
    expect(a.get_by_text('这一部分已有新版本，当前填写仍保留。',exact=True)).to_be_visible();expect(button(a,'提交记录')).to_be_disabled()
    expect(a.get_by_role('textbox',name='今天课上怎么样？',exact=True)).to_have_value('测试：未提交的观察')
    nav(a,'备份填写并载入最新课记');expect(a.get_by_role('textbox',name='今天课上怎么样？',exact=True)).to_have_value('测试：家人新观察')
    expect(a.get_by_text('测试：未提交的观察',exact=True)).to_be_visible();submit(a)
    print('PASS same-partition conflict preserves text, presents latest and saves only after explicit reconciliation',flush=True)
    # Commit with lost response; pending op is restored after reload and retried without duplication.
    nav(a,'记录上课');nav(a,'课程名称（必填）：请选择课程');button(a.get_by_role('dialog',name='选择课程',exact=True),'测试语言课').click();nav(a,'授课老师（必填）：请选择老师');button(a.get_by_role('dialog',name='选择老师',exact=True),'测试王老师').click()
    def lose_response(route):
        response=route.fetch();assert response.ok;route.abort('failed')
    a.route('**/api/prd/queue',lose_response,times=1);nav(a,'提交记录');expect(a.get_by_role('dialog')).to_have_count(0);expect(button(a,'检查连接并继续上传')).to_be_visible()
    a.wait_for_function("Object.values(JSON.parse(localStorage.getItem(Object.keys(localStorage).find(k=>k.endsWith('lesson-queue:v1')))).entries).some(e=>e.status==='uncertain'&&e.reason.includes('连接'))")
    assert len(state()['lessons'])==2
    a.reload();expect(button(a,'记录上课')).to_be_enabled();expect(button(a,'检查连接并继续上传')).to_have_count(0,timeout=20000);assert len(state()['lessons'])==2
    second_id=next(l['id'] for l in state()['lessons'] if l['id']!=lesson_id)
    assert len(call('GET','/prd/lessons/'+second_id+'/history'))==1
    print('PASS lost response reload/retry creates exactly one lesson/revision; real second lesson is allowed',flush=True)
    # Counting, error flag and protected withdrawal/deletion are exercised via UI.
    nav(a,'记录');nav(a,'课时核对');nav(a,'下个月');nav(a,'测试王老师：2 节');expect(a.get_by_role('button',name=re.compile(r'^查看课记：'))).to_have_count(2)
    nav(a,'课堂记录');lesson_detail(a);nav(a,'标记记错，请管理员核对');submit(a,'确认操作')
    nav(a,'课时核对');expect(button(a,'测试王老师：2 节 · 待核对 1')).to_be_visible()
    nav(b,'记录');nav(b,'课时核对');nav(b,'下个月');nav(b,'测试王老师：2 节 · 待核对 1');lesson_detail(b)
    assert button(b,'删除课记（管理员）').count()==0 and button(b,'撤销这条课记').count()==0;dismiss(b,'课记详情')
    # Creator can withdraw the unreferenced second record, while original source is protected.
    nav(a,'课堂记录');buttons=a.get_by_role('button',name=re.compile(r'^查看课记：'));buttons.first.click();nav(a,'撤销这条课记');submit(a,'确认操作')
    assert next(l for l in state()['lessons'] if l['id']==second_id)['status']=='withdrawn'
    nav(a,'课堂记录');lesson_detail(a);nav(a,'查看课记修改记录');expect(a.get_by_text('观察第 1 版 · 整理第 1 版',exact=True)).to_be_visible();nav(a,'删除课记（管理员）');nav(a,'确认操作');expect(a.get_by_role('alert')).to_be_visible();assert next(l for l in state()['lessons'] if l['id']==lesson_id)['status']=='recorded'
    dismiss(a,'核对课记操作')
    print('PASS month totals, duplicate lessons, marked errors still counted, creator withdrawal, member/admin permissions and source protection',flush=True)
    for page in [a,b]:
        if page.get_by_role('dialog').count():raise AssertionError('Unexpected overlay')
        page.set_viewport_size({'width':320,'height':740});assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
    a.screenshot(path=str(OUT/'lesson-records-mobile.png'));assert not errors,errors
    print('PASS 320px layout and no browser runtime errors',flush=True);browser.close()
