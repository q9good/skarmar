"""User-reported draft leakage, uploads, hierarchy and settlement in a fresh local DB."""
import os, re, calendar
from datetime import date, timedelta
from pathlib import Path
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import expect, sync_playwright

BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/prd-feedback-ui');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
    browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
    ctx=browser.new_context(viewport={'width':390,'height':844});page=ctx.new_page();errors=[]
    page.on('pageerror',lambda e:errors.append(str(e)))
    def btn(name,scope=page):return scope.get_by_role('button',name=name,exact=True)
    def click(name,scope=page):btn(name,scope).click()
    def field(name):return page.get_by_role('textbox',name=name,exact=True)
    def call(method,path,body=None):
        r=ctx.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'})
        assert r.ok,(r.status,r.text());return r.json()
    def state():return call('GET','/prd/state')
    def entity(kind,value):
        id=str(uuid4());body={'id':id,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value}}
        if kind=='method':body['method_create_mode']='full'
        return call('PUT','/prd/records/'+id,body)['value']
    def shot(name):
        page.evaluate("()=>{for(const e of document.querySelectorAll('*'))if(['auto','scroll'].includes(getComputedStyle(e).overflowY))e.scrollTop=0;}")
        page.screenshot(path=str(OUT/(name+'.png')))
    page.goto(BASE);field('账号').fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录');expect(btn('记录课堂')).to_be_enabled()
    today=page.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date())")
    long=entity('goal',{'title':'合成交往方向','level':'long'})
    entity('goal',{'title':'合成学习方向','level':'long'});entity('goal',{'title':'合成生活方向','level':'long'})
    medium=entity('goal',{'title':'合成轮流目标','level':'medium','parent_id':long['id']})
    short=entity('goal',{'title':'合成轮流滚球','level':'short','parent_id':medium['id']})
    shared=entity('method',{'title':'合成等待回应','steps':'示范并等待回应','goal_ids':[long['id'],short['id']]})
    own=entity('method',{'title':'合成小步示范','steps':'先示范一个步骤','goal_ids':[medium['id']]})
    c1=entity('catalog',{'kind':'course','name':'合成认知课','active':True});c2=entity('catalog',{'kind':'course','name':'合成社交课','active':True})
    t1=entity('catalog',{'kind':'teacher','name':'合成甲老师','active':True});t2=entity('catalog',{'kind':'teacher','name':'合成乙老师','active':True})
    last=date.fromisoformat(today).replace(day=1)-timedelta(days=1)
    def lesson(day,course,teacher):
        id=str(uuid4());return call('PUT','/prd/lessons/'+id,{'id':id,'operation_id':str(uuid4()),'expected_version':None,'action':{'kind':'observation','value':{'business_date':day,'course_id':course['id'],'teacher_id':teacher['id']}}})
    for c,t in [(c1,t1),(c1,t2),(c2,t1)]:lesson(last.isoformat(),c,t)
    page.reload();expect(btn('记录课堂')).to_be_enabled()
    card=btn('查看长期关注');expect(card).to_contain_text('合成交往方向');expect(card).to_contain_text('合成学习方向');expect(card).to_contain_text('合成生活方向')
    expect(btn('使用说明与手机桌面入口')).to_have_count(0);shot('today-directions')

    click('我的账号');expect(btn('使用说明与手机桌面入口')).to_have_text('?');click('使用说明与手机桌面入口');expect(page.get_by_text('怎样理解方法的证据',exact=True)).to_be_visible();click('关闭使用与方法说明')
    click('维护领域选项');click('新增领域');expect(btn('保存')).to_be_disabled()
    field('领域名称').fill('测'*121);click('保存');expect(page.get_by_role('alert')).to_contain_text('名称须为1至120个字符')
    click('关闭维护选项');click('今天')
    expect(page.get_by_role('alert')).to_have_count(0);expect(btn(re.compile('^继续填写：'))).to_have_count(0)
    expect(btn('记一个瞬间')).to_be_visible();click('目标');expect(page.get_by_role('alert')).to_have_count(0)
    expect(btn(re.compile('^继续填写：'))).to_have_count(0)
    expect(field('搜索目标')).to_have_count(0);click('筛选目标');expect(field('搜索目标')).to_be_visible();field('搜索目标').fill('滚球');click('筛选目标')
    expect(btn('查看目标：合成轮流滚球')).to_be_visible();expect(btn('查看目标：合成交往方向')).to_have_count(0)
    click('筛选目标');field('搜索目标').fill('');click('筛选目标')
    click('方法');expect(field('搜索方法')).to_have_count(0);click('方法列表设置');expect(field('搜索方法')).to_be_visible();click('方法列表设置');expect(btn('使用说明与手机桌面入口')).to_have_count(0)

    click('我的账号');click('本机未完成填写');click('继续填写：'+'测'*121);expect(field('领域名称')).to_have_value('测'*121)
    click('放弃这份填写');click('确认放弃这份填写');expect(page.get_by_role('dialog')).to_have_count(0)
    click('维护领域选项');click('新增领域');click('保留草稿并退出');expect(btn('继续填写：领域 · 未填写名称')).to_be_visible()
    click('今天');expect(btn('继续填写：领域 · 未填写名称')).to_have_count(0)
    click('我的账号');click('本机未完成填写');click('继续填写：领域 · 未填写名称');expect(btn('保存')).to_be_disabled()
    click('放弃这份填写');click('继续填写');expect(field('领域名称')).to_be_visible();click('放弃这份填写');click('确认放弃这份填写');click('返回')
    page.reload();expect(btn('记录课堂')).to_be_enabled();click('目标');expect(btn('继续填写：领域 · 未填写名称')).to_have_count(0)
    click('新建目标');field('本领名称').fill('合成未提交目标');click('保留草稿并退出');expect(btn('继续填写：合成未提交目标')).to_be_visible()
    click('今天');expect(btn('继续填写：合成未提交目标')).to_have_count(0);click('目标');click('继续填写：合成未提交目标');expect(field('本领名称')).to_have_value('合成未提交目标')
    click('放弃这份填写');click('确认放弃这份填写');assert not any(g['input']['title']=='合成未提交目标' for g in state()['goals'])
    click('今天');click('记一个瞬间');click('保存观察');expect(page.get_by_role('alert')).to_be_visible();click('关闭填写瞬间观察')
    click('目标');expect(page.get_by_role('alert')).to_have_count(0);expect(btn(re.compile('^继续观察填写：'))).to_have_count(0)
    print('PASS named/blank drafts are recoverable in their own location; explicit discard/cancel works; validation never leaks into Today/Goals; help and search are tucked away',flush=True)

    click('跟进');expect(btn('新建经验')).to_have_count(0);expect(btn('新建困难')).to_have_count(0)
    click('经验库与困难');click('新建经验');field('经验名称（必填）').fill('合成分层经验');field('做法与依据').fill('熟悉游戏先示范并等待')
    click('选择关联目标与方法');expect(page.get_by_role('checkbox',name='关联目标：合成轮流滚球',exact=True)).to_have_count(0)
    page.get_by_role('checkbox',name='关联目标：合成交往方向',exact=True).click();click('展开关联层级：合成交往方向');click('展开关联层级：合成轮流目标');click('展开关联层级：合成轮流滚球')
    page.get_by_role('checkbox',name='关联目标：合成轮流滚球',exact=True).click()
    checks=page.get_by_role('checkbox',name='关联方法：合成等待回应',exact=True);expect(checks).to_have_count(2);checks.first.click();expect(checks.first).to_be_checked();expect(checks.last).to_be_checked()
    checks.last.click();expect(checks.first).not_to_be_checked();checks.first.click();shot('followup-hierarchy')
    click('关闭关联目标与方法');page.get_by_role('combobox',name='经验阶段',exact=True).select_option('idea');click('保存跟进');expect(page.get_by_role('dialog')).to_have_count(0)
    k=next(k for k in state()['knowledge'] if k['details']['title']=='合成分层经验')
    assert set(k['input']['goal_ids'])=={long['id'],short['id']} and k['input']['method_ids']==[shared['id']]
    click('困难');click('新建困难');field('困难名称（必填）').fill('合成分层困难');field('困难描述').fill('换材料后需要再示范');click('选择关联目标与方法')
    click('展开关联层级：合成交往方向');page.get_by_role('checkbox',name='关联目标：合成轮流目标',exact=True).click();click('展开关联层级：合成轮流目标');page.get_by_role('checkbox',name='关联方法：合成小步示范',exact=True).click()
    click('关闭关联目标与方法');click('保存跟进');expect(page.get_by_role('dialog')).to_have_count(0)
    k=next(k for k in state()['knowledge'] if k['details']['title']=='合成分层困难');assert k['input']['goal_ids']==[medium['id']] and k['input']['method_ids']==[own['id']]
    click('要处理的');expect(btn('新建经验')).to_have_count(0);expect(btn('新建困难')).to_have_count(0)
    print('PASS library-only creation, parent/child goal choices and shared-method multi-selection persist exact independent bindings for both experiences and difficulties',flush=True)

    click('记录');click('课时核对');expect(btn('合成甲老师：2 节')).to_be_visible();expect(btn('合成乙老师：1 节')).to_be_visible()
    click('合成甲老师：2 节');expect(btn(re.compile('^查看课记：'))).to_have_count(2)
    click('按课程汇总');expect(btn('合成认知课：2 节')).to_be_visible();expect(btn('合成社交课：1 节')).to_be_visible();expect(btn(re.compile('^查看课记：'))).to_have_count(0)
    click('合成认知课：2 节');expect(btn(re.compile('^查看课记：'))).to_have_count(2);click('下个月');expect(btn(re.compile('^查看课记：'))).to_have_count(0)
    click('按老师汇总');expect(page.get_by_text('这个月还没有上课记录。',exact=True)).to_be_visible()
    print('PASS teacher default/course toggle uses previous calendar month, matching counts and independent drill-downs without stale selection across months',flush=True)

    click('今天');click('记录课堂')
    click('课程名称（必填）：请选择课程');click('合成认知课',page.get_by_role('dialog',name='选择课程',exact=True))
    click('授课老师（必填）：请选择老师');click('合成甲老师',page.get_by_role('dialog',name='选择老师',exact=True))
    held=[]
    def hold_receipt(route):
        response=route.fetch();assert response.ok,response.text();held.append((route,response))
    page.route('**/api/prd/queue',hold_receipt,times=1)
    click('提交记录');expect(page.get_by_role('dialog')).to_have_count(0)
    expect(page.get_by_text('正在上传课记，服务器确认后才计节数…',exact=True)).to_be_visible()
    page.wait_for_function("()=>Object.keys(localStorage).some(k=>k.endsWith('lesson-queue:v1')&&Object.values(JSON.parse(localStorage.getItem(k)).entries).some(e=>e.status==='uncertain'))")
    for _ in range(100):
        if held:break
        page.wait_for_timeout(50)
    assert held and len(state()['lessons'])==4
    expect(btn('补充课堂观察')).to_have_count(0)
    route,response=held[0];route.fulfill(response=response)
    expect(btn('补充课堂观察')).to_be_visible();expect(page.get_by_text(re.compile('^待上传课记与选项：'))).to_have_count(0)
    latest=next(l for l in state()['lessons'] if l['observation']['business_date']==today)
    assert len(call('GET','/prd/lessons/'+latest['id']+'/history'))==1
    other=ctx.new_page();other.goto(BASE);expect(btn('记录课堂',other)).to_be_enabled()
    expect(other.get_by_text(re.compile('^合成认知课 · 合成甲老师')).first).to_be_visible()
    assert not errors,errors
    print('PASS submit starts a real upload immediately, waits for authoritative receipt before success, creates exactly one server history entry and is visible in another browser page',flush=True)
    browser.close()
