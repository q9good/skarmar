"""Real-browser offline recovery, dependency, identity and cross-tab checks; synthetic localhost only."""
import os,re,json
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import sync_playwright,expect
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 ctx=browser.new_context(viewport={'width':360,'height':740});page=ctx.new_page();errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 def button(name):return page.get_by_role('button',name=name,exact=True)
 def click(name):button(name).click()
 def login(user='owner',password='synthetic-prd-ui-password'):
  page.goto(BASE);page.get_by_role('textbox',name='账号',exact=True).fill(user);page.get_by_label('密码',exact=True).fill(password);click('登录');expect(button('记录上课')).to_be_enabled()
 def call(method,path,body=None):
  r=ctx.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'});assert r.ok,r.text();return r.json()
 def state():return call('GET','/prd/state')
 def wait_count(n):
  for _ in range(100):
   if len(state()['lessons'])==n:return
   page.wait_for_timeout(100)
  raise AssertionError(f'Expected {n} lessons, got '+str(len(state()['lessons'])))
 def key(suffix):return page.evaluate("suffix=>Object.keys(localStorage).find(k=>k.endsWith(suffix))",suffix)
 def readqueue():return page.evaluate("JSON.parse(localStorage.getItem(Object.keys(localStorage).find(k=>k.endsWith('lesson-queue:v1'))))")
 def create_options():
  for kind,label,name in [('course','课程','离线课程'),('teacher','老师','离线老师')]:
   click(('课程名称' if kind=='course' else '授课老师')+'（必填）：请选择'+label)
   page.get_by_role('textbox',name='新增'+label+'名称',exact=True).fill(name);click('保存并选用');expect(page.get_by_role('dialog')).to_have_count(1)
 def new_lesson():
  click('记录上课')
  for prefix,label,name in [('课程名称','课程','离线课程'),('授课老师','老师','离线老师')]:
   click(prefix+'（必填）：请选择'+label);page.get_by_role('dialog',name='选择'+label,exact=True).get_by_role('button',name=name,exact=True).click()
 def submit():click('提交记录');expect(page.get_by_role('dialog')).to_have_count(0)
 login();owner=call('GET','/auth/me');member=call('POST','/auth/users',{'username':'family','display_name':'测试家人','password':'s','role':'member'})
 # Cancelling an untouched lesson is local-only and removes its two orphaned new options.
 ctx.set_offline(True);click('记录上课');create_options();click('补充观察');page.get_by_role('textbox',name='今天课上怎么样？',exact=True).fill('取消前的本机观察');submit()
 button(re.compile('^取消待上传：课记：')).click();expect(page.get_by_role('dialog',name='取消待上传内容',exact=True)).to_be_visible();click('确认取消待上传内容')
 expect(page.get_by_text('已取消本机待上传课记，并清理 2 个不再使用的新增选项。',exact=True)).to_be_visible();assert readqueue()['entries']=={};assert not state()['lessons'] and not state()['catalog'];click('关闭取消提示')
 print('PASS untouched queued lesson cancellation removes orphan catalogs without server writes',flush=True)
 ctx.set_offline(True);click('记录上课');create_options();click('补充观察');page.get_by_role('textbox',name='今天课上怎么样？',exact=True).fill('离线观察：轮流游戏');submit()
 expect(page.get_by_text('待上传课记与选项：3',exact=True)).to_be_visible();pending=readqueue();lesson_id=next(i for i,e in pending['entries'].items() if e['write']['kind']=='lesson');operation=pending['entries'][lesson_id]['write']['value']['operation_id']
 click('记录');click('课时核对');click('下个月');expect(page.get_by_text('这个月还没有上课记录。',exact=True)).to_be_visible()
 click('目标');click('今天');expect(page.get_by_text('待上传课记与选项：3',exact=True)).to_be_visible()
 ctx.set_offline(False);expect(page.get_by_text(re.compile('^待上传课记与选项：'))).to_have_count(0,timeout=20000)
 s=state();assert len(s['lessons'])==1 and len(s['catalog'])==2;assert s['lessons'][0]['observation']['notes']=='离线观察：轮流游戏';assert len(call('GET','/prd/lessons/'+lesson_id+'/history'))==1
 print('PASS offline new course/teacher dependencies, queued observation survives tabs, excluded from totals until server confirmed',flush=True)
 # Close/reopen while offline is not a promise to bypass authentication. Reopen online resumes the durable operation.
 ctx.set_offline(True);new_lesson();submit();q=readqueue();new_id=next(i for i,e in q['entries'].items() if e['status']!='confirmed');op=q['entries'][new_id]['write']['value']['operation_id'];ctx.set_offline(False)
 page.reload();expect(button('记录上课')).to_be_enabled();expect(page.get_by_text(re.compile('^待上传课记与选项：'))).to_have_count(0,timeout=20000)
 assert len(state()['lessons'])==2;assert readqueue()['entries'][new_id]['write']['value']['operation_id']==op
 print('PASS page reload online automatically resumes original operation once',flush=True)
 # A server commit with a dropped response must be retrieved with the same operation.
 new_lesson()
 def lose(route):
  response=route.fetch();assert response.ok;route.abort('failed')
 page.route('**/api/prd/queue',lose,times=1);submit();expect(page.get_by_text(re.compile('^待上传课记与选项：'))).to_be_visible();wait_count(3)
 page.wait_for_function("Object.values(JSON.parse(localStorage.getItem(Object.keys(localStorage).find(k=>k.endsWith('lesson-queue:v1')))).entries).some(e=>e.status==='uncertain'&&e.reason.includes('连接'))")
 expect(button(re.compile('^取消待上传'))).to_have_count(0)
 click('检查连接并继续上传');expect(page.get_by_text(re.compile('^待上传课记与选项：'))).to_have_count(0,timeout=20000);assert len(state()['lessons'])==3
 print('PASS response loss retrieves confirmed receipt without new lesson',flush=True)
 # Inactive option detected at reconnect pauses, retaining exact input. Re-enable then explicitly resume.
 ctx.set_offline(True);new_lesson();submit();q=readqueue();blocked_id=next(i for i,e in q['entries'].items() if e['status']!='confirmed')
 teacher=next(c for c in state()['catalog'] if c['input']['kind']=='teacher')
 def active(value,version):return call('PUT','/prd/records/'+teacher['id'],{'id':teacher['id'],'operation_id':str(uuid4()),'expected_version':version,'change_note':'测试选项启停','mutation':{'kind':'catalog','value':{**teacher['input'],'active':value}}})
 active(False,teacher['version']);ctx.set_offline(False);expect(page.get_by_text('课程或老师尚未创建、已停用或需要核对；原课记保留，暂不计节数。',exact=True)).to_be_visible();assert len(state()['lessons'])==3
 active(True,teacher['version']+1);click('检查连接并继续上传');expect(button('核对后继续上传：'+blocked_id[:8])).to_be_visible();click('核对后继续上传：'+blocked_id[:8]);expect(page.get_by_text(re.compile('^待上传课记与选项：'))).to_have_count(0,timeout=20000);assert len(state()['lessons'])==4
 print('PASS inactive catalog pauses without data loss or counts, explicit resume after correction',flush=True)
 # Replace cookie from outside this page, emulating another browser tab changing account.
 ctx.set_offline(True);new_lesson();submit()
 call('POST','/auth/logout');call('POST','/auth/login',{'username':'family','password':'s'});ctx.set_offline(False);expect(page.get_by_text('当前账号或空间已变化，请登录原账号后核对并继续。',exact=True)).to_be_visible();assert len(state()['lessons'])==4
 page.reload();expect(button('记录上课')).to_be_enabled();expect(page.get_by_text(re.compile('^待上传课记与选项：'))).to_have_count(0);expect(button(re.compile('^取消待上传'))).to_have_count(0);assert len(state()['lessons'])==4
 call('POST','/auth/logout');call('POST','/auth/login',{'username':'owner','password':'synthetic-prd-ui-password'});page.reload();expect(button('记录上课')).to_be_enabled();expect(button(re.compile('^核对后继续上传：'))).to_be_visible();button(re.compile('^核对后继续上传：')).click();expect(page.get_by_text(re.compile('^待上传课记与选项：'))).to_have_count(0,timeout=20000);assert len(state()['lessons'])==5
 print('PASS cross-tab account change cannot replay another actor queue; original actor explicitly resumes',flush=True)
 # Two tabs add separate offline records into one locked durable queue.
 other=ctx.new_page();other.goto(BASE);expect(other.get_by_role('button',name='记录上课',exact=True)).to_be_enabled();ctx.set_offline(True)
 for current in [page,other]:
  current.get_by_role('button',name='记录上课',exact=True).click()
  for prefix,label,name in [('课程名称','课程','离线课程'),('授课老师','老师','离线老师')]:
   current.get_by_role('button',name=prefix+'（必填）：请选择'+label,exact=True).click();current.get_by_role('dialog',name='选择'+label,exact=True).get_by_role('button',name=name,exact=True).click()
  current.get_by_role('button',name='提交记录',exact=True).click();expect(current.get_by_role('dialog')).to_have_count(0)
 ctx.set_offline(False)
 for current in [page,other]:current.evaluate("window.dispatchEvent(new Event('online'))")
 expect(page.get_by_text(re.compile('^待上传课记与选项：'))).to_have_count(0,timeout=20000);assert len(state()['lessons'])==7
 print('PASS multiple real browser tabs retain both queued lessons and replay each exactly once',flush=True)
 # A duplicate offline catalog is explicitly reconciled to the existing stable ID.
 ctx.set_offline(True);click('记录上课');click('课程名称（必填）：请选择课程');page.get_by_role('textbox',name='新增课程名称',exact=True).fill('离线课程');click('保存并选用');expect(page.get_by_role('dialog')).to_have_count(1);click('授课老师（必填）：请选择老师');page.get_by_role('dialog',name='选择老师',exact=True).get_by_role('button',name='离线老师',exact=True).click();submit();ctx.set_offline(False)
 expect(button('改用已有选项：离线课程')).to_be_visible(timeout=20000);click('改用已有选项：离线课程');click('核对并改用：离线课程');expect(page.get_by_role('dialog')).to_have_count(0);wait_count(8);assert len(state()['catalog'])==2;assert any(e['status']=='linked' for e in readqueue()['entries'].values())
 page.reload();expect(button('记录上课')).to_be_enabled();expect(page.get_by_text(re.compile('^待上传课记与选项：'))).to_have_count(0);assert len(state()['lessons'])==8
 print('PASS duplicate offline course explicitly links existing catalog, resumes only unsent lesson and survives reload without duplicate options or lessons',flush=True)
 page.set_viewport_size({'width':320,'height':740});assert not page.evaluate('document.documentElement.scrollWidth > innerWidth');assert not errors,errors
 browser.close()
