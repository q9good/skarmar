"""A stale form cannot submit or read as an account switched in another window."""
from prd_ui_helpers import reveal_control
import os,json
from urllib.parse import urlparse
from playwright.sync_api import sync_playwright,expect
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
with sync_playwright() as p:
 b=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox']);ctx=b.new_context(viewport={'width':360,'height':740});page=ctx.new_page();errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 def btn(n):return page.get_by_role('button',name=n,exact=True)
 def call(path,body):
  r=ctx.request.post(BASE+'/api'+path,data=body,headers={'X-Skarma-Request':'1'});assert r.ok,r.text();return r.json()
 page.goto(BASE);reveal_control(page,'textbox','账号').fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');btn('登录').click();expect(btn('记录上课')).to_be_enabled();owner=ctx.request.get(BASE+'/api/auth/me').json()['user']['id'];call('/auth/users',{'username':'other','display_name':'另一位家人','password':'s','role':'member'})
 btn('目标').click();btn('新建目标').click();reveal_control(page,'textbox','本领名称').fill('原账号未提交填写');expect(page.get_by_text('草稿已存本机',exact=True)).to_be_visible()
 # Emulate a cookie change in another tab while this page has not polled yet.
 call('/auth/login',{'username':'other','password':'s'});btn('保存').click();expect(page.get_by_role('textbox',name='账号',exact=True)).to_be_visible();state=ctx.request.get(BASE+'/api/prd/state').json();assert not state['goals'],state
 reveal_control(page,'textbox','账号').fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');btn('登录').click();btn('目标').click();expect(btn('继续填写：原账号未提交填写')).to_be_visible();btn('继续填写：原账号未提交填写').click();expect(page.get_by_role('textbox',name='本领名称',exact=True)).to_have_value('原账号未提交填写');btn('保存').click();expect(page.get_by_role('dialog',name='填写目标',exact=True)).to_have_count(0);state=ctx.request.get(BASE+'/api/prd/state').json();assert len(state['goals'])==1
 gid=state['goals'][0]['id'];h=ctx.request.get(BASE+'/api/prd/records/'+gid+'/history').json();assert h[0]['actor_id']==owner
 # A committed request with a lost response must remain exactly retryable across
 # a rejected cookie-switched retry and reauthentication of its original owner.
 btn('目标').click();btn('新建目标').click();reveal_control(page,'textbox','本领名称').fill('已提交响应丢失的目标')
 def lose(route):
  response=route.fetch();assert response.ok,response.text();route.abort('failed')
 page.route('**/api/prd/records/*',lose,times=1);btn('保存').click();expect(btn('重试同一次保存')).to_be_enabled()
 def pending():
  return page.evaluate("""owner=>{const k=Object.keys(localStorage).find(k=>k.includes(':'+owner+':')&&k.endsWith('prd-drafts:v1'));if(!k)throw new Error('missing goal draft key');return Object.values(JSON.parse(localStorage.getItem(k))).find(v=>v.pending)?.pending??null;}""",owner)
 original=pending();assert original,original
 call('/auth/login',{'username':'other','password':'s'});btn('重试同一次保存').click();expect(page.get_by_role('textbox',name='账号',exact=True)).to_be_visible();assert pending()==original,(pending(),original)
 reveal_control(page,'textbox','账号').fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');btn('登录').click();btn('目标').click();btn('继续填写：已提交响应丢失的目标').click();expect(btn('重试同一次保存')).to_be_enabled();btn('重试同一次保存').click();expect(page.get_by_role('dialog',name='填写目标',exact=True)).to_have_count(0)
 state=ctx.request.get(BASE+'/api/prd/state').json();rows=[v for v in state['goals'] if v['input']['title']=='已提交响应丢失的目标'];assert len(rows)==1;history=ctx.request.get(BASE+'/api/prd/records/'+rows[0]['id']+'/history').json();assert len(history)==1,history
 # Block local persistence, then change the app's authenticated identity before
 # the old async handler can send. A global identity read at send time is unsafe.
 btn('目标').click();btn('新建目标').click();reveal_control(page,'textbox','本领名称').fill('延迟存储期间的原账号填写');expect(page.get_by_text('草稿已存本机',exact=True)).to_be_visible()
 key=page.evaluate("owner=>Object.keys(localStorage).find(k=>k.includes(':'+owner+':')&&k.endsWith('prd-drafts:v1'))",owner)
 initial=page.evaluate('key=>JSON.parse(localStorage.getItem(key))',key);entry=next(v for v in initial.values() if v['mutation']['value']['title']=='延迟存储期间的原账号填写')
 holder=ctx.new_page();holder.goto(BASE+'/api/health');holder.evaluate("""key=>{window.lockHeld=false;void navigator.locks.request('skarmar-draft:'+key,async()=>{window.lockHeld=true;await new Promise(r=>window.releaseLock=r);});}""",key);holder.wait_for_function('window.lockHeld===true')
 btn('保存').click();expect(btn('正在保存…')).to_be_disabled();call('/auth/logout',{});page.evaluate("window.dispatchEvent(new Event('focus'))");expect(page.get_by_role('textbox',name='账号',exact=True)).to_be_visible()
 reveal_control(page,'textbox','账号').fill('other');page.get_by_label('密码',exact=True).fill('s');btn('登录').click();expect(btn('记录上课')).to_be_enabled();holder.evaluate('window.releaseLock()')
 page.wait_for_function("({key,id,op})=>{const d=JSON.parse(localStorage.getItem(key))[id];return !d||(!d.pending&&d.operation_id!==op);}",arg={"key":key,"id":entry["id"],"op":entry["operation_id"]});state=ctx.request.get(BASE+'/api/prd/state').json();assert not any(g['input']['title']=='延迟存储期间的原账号填写' for g in state['goals']),state
 raw=page.evaluate('key=>JSON.parse(localStorage.getItem(key))',key);assert any(v['mutation']['value']['title']=='延迟存储期间的原账号填写' for v in raw.values()),raw;holder.close()
 assert not errors,errors
 print('PASS stale/cookie-switched and delayed old-session handlers cannot submit as another actor; original input and exact uncertain receipt survive',flush=True);b.close()
