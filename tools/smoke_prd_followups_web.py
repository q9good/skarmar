"""Synthetic localhost-only followup, practice and review browser lifecycle."""
from prd_ui_helpers import reveal_control
import os,re
from uuid import uuid4
from pathlib import Path
from urllib.parse import urlparse
from playwright.sync_api import expect,sync_playwright
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/prd-followup-ui');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 first=browser.new_context(viewport={'width':360,'height':740});second=browser.new_context(viewport={'width':430,'height':932});a,b=first.new_page(),second.new_page();errors=[]
 for page in [a,b]:page.on('pageerror',lambda e:errors.append(str(e)))
 def button(page,name):return page.get_by_role('button',name=name,exact=True)
 def click(page,name):reveal_control(page,'button',name).click()
 def field(page,name):return reveal_control(page,'textbox',name)
 def login(page,user,password):
  page.goto(BASE);field(page,'账号').fill(user);page.get_by_label('密码',exact=True).fill(password);click(page,'登录');expect(button(page,'记录上课')).to_be_visible()
 def call(method,path,body=None):
  r=first.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'});assert r.ok,r.text();return r.json()
 def state():return call('GET','/prd/state')
 def read(view,id='',kind=''):return call('GET',f'/prd/process?view={view}&id={id}&object_kind={kind}')
 def close(page,title):click(page,'关闭'+title)
 def save(page,title='保存跟进',parent=True):
  editor_title=page.get_by_role('dialog').get_attribute('aria-label');assert editor_title
  click(page,title)
  # Hidden parent and visible editor both count as one accessible dialog. Wait
  # for this exact editor to unmount rather than a count that is already true.
  expect(page.get_by_role('dialog',name=editor_title,exact=True,include_hidden=True)).to_have_count(0)
  expect(page.get_by_role('dialog')).to_have_count(1 if parent else 0)
 def library(page):
  click(page,'跟进');click(page,'经验库与困难')
 def detail(page,title):click(page,'查看知识：'+title);expect(page.get_by_role('dialog',name='经验与困难详情',exact=True)).to_be_visible()
 def fill_bindings(page):
  click(page,'选择关联目标与方法');reveal_control(page,'checkbox','关联目标：测试近期本领').click();close(page,'关联目标与方法')
 login(a,'owner','synthetic-prd-ui-password');assert not state()['knowledge']
 family=call('POST','/auth/users',{'username':'family','display_name':'测试家人','password':'s','role':'member'})
 gid,mid=str(uuid4()),str(uuid4())
 for id,kind,value in [(gid,'goal',{'title':'测试近期本领','level':'short'}),(mid,'method',{'title':'测试方法','goal_ids':[gid]})]:
  body={'id':id,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value}};body.update({'method_create_mode':'quick'} if kind=='method' else {});call('PUT','/prd/records/'+id,body)
 a.reload();click(a,'跟进');click(a,'新建经验');field(a,re.compile(r'^(?:经验|困难)名称（必填）$')).fill('测试示范经验');field(a,'做法与依据').fill('测试：先示范，孩子愿意再尝试');fill_bindings(a);reveal_control(a,'combobox','经验阶段').select_option('helpful');save(a,parent=False)
 click(a,'经验库与困难');detail(a,'测试示范经验');kid=state()['knowledge'][0]['input']['id'];assert state()['knowledge'][0]['input']['stage']=='helpful'
 click(a,'记录本次经验反馈');reveal_control(a,'combobox','本次结果').select_option('helpful');field(a,'本次实际观察').fill('测试：愿意轮流');save(a);expect(a.get_by_text('实践总次数 1（附注不计次数）',exact=True)).to_be_visible()
 login(b,'family','s');library(b);detail(b,'测试示范经验');click(b,'补充看法：测试：愿意轮流');field(b,'补充不同的观察').fill('测试：另一种材料下没帮上');save(b);assert read('knowledge',kid)['summary']['total']==1
 click(b,'更正主结果：测试：愿意轮流');reveal_control(b,'combobox','本次结果').select_option('uncertain');field(b,'本次实际观察').fill('测试：更正为不确定');field(b,'主结果更正说明').fill('测试：核对后发现条件不同');save(b);assert read('knowledge',kid)['summary']['counts']=={'uncertain':1};click(b,'查看事件修订：测试：更正为不确定');expect(b.get_by_text(re.compile(r'^第 1 版 · 有帮助'))).to_be_visible();close(b,'实践事件修订历史')
 print('PASS direct existing-practice stage, independent feedback, authored annotation and explicit result revision keep one usage',flush=True)
 # Same-object edits never silently overwrite another member.
 click(a,'编辑经验或困难');field(a,'做法与依据').fill('测试：未提交经验内容');click(b,'编辑经验或困难');field(b,'做法与依据').fill('测试：家人更新经验内容');save(b)
 a.evaluate("window.dispatchEvent(new Event('focus'))");expect(a.get_by_text('已有新版本，你的填写仍保留。请核对最新内容后再保存。',exact=True)).to_be_visible();expect(field(a,'做法与依据')).to_have_value('测试：未提交经验内容');expect(button(a,'保存跟进')).to_be_disabled();click(a,'备份填写并载入最新跟进');expect(field(a,'做法与依据')).to_have_value('测试：家人更新经验内容');expect(a.get_by_text('测试：未提交经验内容',exact=True)).to_be_visible();save(a);close(a,'经验与困难详情');close(b,'经验与困难详情')
 click(a,'新建困难');field(a,re.compile(r'^(?:经验|困难)名称（必填）$')).fill('测试等待困难');field(a,'困难描述').fill('测试：等待时还需要支持');fill_bindings(a);click(a,'补充条件、成人回应与比较口径');field(a,'比较口径').fill('每次5次机会');field(a,'比较所用的帮助条件').fill('示范一次');save(a,parent=False);detail(a,'测试等待困难');diff=next(k for k in state()['knowledge'] if k['input']['kind']=='difficulty');did=diff['input']['id']
 for i in range(3):
  click(a,'记录本次困难跟进');reveal_control(a,'combobox','本次结果').select_option('same');field(a,'本次实际观察').fill(f'测试：可比观察{i+1}');reveal_control(a,'checkbox','家人确认：同口径、同帮助条件，可比较').click();save(a)
  if i<2:assert button(a,'查看并确认重点与每周回看').count()==0
 expect(button(a,'查看并确认重点与每周回看')).to_be_visible();assert not read('knowledge',did)['knowledge']['details']['priority']
 click(a,'记录本次困难跟进');reveal_control(a,'combobox','本次结果').select_option('unobserved');field(a,'本次实际观察').fill('测试：本次没有出现这种情境');save(a);assert read('knowledge',did)['summary']['comparable_count']==3
 click(a,'查看并确认重点与每周回看');field(a,'修改依据或原因').fill('测试：明确决定每周回看');save(a);schedule=next(s for s in read('board')['schedules'] if s['object_id']==did);assert schedule['interval']=='weekly',(schedule,read('knowledge',did))
 click(a,'编辑经验或困难');reveal_control(a,'combobox',re.compile(r'^(?:经验|困难)状态$')).select_option('resolved');field(a,'依据和条件（可选）').fill('测试：已有几次可见改善');field(a,'修改依据或原因').fill('测试：家人明确解决');save(a);assert not next(s for s in read('board')['schedules'] if s['object_id']==did)['eligible']
 click(a,'提炼为新经验');field(a,re.compile(r'^(?:经验|困难)名称（必填）$')).fill('测试提炼的新经验');field(a,'做法与依据').fill('测试：在新的条件下先等待');reveal_control(a,'combobox','经验阶段').select_option('try');field(a,'与原经验或困难的差异').fill('测试：扩展到不同材料');save(a);close(a,'经验与困难详情');assert len(state()['knowledge'])==3
 print('PASS conflicts preserve input; three comparable records only propose weekly review; unobserved excluded; explicit resolution and derived experience preserve the original',flush=True)
 # A formal summary can record existing-knowledge feedback in its own atomic save.
 today=call('GET','/prd/coverage?days=30&source=family')['end'];sid=str(uuid4());fid=str(uuid4())
 call('PUT','/prd/sessions/'+sid,{'id':sid,'operation_id':str(uuid4()),'expected_version':None,'action':'complete','input':{'business_date':today,'time_slot':'晚上','summary':{'targets':[{'goal_id':gid,'performed':True,'observation':'测试：复盘目标观察'}],'confirmation':'used','methods':[{'id':mid,'version':1}],'signals':[{'kind':'求助','goal_ids':[gid]},{'kind':'主动开始','method_ids':[mid]},{'kind':'投入愉快'}],'feedback':[{'id':fid,'knowledge_id':kid,'result':'helpful'}]}}})
 assert read('knowledge',kid)['summary']['total']==2
 click(a,'目标');click(a,'查看目标：测试近期本领');click(a,'现在复盘这个对象');expect(a.get_by_text('明确关联的参与信号：求助 · '+today,exact=True)).to_be_visible();assert a.get_by_text('明确关联的参与信号：投入愉快 · '+today,exact=True).count()==0
 click(b,'目标');click(b,'查看目标：测试近期本领');click(b,'现在复盘这个对象')
 reveal_control(a,'combobox','本次复盘决定').select_option('adjust');field(a,'复盘依据与说明').fill('测试：根据实际表现调整');field(a,'调整后的标准或做法（可选）').fill('测试：减少帮助，观察主动表达');click(a,'新增复盘下一步');field(a,'复盘下一步 1').fill('测试：请老师示范一个变化');save(a,'保存复盘');assert read('reviews')['reviews'][0]['object_id']==gid
 b.evaluate("window.dispatchEvent(new Event('focus'))");expect(b.get_by_text('已有新版本，你的填写仍保留。请核对最新内容后再保存。',exact=True)).to_be_visible();click(b,'备份填写并载入最新跟进');expect(b.get_by_text(re.compile(r'^上次决定：调整做法或条件'))).to_be_visible();close(b,'逐项复盘');close(b,'查看详情与修改记录')
 close(a,'查看详情与修改记录');assert read('review_context',gid,'goal')['total']==0
 print('PASS embedded summary feedback, explicit signal scope, atomic review/action/calendar and two-member completion protection',flush=True)
 # Explicit action assignment and cancellation remain ordinary member edits, with history.
 click(a,'跟进');click(a,'维护下一步：测试：请老师示范一个变化');reveal_control(a,'combobox','下一步负责人').select_option(family['id']);field(a,'下一步修改说明').fill('测试：请家人接手');save(a,parent=False);task=next(a for a in state()['actions'] if a['input']['content']=='测试：请老师示范一个变化');assert task['input']['assignee_id']==family['id']
 click(a,'维护下一步：测试：请老师示范一个变化');reveal_control(a,'combobox','下一步状态').select_option('cancelled');field(a,'下一步修改说明').fill('测试：明确取消');save(a,parent=False);reveal_control(a,'checkbox','显示已完成或取消的下一步').click();click(a,'维护下一步：测试：请老师示范一个变化');click(a,'查看下一步修改记录');expect(a.get_by_text(re.compile(r'^第 3 版'))).to_be_visible();close(a,'下一步修订历史');close(a,'维护下一步')
 # Revise the historical description without advancing the schedule a second time.
 click(a,'跟进');click(a,'查看已完成复盘');click(a,'更正复盘说明：测试近期本领');field(a,'复盘依据与说明').fill('测试：补充这次复盘的说明');before=next(s for s in read('board')['schedules'] if s['object_id']==gid);save(a,'保存复盘说明修订');after=next(s for s in read('board')['schedules'] if s['object_id']==gid);assert before==after;close(a,'已完成复盘')
 click(a,'准备给专业人员的简要更新');reveal_control(a,'checkbox','摘要目标：测试近期本领').click();click(a,'生成可删减的摘要草稿');expect(field(a,'可编辑的更新摘要')).to_contain_text('测试：复盘目标观察')
 with a.expect_download() as download:click(a,'下载这份摘要')
 assert download.value.suggested_filename.startswith('skarmar-summary-');close(a,'简要更新预览')
 # Group deferral displays a concrete list and never adds completion events.
 for n in range(2):
  id=str(uuid4());call('POST','/prd/process',{'kind':'knowledge','value':{'id':id,'operation_id':str(uuid4()),'expected_version':None,'action':'create','input':{'id':id,'kind':'difficulty','content':f'测试无新证据{n}','goal_ids':[gid],'next_review_date':today},'details':{'title':f'测试顺延组{n}'}}})
 a.reload();click(a,'开始本周复盘');click(a,'查看无新证据组顺延');next_day=__import__('datetime').date.fromisoformat(today)+__import__('datetime').timedelta(days=7);a.get_by_label('整组顺延到：选择日期',exact=True).fill(str(next_day));n=len(read('reviews')['reviews']);save(a,'确认顺延这组事项');assert len(read('reviews')['reviews'])==n;close(a,'共享复盘队列');click(a,'跟进')
 # Lost response after the server commits a new knowledge object: refresh and retry once.
 click(a,'新建经验');field(a,re.compile(r'^(?:经验|困难)名称（必填）$')).fill('测试弱网经验');field(a,'做法与依据').fill('测试：弱网保存');fill_bindings(a);reveal_control(a,'combobox','经验阶段').select_option('idea')
 def lose(route):
  response=route.fetch();assert response.ok,response.text();route.abort('failed')
 a.route('**/api/prd/process',lose,times=1);click(a,'保存跟进');expect(button(a,'重试同一次跟进保存')).to_be_visible();assert len(state()['knowledge'])==6;a.reload();click(a,'继续跟进填写：经验或困难 · 测试弱网经验');save(a,'重试同一次跟进保存',parent=False);assert len(state()['knowledge'])==6
 print('PASS historical review correction preserves schedule; editable downloadable brief; lost-response refresh retry is idempotent',flush=True)
 a.set_viewport_size({'width':320,'height':740});assert not a.evaluate('document.documentElement.scrollWidth > innerWidth');a.screenshot(path=str(OUT/'followup-320.png'));assert not errors,errors;browser.close();print('PASS narrow layout and no browser exceptions',flush=True)
