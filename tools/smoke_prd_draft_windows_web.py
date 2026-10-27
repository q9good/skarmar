"""Same account in multiple windows must not overwrite unrelated local drafts."""
from prd_ui_helpers import reveal_control
import os,re
from urllib.parse import urlparse
from playwright.sync_api import sync_playwright,expect
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox']);ctx=browser.new_context(viewport={'width':360,'height':740});a=ctx.new_page();errors=[];a.on('pageerror',lambda e:errors.append(str(e)))
 def btn(page,n):return page.get_by_role('button',name=n,exact=True)
 def click(page,n):reveal_control(page,'button',n).click()
 def field(page,n):return reveal_control(page,'textbox',n)
 def saved(page):expect(page.get_by_text('草稿已存本机',exact=True)).to_be_visible()
 a.goto(BASE);field(a,'账号').fill('owner');a.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click(a,'登录');expect(btn(a,'记录上课')).to_be_enabled();b=ctx.new_page();b.on('pageerror',lambda e:errors.append(str(e)));b.goto(BASE);expect(btn(b,'记录上课')).to_be_enabled()
 for page,title in [(a,'窗口A保留的目标'),(b,'窗口B保留的目标')]:
  click(page,'目标');click(page,'新建目标');field(page,'本领名称').fill(title);saved(page);click(page,'关闭填写目标')
 a.reload();expect(btn(a,'记录上课')).to_be_enabled();click(a,'目标');expect(btn(a,'继续填写：窗口A保留的目标')).to_be_visible();expect(btn(a,'继续填写：窗口B保留的目标')).to_be_visible();assert not errors,errors
 print('PASS unrelated goal drafts survive same-account multi-window writes and reload',flush=True)
 # Edit the same saved local draft from two windows. Keep both versions, then
 # reload the conflicted window and explicitly restore its retained content.
 b.reload();expect(btn(b,'记录上课')).to_be_enabled();click(b,'目标')
 for page in [a,b]:click(page,'继续填写：窗口A保留的目标')
 field(a,'本领名称').fill('同条冲突A填写');saved(a);field(b,'本领名称').fill('同条冲突B填写');expect(b.get_by_role('alert')).to_contain_text('另一窗口已修改这份草稿');expect(field(a,'本领名称')).to_have_value('同条冲突A填写');expect(field(b,'本领名称')).to_have_value('同条冲突B填写');click(b,'关闭填写目标');b.reload();expect(btn(b,'记录上课')).to_be_enabled();click(b,'目标');expect(btn(b,'继续填写：同条冲突A填写')).to_be_visible();click(b,'核对保留的目标方法填写');expect(b.get_by_role('dialog').get_by_text('名称：同条冲突B填写',exact=True)).to_be_visible();click(b,'载入保留填写 1 进行核对');expect(field(b,'本领名称')).to_have_value('同条冲突B填写');click(b,'关闭填写目标');click(a,'关闭填写目标');a.reload();expect(btn(a,'记录上课')).to_be_enabled();click(a,'目标');expect(btn(a,'继续填写：同条冲突B填写')).to_be_visible();click(a,'核对保留的目标方法填写');expect(a.get_by_role('dialog').get_by_text('名称：同条冲突A填写',exact=True)).to_be_visible();click(a,'关闭核对目标方法窗口填写')
 print('PASS conflicting same-record drafts preserve both copies across reload and require explicit restoration',flush=True)
 def close_top(page):btn(page,re.compile('^关闭')).last.click()
 def storage_text(suffix):
  return a.evaluate("suffix=>{const key=Object.keys(localStorage).find(k=>k.endsWith(suffix));return key?localStorage.getItem(key):'';}",suffix)
 def verify(suffix,one,two):
  text=storage_text(suffix);assert one in text and two in text,(suffix,text)
 # Both pages mount each editor before either edits its own draft.
 for page in [a,b]:click(page,'今天')
 for page,title in [(a,'窗口A本机计划'),(b,'窗口B本机计划')]:
  click(page,'记录一次家庭干预');field(page,'准备怎么做').fill(title);saved(page);close_top(page)
 verify(':session-drafts:v1','窗口A本机计划','窗口B本机计划')
 for page,title in [(a,'窗口A课堂观察'),(b,'窗口B课堂观察')]:
  click(page,'记录上课')
  if btn(page,'知道了，不再提示').is_visible():click(page,'知道了，不再提示')
  click(page,'补充观察');field(page,'今天课上怎么样？').fill(title);saved(page);close_top(page)
 verify(':lesson-drafts:v1','窗口A课堂观察','窗口B课堂观察')
 for page in [a,b]:click(page,'跟进')
 for page,title in [(a,'窗口A经验填写'),(b,'窗口B经验填写')]:
  click(page,'新建经验');field(page,re.compile(r'^(?:经验|困难)名称（必填）$')).fill(title);field(page,'做法与依据').fill(title+'的独立原文');saved(page);close_top(page)
 verify(':followup-drafts:v1','窗口A经验填写','窗口B经验填写')
 for page in [a,b]:click(page,'今天')
 for page,title in [(a,'窗口A瞬间观察'),(b,'窗口B瞬间观察')]:
  click(page,'记一个瞬间');field(page,'想记下的瞬间').fill(title);saved(page);close_top(page)
 verify(':observation-drafts:v1','窗口A瞬间观察','窗口B瞬间观察')
 a.reload();expect(btn(a,'记录上课')).to_be_enabled();assert not ctx.request.get(BASE+'/api/prd/state').json()['goals'];assert not errors,errors
 print('PASS intervention, classroom, knowledge and observation drafts all preserve independent windows; recovery never auto-uploads business data',flush=True);browser.close()
