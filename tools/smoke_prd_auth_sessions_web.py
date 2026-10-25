"""Trusted cookies, explicit revocation, renewal and original-form return; synthetic localhost only."""
import os,json
from urllib.parse import urlparse
from playwright.sync_api import sync_playwright,expect
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox']);a=browser.new_context(viewport={'width':360,'height':740});b=browser.new_context();page=a.new_page();other=b.new_page();errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 def btn(name,who=page):return who.get_by_role('button',name=name,exact=True)
 def login(who,trusted=False):
  who.goto(BASE);who.get_by_role('textbox',name='账号',exact=True).fill('owner');who.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');check=who.get_by_role('checkbox',name='在这台家庭设备保持登录约一年',exact=True);expect(check).to_have_attribute('aria-checked','false')
  if trusted:check.click()
  btn('登录',who).click();expect(btn('记录上课',who)).to_be_enabled()
 login(page,True);cookie=next(c for c in a.cookies() if c['name']=='skarma_session');assert cookie['httpOnly'] and cookie['sameSite']=='Strict';assert cookie['expires']>__import__('time').time()+364*86400
 login(other)
 btn('我的账号').click();btn('退出其他设备').click();btn('取消退出其他设备').click()
 response=b.request.get(BASE+'/api/auth/me');assert response.ok,(response.status,response.text())
 btn('退出其他设备').click();btn('确认退出本账号其他设备').click();expect(page.get_by_text('其他设备已退出，这台设备保持登录。',exact=True)).to_be_visible();assert b.request.get(BASE+'/api/auth/me').status==401;assert a.request.get(BASE+'/api/auth/me').ok
 print('PASS trusted opt-in secure cookie attributes and cancellable scoped other-device revocation',flush=True)
 # Open a real goal draft, then receive an impending-expiry metadata response on reload.
 btn('返回').click();btn('目标').click();btn('新建目标').click();page.get_by_role('textbox',name='本领名称',exact=True).fill('续期保留填写');btn('关闭填写目标').click()
 def expires_in(seconds):
  def handler(route):
   response=route.fetch();body=response.json();body['session']={'expires_at':__import__('time').time()+seconds,'trusted_device':True};route.fulfill(response=response,json=body)
  return handler
 page.route('**/api/auth/me',expires_in(14*86400+60),times=1);page.reload();expect(btn('重新验证并继续填写')).to_have_count(0)
 page.route('**/api/auth/me',expires_in(14*86400-60),times=1);page.reload();expect(btn('重新验证并继续填写')).to_be_visible();btn('继续填写：续期保留填写').click();expect(page.get_by_role('textbox',name='本领名称',exact=True)).to_have_value('续期保留填写')
 # Reauthenticate from the active form itself; it must remain mounted under the renewal overlay.
 field=page.get_by_role('textbox',name='本领名称',exact=True);field.fill('续期中的原表单');btn('登录即将到期，先验证登录再继续填写').click();page.get_by_role('textbox',name='续期密码',exact=True).fill('synthetic-prd-ui-password');btn('验证并回到填写').click();expect(page.get_by_role('dialog',name='续期登录',exact=True)).to_have_count(0);expect(page.get_by_role('textbox',name='本领名称',exact=True)).to_have_value('续期中的原表单')
 assert a.request.get(BASE+'/api/auth/me').ok;assert not errors,errors
 print('PASS trusted renewal prompt starts inside 14 days, stays nonblocking, and reauthentication preserves the original form',flush=True);browser.close()
