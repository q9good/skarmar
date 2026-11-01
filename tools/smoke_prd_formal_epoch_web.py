"""Isolated localhost: old trial drafts cannot resume/upload; formal drafts survive refresh."""
import os
from urllib.parse import urlparse
from playwright.sync_api import expect, sync_playwright
from prd_ui_helpers import reveal_control

BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
EPOCH='formal-20261101'
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 ctx=browser.new_context(viewport={'width':390,'height':844});page=ctx.new_page();errors=[]
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.goto(BASE);page.get_by_role('textbox',name='账号',exact=True).fill('owner')
 page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');page.get_by_role('button',name='登录',exact=True).click()
 expect(page.get_by_role('button',name='记录课堂',exact=True)).to_be_enabled()
 identity=ctx.request.get(BASE+'/api/auth/me').json();scope=identity['space']+':'+identity['user']['id']
 suffixes=['prd-drafts','session-drafts','lesson-drafts','observation-drafts','followup-drafts','lesson-queue','snapshot']
 old_keys=['skarmar:'+scope+':'+kind+':v1' for kind in suffixes]
 page.evaluate("keys=>keys.forEach(k=>localStorage.setItem(k,'{invalid old trial payload'))",old_keys)
 writes=[]
 page.on('request',lambda req:writes.append(req) if req.method in {'POST','PUT','DELETE'} and '/api/' in req.url and '/auth/' not in req.url else None)
 page.reload();expect(page.get_by_role('button',name='记录课堂',exact=True)).to_be_enabled()
 for tab in ['目标','方法','记录','跟进','今天']:
  reveal_control(page,'button',tab).click()
 assert not errors,errors
 assert writes==[],[(r.method,r.url) for r in writes]
 reveal_control(page,'button','目标').click();reveal_control(page,'button','新建目标').click()
 reveal_control(page,'textbox','本领名称').fill('合成正式目标：刷新后保留')
 expect(page.get_by_text('草稿已存本机',exact=True)).to_be_visible()
 key=EPOCH+':skarmar:'+scope+':prd-drafts:v1'
 page.wait_for_function("key=>Object.values(JSON.parse(localStorage.getItem(key)??'{}')).some(v=>v.mutation?.value?.title==='合成正式目标：刷新后保留')",arg=key)
 page.reload();reveal_control(page,'button','目标').click()
 reveal_control(page,'button','继续填写：合成正式目标：刷新后保留').click()
 expect(page.get_by_role('textbox',name='本领名称',exact=True)).to_have_value('合成正式目标：刷新后保留')
 reveal_control(page,'button','保存').click();expect(page.get_by_role('dialog',name='填写目标',exact=True)).to_have_count(0)
 snapshot=ctx.request.get(BASE+'/api/prd/state').json()
 assert [g['input']['title'] for g in snapshot['goals']]==['合成正式目标：刷新后保留']
 assert writes and all(r.headers.get('x-skarma-data-epoch')==EPOCH for r in writes)
 assert page.evaluate("keys=>keys.every(k=>localStorage.getItem(k)==='{invalid old trial payload')",old_keys)
 assert not errors,errors
 print('PASS formal browser generation: seven trial caches ignored, no automatic trial writes, formal draft restored and actual save carries epoch')
 browser.close()
