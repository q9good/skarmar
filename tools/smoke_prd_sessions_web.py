"""Real browser family intervention lifecycle; fresh localhost synthetic database only."""
from prd_ui_helpers import reveal_control
import os,re
from pathlib import Path
from uuid import uuid4
from urllib.parse import urlparse
from playwright.sync_api import expect,sync_playwright
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/prd-session-ui');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 first=browser.new_context(viewport={'width':360,'height':640});second=browser.new_context(viewport={'width':430,'height':932});a,b=first.new_page(),second.new_page();errors=[]
 for page in [a,b]:page.on('pageerror',lambda e:errors.append(str(e)))
 def button(page,name):return page.get_by_role('button',name=name,exact=True)
 def click(page,name):reveal_control(page,'button',name).click()
 def field(page,name):return reveal_control(page,'textbox',name)
 def login(page,user,password):
  page.goto(BASE);field(page,'账号').fill(user);page.get_by_label('密码',exact=True).fill(password);click(page,'登录');expect(page.get_by_role('button',name=re.compile(r'^(记录一次家庭干预|填写今天的总结)$'))).to_be_visible()
 def call(method,path,body=None):
  r=first.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'});assert r.ok,r.text();return r.json()
 def state():return call('GET','/prd/state')
 def save(page,title):click(page,title);expect(page.get_by_role('dialog')).to_have_count(0)
 def select_target(page,title):
  if page.get_by_role('dialog',name='填写干预总结',exact=True).count() and page.get_by_role('button',name='选择本次目标',exact=True).count()==0:click(page,'调整总结目标与原计划')
  click(page,'选择本次目标');page.get_by_role('checkbox',name='选择目标：'+title,exact=True).click();click(page,'完成目标选择')
 def open_finished(page,observation):click(page,re.compile('.*'+re.escape(observation)+'.*'));expect(page.get_by_role('dialog')).to_have_count(1)
 login(a,'owner','synthetic-prd-ui-password');assert not state()['sessions']
 for title in ['记录上课','记录一次家庭干预']:expect(button(a,title)).to_be_in_viewport(ratio=1)
 call('POST','/auth/users',{'username':'family','display_name':'测试家人','password':'s','role':'member'})
 gids=[]
 for title in ['测试轮流','测试求助']:
  id=str(uuid4());gids.append(id);call('PUT','/prd/records/'+id,{'id':id,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':'goal','value':{'title':title,'level':'short'}}})
 a.reload();click(a,'记录一次家庭干预');expect(a.get_by_role('dialog',name='填写干预计划',exact=True)).to_be_visible();select_target(a,'测试轮流');reveal_control(a,'textbox','计划笔记：测试轮流').fill('轮流的原笔记');select_target(a,'测试求助');reveal_control(a,'textbox','计划笔记：测试求助').fill('求助的原笔记')
 reveal_control(a,'combobox','目标角色：测试求助').select_option('main');expect(field(a,'计划笔记：测试轮流')).to_have_value('轮流的原笔记');expect(field(a,'计划笔记：测试求助')).to_have_value('求助的原笔记');reveal_control(a,'combobox','目标角色：测试轮流').select_option('main')
 field(a,'准备怎么做').fill('测试：一起玩滚球，练轮流和求助')
 click(a,'快捷新增计划方法');field(a,'新方法名称').fill('测试先示范')
 def lose_method(route):
  assert route.request.post_data_json['method_create_mode']=='quick'
  response=route.fetch();assert response.ok;route.abort('failed')
 a.route('**/api/prd/records/*',lose_method,times=1);click(a,'保存方法并选入本次');expect(a.get_by_role('alert')).to_be_visible();assert len(state()['methods'])==1 and state()['methods'][0]['input']['steps']=='';click(a,'关闭快捷新增方法');click(a,'关闭填写干预计划');a.reload();click(a,re.compile(r'^继续干预填写：'));click(a,'继续确认快捷新增');click(a,'保存方法并选入本次');expect(a.get_by_role('dialog')).to_have_count(1);assert len(state()['methods'])==1
 save(a,'保存共享草稿并退出');assert len(state()['sessions'])==1 and state()['sessions'][0]['status']=='planned';assert call('GET','/prd/coverage?days=30&source=family')['records']['selected']==0
 expect(button(a,'填写今天的总结')).to_be_visible();print('PASS phone first screen, target roles swap without lost notes, inline method and shared plan excluded from counts',flush=True)
 login(b,'family','s')
 click(b,'填写今天的总结');field(b,'实际表现：测试轮流').fill('测试：等待了两次，主动把球给我');reveal_control(b,'checkbox','这次做了：测试求助').click()
 click(b,'正式提交干预总结');expect(b.get_by_role('alert')).to_contain_text('请确认实际使用的方法')
 reveal_control(b,'combobox','实际方法确认').select_option('unknown')
 click(b,'新增本次困难');field(b,'实际情况／必填 1').fill('测试：等待时离开了座位');field(b,'成人回应／可选 1').fill('测试：我先暂停并陪他休息');field(b,'原因猜测／可选 1').fill('测试：可能有些累了');field(b,'本次条件／可选 1').fill('测试：换了新材料')
 click(b,'查看填写示例：实际情况／必填 1');expect(b.get_by_text('大人怎样回应，后来呢？',exact=True)).to_be_visible();click(b,'查看填写示例：原因猜测／可选 1');expect(b.get_by_text('有什么尚未确定的猜测？',exact=True)).to_be_visible();expect(b.get_by_text('合成示例 · 例如：可能有些累了',exact=True)).to_be_visible();expect(field(b,'原因猜测／可选 1')).to_have_value('测试：可能有些累了')
 click(b,'新增本次经验');field(b,'经验内容 2').fill('测试：轮流前先示范有帮助');reveal_control(b,'combobox','经验阶段 2').select_option('try')
 click(b,'补充参与信号、情境和下一步');reveal_control(b,'checkbox','参与信号：主动开始').click();click(b,'信号范围：主动开始 · 整次背景');expect(b.get_by_role('dialog').get_by_role('checkbox')).to_have_count(1);reveal_control(b,'checkbox','测试轮流').click();click(b,'关闭参与信号关联范围')
 click(b,'新增本次下一步');field(b,'本次下一步 1').fill('测试：下次请老师示范等待')
 save(b,'正式提交干预总结');s=state();completed=s['sessions'][0];sid=completed['id'];assert completed['status']=='completed' and len(completed['actual_targets'])==1 and not completed['actual_methods'];assert len(s['knowledge'])==2 and len(s['actions'])==1
 assert completed['input']['summary']['signals'][0]['goal_ids']==[gids[0]];assert all(k['input']['goal_ids']==[gids[0]] for k in s['knowledge'])
 difficulty=next(k for k in s['knowledge'] if k['input']['kind']=='difficulty');assert difficulty['input']['content']=='测试：等待时离开了座位';assert difficulty['details']['adult_response']=='测试：我先暂停并陪他休息';assert difficulty['details']['guess']=='测试：可能有些累了';assert difficulty['details']['conditions']=='测试：换了新材料'
 print('PASS a second family member completes the shared plan; skipped target, unknown method, explicit signal scope and atomic knowledge/actions',flush=True)
 click(b,'目标');click(b,'查看目标：测试轮流');expect(b.get_by_text('关联困难：测试：等待时离开了座位',exact=True)).to_be_visible();expect(b.get_by_text(re.compile(r'^家庭观察：'))).to_be_visible();click(b,re.compile(r'^查看关联干预：'));expect(b.get_by_role('dialog',name='干预详情',exact=True)).to_be_visible();b.go_back();expect(b.get_by_role('dialog',name='查看详情与修改记录',exact=True)).to_be_visible();click(b,'关闭查看详情与修改记录')
 click(b,'跟进');click(b,'经验库与困难');expect(b.get_by_text('测试：轮流前先示范有帮助',exact=True)).to_be_visible();click(b,'查看知识：测试：等待时离开了座位');expect(b.get_by_text('实际情况：测试：等待时离开了座位',exact=True)).to_be_visible();expect(b.get_by_text('成人回应：测试：我先暂停并陪他休息',exact=True)).to_be_visible();expect(b.get_by_text('原因猜测（待核实）：测试：可能有些累了',exact=True)).to_be_visible();click(b,'关闭经验与困难详情')
 click(b,'今天');open_finished(b,'测试：等待了两次，主动把球给我');click(b,'修改已完成总结');field(b,'实际表现：测试轮流').fill('测试：修订为等待三次');field(b,'成人回应／可选 1').fill('测试：后来先回应求助再休息');field(b,'经验内容 2').fill('测试：经验补充了条件');field(b,'本次修订说明').fill('测试：补充观察条件');save(b,'保存总结修订');s=state();assert len(s['sessions'])==1 and len(s['knowledge'])==2 and next(k for k in s['knowledge'] if k['input']['kind']=='experience')['version']==2;revised_difficulty=next(k for k in s['knowledge'] if k['input']['kind']=='difficulty');assert revised_difficulty['version']==2 and revised_difficulty['details']['adult_response']=='测试：后来先回应求助再休息'
 print('PASS goal/source links, nested Back, member revision and existing outputs updated without duplicate counts',flush=True)
 # Concurrent edits keep both the server result and the active local text.
 a.reload();open_finished(a,'测试：修订为等待三次');click(a,'修改已完成总结');field(a,'实际表现：测试轮流').fill('测试：尚未提交的观察')
 open_finished(b,'测试：修订为等待三次');click(b,'修改已完成总结');field(b,'实际表现：测试轮流').fill('测试：家人又补充观察');field(b,'本次修订说明').fill('测试：新补充');save(b,'保存总结修订')
 a.evaluate("window.dispatchEvent(new Event('focus'))");expect(a.get_by_text('这条干预已有新版本，你的填写仍保留。',exact=True)).to_be_visible();expect(field(a,'实际表现：测试轮流')).to_have_value('测试：尚未提交的观察');expect(button(a,'保存总结修订')).to_be_disabled();click(a,'备份填写并载入最新干预');expect(field(a,'实际表现：测试轮流')).to_have_value('测试：家人又补充观察');expect(a.get_by_text('测试：尚未提交的观察',exact=True)).to_be_visible();field(a,'本次修订说明').fill('测试：核对后确认');save(a,'保存总结修订')
 print('PASS concurrent completed-summary edits preserve input and require explicit reconciliation',flush=True)
 click(a,'记录一次家庭干预');click(a,'更多干预操作');click(a,'已经做完：直接记总结');select_target(a,'测试求助');field(a,'实际表现：测试求助').fill('测试：用手势求助两次');click(a,'快捷新增实际方法');field(a,'新方法名称').fill('测试等待手势');click(a,'保存方法并选入本次');expect(a.get_by_role('combobox',name='实际方法确认',exact=True)).to_have_value('used')
 def lose_response(route):
  response=route.fetch();assert response.ok,response.text();route.abort('failed')
 a.route('**/api/prd/sessions/*',lose_response,times=1);click(a,'正式提交干预总结');expect(button(a,'重试同一次干预保存')).to_be_visible();assert len(state()['sessions'])==2
 a.reload();click(a,re.compile(r'^继续干预填写：'));save(a,'重试同一次干预保存');s=state();other=next(s for s in s['sessions'] if s['id']!=sid);assert len(s['sessions'])==2 and len(call('GET','/prd/sessions/'+other['id']+'/history'))==1;assert len(other['actual_methods'])==1 and other['input']['plan']=='';quick_method=next(m for m in s['methods'] if m['input']['title']=='测试等待手势');assert quick_method['input']['goal_ids']==[gids[1]] and quick_method['input']['steps']==''
 print('PASS direct summary, inline actual method, and lost response across reload retries exactly once',flush=True)
 click(a,'记录');click(a,'家庭干预');expect(a.get_by_text('家庭干预记录',exact=True)).to_be_visible();expect(a.get_by_role('button',name=re.compile(r'^\d{4}-\d{2}-\d{2} · 已完成\d+次$'))).not_to_have_count(0)
 click(a,'查看目标覆盖与实际方法');expect(a.get_by_text(re.compile(r'.*家庭 2 次，机构 0 节$'))).to_be_visible();reveal_control(a,'combobox','统计来源').select_option('combined');expect(a.get_by_text(re.compile(r'^测试轮流 · 1 次'))).to_be_visible();reveal_control(a,'combobox','统计窗口').select_option('7');expect(a.get_by_text(re.compile(r'^近7天固定色阶：'))).to_be_visible();a.screenshot(path=str(OUT/'coverage-360.png'));click(a,'关闭全部方向的近况')
 click(a,re.compile(r'.*已完成.*测试：用手势求助两次.*'));click(a,'删除干预（管理员）');save(a,'确认干预操作');assert next(s for s in state()['sessions'] if s['id']==other['id'])['status']=='deleted'
 # A planned action is reserved then released when the plan is cancelled.
 click(a,'今天');click(a,'记录一次家庭干预');expect(a.get_by_role('dialog',name='填写干预计划',exact=True)).to_be_visible();select_target(a,'测试轮流');click(a,'补充其他信息（可选）');reveal_control(a,'checkbox','纳入下一步：测试：下次请老师示范等待').click();save(a,'保存共享草稿并退出');assert state()['actions'][0]['status']=='in_plan'
 click(a,'记录');click(a,'家庭干预');click(a,re.compile(r'.*待总结计划.*'));click(a,'取消未执行计划');save(a,'确认干预操作');assert state()['actions'][0]['status']=='pending'
 print('PASS monthly history, 7/30 day source-separated coverage, admin delete, and cancelled plan releases the next action',flush=True)
 # One global draft survives navigation/refresh and is not visible to another account.
 click(a,'今天');click(a,'记录一次家庭干预');expect(a.get_by_role('dialog',name='填写干预计划',exact=True)).to_be_visible();field(a,'准备怎么做').fill('测试：本机保留安排');click(a,'选择本次目标');click(a,'就地新建近期本领');field(a,'新目标名称').fill('测试快捷本领');click(a,'保存目标并选入本次');click(a,'完成目标选择');expect(a.get_by_role('combobox',name='目标角色：测试快捷本领',exact=True)).to_have_value('main');click(a,'关闭填写干预计划');click(a,'目标');expect(button(a,re.compile(r'^继续干预填写：'))).to_be_visible();a.reload();click(a,re.compile(r'^继续干预填写：'));expect(field(a,'准备怎么做')).to_have_value('测试：本机保留安排');click(a,'关闭填写干预计划')
 for page in [a,b]:
  page.set_viewport_size({'width':320,'height':740});assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
 assert not errors,errors;print('PASS global draft recovery and 320px layout without browser errors',flush=True);browser.close()
