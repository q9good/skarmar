"""Deterministic overlay-history race, nesting, forward/back, reload and sentinel-leak checks."""
from prd_ui_helpers import reveal_control
import os,re
from urllib.parse import urlparse
from playwright.sync_api import expect,sync_playwright

BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}

with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 context=browser.new_context(viewport={'width':360,'height':740})
 page=context.new_page()
 errors=[]
 page.on('pageerror',lambda error:errors.append(str(error)))
 def button(name,root=None):
  return (root or page).get_by_role('button',name=name,exact=True)
 def marker():
  return page.evaluate("history.state?.skarmarOverlay || null")
 def open_lesson():
  button('记录上课').click()
  expect(page.get_by_role('dialog',name='记录上课',exact=True)).to_be_visible()
 def close_with_back(title):
  page.go_back()
  expect(page.get_by_role('dialog',name=title,exact=True)).to_have_count(0)
  page.wait_for_function("!history.state?.skarmarOverlay")

 page.goto(BASE)
 reveal_control(page,'textbox','账号').fill('owner')
 page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password')
 button('登录').click()
 expect(button('记录上课')).to_be_enabled()
 app_url=page.url

 # Force stale-sentinel cleanup to remain pending without a timer. Before the fix,
 # the dialog was already interactive here; now it stays hidden until Back completes.
 page.evaluate("""()=>{
  window.__overlayRealBack=history.back;
  window.__overlayBackRequested=false;
  history.pushState({...history.state,skarmarOverlay:'skarmar:forced-stale'},'');
  history.back=()=>{window.__overlayBackRequested=true;};
 }""")
 button('记录上课').click()
 expect(page.get_by_role('dialog',name='记录上课',exact=True)).to_have_count(0)
 assert page.evaluate('window.__overlayBackRequested===true')
 assert page.url==app_url
 page.evaluate("""()=>{history.back=window.__overlayRealBack;history.back();}""")
 expect(page.get_by_role('dialog',name='记录上课',exact=True)).to_be_visible()
 assert marker() and marker().startswith('skarmar:')

 # Nested overlays close one layer per Back; the application URL stays mounted.
 button('课程名称（必填）：请选择课程').click()
 expect(page.get_by_role('dialog',name='选择课程',exact=True)).to_be_visible()
 child_marker=marker()
 page.go_back()
 expect(page.get_by_role('dialog',name='选择课程',exact=True)).to_have_count(0)
 expect(page.get_by_role('dialog',name='记录上课',exact=True)).to_be_visible()
 assert marker()!=child_marker and page.url==app_url
 close_with_back('记录上课')
 assert page.url==app_url

 # Forward must not resurrect an orphan overlay sentinel.
 page.go_forward()
 page.wait_for_function("!history.state?.skarmarOverlay")
 expect(page.get_by_role('dialog')).to_have_count(0)
 assert page.url==app_url

 # Reload on a sentinel entry drops UI state; reopening repairs the orphan without
 # growing history. No sleep/retry is used to make the transition pass.
 open_lesson()
 length_before_reload=page.evaluate('history.length')
 page.reload()
 expect(button('记录上课')).to_be_enabled()
 expect(page.get_by_role('dialog')).to_have_count(0)
 open_lesson()
 assert page.evaluate('history.length')==length_before_reload
 close_with_back('记录上课')

 # Repeated opening/Back reuses one forward slot instead of accumulating sentinels.
 stable_length=None
 for _ in range(8):
  open_lesson()
  assert marker() and marker().startswith('skarmar:')
  close_with_back('记录上课')
  current=page.evaluate('history.length')
  stable_length=current if stable_length is None else stable_length
  assert current==stable_length
  assert page.url==app_url

 assert not errors,errors
 print('PASS deterministic pending sentinel, nested Back, orphan Forward cleanup, reload repair and repeated open/close without history growth',flush=True)
 browser.close()
