"""Synthetic localhost acceptance for the merged map and classroom review loop."""
import json,os,re
from pathlib import Path
from uuid import uuid4
from urllib.parse import urlparse
from playwright.sync_api import sync_playwright,expect
from prd_ui_helpers import reveal_control
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/workflow-2026-11-07/screenshots');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 ctx=browser.new_context(viewport={'width':390,'height':844});observer=browser.new_context(viewport={'width':390,'height':844});a,b=ctx.new_page(),observer.new_page();errors=[];shots=[]
 for page in (a,b):page.on('pageerror',lambda e:errors.append(str(e)))
 def click(name,page=a):reveal_control(page,'button',name).click()
 def field(name,page=a):return reveal_control(page,'textbox',name)
 def login(page,user,password):
  page.goto(BASE);field('账号',page).fill(user);page.get_by_label('密码',exact=True).fill(password);click('登录',page);expect(page.get_by_role('button',name='记录课堂',exact=True)).to_be_visible()
 def call(method,path,body=None,context=ctx,status=200):
  r=context.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','X-Skarma-Data-Epoch':'formal-20261101'});assert r.status==status,(method,path,r.status,r.text());return r.json()
 def entity(kind,value):
  i=str(uuid4());return call('PUT','/prd/records/'+i,{'id':i,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value},**({'method_create_mode':'full'} if kind=='method' else {})})['value']
 def add_knowledge(kind,title,goals=[],methods=[],**details):
  i=str(uuid4());return call('POST','/prd/process',{'kind':'knowledge','value':{'id':i,'operation_id':str(uuid4()),'expected_version':None,'action':'create','input':{'id':i,'kind':kind,'content':title,'goal_ids':goals,'method_ids':methods,'stage':'try' if kind=='experience' else None},'details':{'title':title,**details}}})
 def snap(name,focus=None,page=a):
  if focus is not None:focus.scroll_into_view_if_needed()
  else:page.evaluate("()=>{for(const e of document.querySelectorAll('*'))if(['auto','scroll'].includes(getComputedStyle(e).overflowY))e.scrollTop=0}")
  assert not page.evaluate('document.documentElement.scrollWidth>innerWidth+1')
  page.screenshot(path=str(OUT/(name+'.png')));shots.append(name+'.png')
 login(a,'owner','synthetic-prd-ui-password')
 hi=entity('goal',{'title':'合成关注互动','level':'long','long_priority':2})
 normal=entity('goal',{'title':'合成学习支持','level':'long','long_priority':1})
 low=entity('goal',{'title':'合成其他方向','level':'long','long_priority':0})
 short=entity('goal',{'title':'合成轮流一步','level':'short','parent_id':hi['id']})
 m=entity('method',{'title':'合成示范等待','steps':'合成先示范一步，再等待孩子回应','goal_ids':[short['id'],normal['id']],'priority':2})
 add_knowledge('difficulty','合成等待困难',[short['id']],priority=True)
 add_knowledge('experience','合成简短示范经验',[],[m['id']])
 add_knowledge('experience','合成其他原则不应串联',[low['id']])
 a.reload();expect(a.get_by_role('button',name='记录课堂',exact=True)).to_be_visible()
 widget=a.get_by_role('button',name='查看长期关注',exact=True);expect(widget).to_contain_text('合成关注互动');assert widget.inner_text().index('合成关注互动')<widget.inner_text().index('合成其他方向')
 def change_priority(g,priority):
  request={'id':g['id'],'operation_id':str(uuid4()),'expected_version':g['version'],'change_note':'','mutation':{'kind':'goal','value':{**g['input'],'long_priority':priority}}};return call('PUT','/prd/records/'+g['id'],request)['value']
 hi=change_priority(hi,0);low=change_priority(low,2);a.evaluate("window.dispatchEvent(new Event('focus'))")
 expect(widget).to_contain_text('合成其他方向');a.wait_for_function("()=>{const e=document.querySelector('[aria-label=\"查看长期关注\"]');return e&&e.innerText.indexOf('合成其他方向')<e.innerText.indexOf('合成关注互动')}")
 hi=change_priority(hi,2);low=change_priority(low,0);a.reload();expect(a.get_by_role('button',name='记录课堂',exact=True)).to_be_visible()
 click('目标与方法');graph=a.get_by_test_id('goal-method-map');expect(graph).to_be_visible();expect(a.get_by_role('button',name='新建目标',exact=True)).to_have_count(0)
 reveal_control(a,'checkbox','显示关联经验和困难').click();click('展开全部关系')
 branch=graph.get_by_test_id('map-goal-'+hi['id']);expect(branch.get_by_text('合成等待困难',exact=False).first).to_be_visible();expect(branch.get_by_text('合成其他原则不应串联',exact=False)).to_have_count(0)
 expect(graph.get_by_test_id('map-method-'+short['id']+'-'+m['id'])).to_be_visible();expect(graph.get_by_test_id('map-method-'+normal['id']+'-'+m['id'])).to_be_visible()
 snap('01-merged-map-390')
 a.set_viewport_size({'width':320,'height':844});snap('02-hierarchy-and-method-320',graph.get_by_test_id('map-goal-'+short['id']));a.set_viewport_size({'width':390,'height':844})
 click('目标');expect(a.get_by_role('button',name='新建目标',exact=True)).to_be_visible();click('方法');expect(a.get_by_role('button',name='新建方法',exact=True)).to_be_visible();click('今天')
 click('记录家庭');field('准备做什么').fill('合成活动：和孩子玩滚球，准备好时再试一步')
 plan=a.get_by_test_id('plan-goal-method-map');expect(plan).to_be_visible();expect(a.get_by_role('checkbox',name=re.compile('^选择目标：'))).to_have_count(0)
 plan.get_by_role('checkbox',name='显示关联经验和困难',exact=True).click();plan.get_by_role('button',name='展开全部关系',exact=True).click();snap('03-readonly-plan-map-390',a.get_by_label('准备做什么',exact=True))
 plan.get_by_test_id('map-method-'+short['id']+'-'+m['id']).get_by_role('button',name='查看方法：'+m['input']['title'],exact=True).click();expect(a.get_by_role('dialog',name='查看详情与修改记录',exact=True)).to_be_visible();click('关闭查看详情与修改记录');expect(field('准备做什么')).to_have_value('合成活动：和孩子玩滚球，准备好时再试一步')
 click('保存计划并退出');expect(a.get_by_role('dialog')).to_have_count(0)
 saved=call('GET','/prd/state')['sessions'][0];assert saved['status']=='planned' and not saved['actual_targets'] and not saved['input']['targets'] and not saved['input']['methods']
 call('POST','/auth/users',{'username':'grandparent','display_name':'合成记录者','password':'s','role':'member'})
 course=entity('catalog',{'kind':'course','name':'合成课堂','active':True});teacher=entity('catalog',{'kind':'teacher','name':'合成授课老师','active':True})
 login(b,'grandparent','s');today=b.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai'}).format(new Date())")
 lid=str(uuid4());obs={'business_date':today,'course_id':course['id'],'teacher_id':teacher['id'],'notes':''}
 original=call('PUT','/prd/lessons/'+lid,{'id':lid,'operation_id':str(uuid4()),'expected_version':None,'action':{'kind':'observation','value':obs}},context=observer);assert original['request_generation']==0
 b.reload();expect(b.get_by_role('button',name='记录课堂',exact=True)).to_be_visible();click('查看课记：合成课堂 · 合成授课老师',b);expect(b.get_by_role('button',name='查看课记修改记录',exact=True)).to_have_count(0)
 click('补充／修改课堂观察',b);reveal_control(b,'checkbox','老师交代的练习或建议').click();teacher_field=field('老师交代的练习或建议',b);placeholder=teacher_field.get_attribute('placeholder');assert '目标是' in placeholder;click('查看填写示例：老师交代的练习或建议',b);expect(b.get_by_text(re.compile('合成示例.*老师猜测.*建议'))).to_be_visible()
 teacher_note='合成老师建议：先听完再开始；第二步容易漏。老师猜测尚未准备好，建议等他表示准备好，再给短指令。'
 teacher_field.fill(teacher_note);click('提交记录',b);expect(b.get_by_role('dialog')).to_have_count(0);expect(b.get_by_role('button',name='检查连接并继续上传',exact=True)).to_have_count(0,timeout=20000)
 a.evaluate("window.dispatchEvent(new Event('focus'))");expect(a.get_by_text('待回看课堂观察：'+today+' · 合成课堂',exact=True)).to_be_visible(timeout=10000)
 click('整理：合成课堂');expect(a.get_by_role('dialog',name='整理课堂记录',exact=True)).to_be_visible();snap('04-parent-review-and-convert-390')
 click('根据观察新建目标');field('本领名称').fill('合成由观察提出的目标');reveal_control(a,'combobox','目标关注优先级').select_option('2');click('保存');expect(a.get_by_role('dialog',name='整理课堂记录',exact=True)).to_be_visible()
 new_goal=next(g for g in call('GET','/prd/state')['goals'] if g['input']['title']=='合成由观察提出的目标');assert new_goal['input']['long_priority']==2
 click('根据观察新建方法');field('方法名称').fill('合成由观察提出的方法');field('具体做法／必填').fill('合成等待准备好，再用短指令逐步尝试')
 click('选择方法适用目标（已选0）');reveal_control(a,'checkbox','合成由观察提出的目标').click();click('关闭选择方法适用目标');reveal_control(a,'combobox','方法关注优先级').select_option('2');click('保存');expect(a.get_by_role('dialog',name='整理课堂记录',exact=True)).to_be_visible()
 click('核对并刷新候选项');click('记一条经验');field('经验内容 1').fill('合成先获得准备信号再给短指令，作为待验证做法');reveal_control(a,'combobox','经验阶段 1').select_option('idea');click('调整关联 1');reveal_control(a,'checkbox','合成由观察提出的目标').click();reveal_control(a,'checkbox','合成由观察提出的方法').click();click('关闭调整经验或困难的关联')
 reveal_control(a,'combobox','这条记录如何处理').select_option('complete');click('保存整理');expect(a.get_by_role('dialog')).to_have_count(0)
 after=call('GET','/prd/state');lesson=next(l for l in after['lessons'] if l['id']==lid);assert lesson['organization']['state']=='done' and not lesson['organization']['targets'] and not lesson['organization']['methods']
 derived=next(k for k in after['knowledge'] if k['source_lesson_id']==lid);assert derived['source_observation']['teacher_notes']==teacher_note and derived['input']['stage']=='idea'
 next_note=teacher_note+' 合成第二次补充：换材料后需要更多等待。';latest={**lesson['observation'],'teacher_notes':next_note,'request_review':False,'review_message':''}
 call('PUT','/prd/lessons/'+lid,{'id':lid,'operation_id':str(uuid4()),'expected_version':lesson['observation_version'],'action':{'kind':'observation','value':latest}},context=observer)
 a.evaluate("window.dispatchEvent(new Event('focus'))");expect(a.get_by_text('待回看课堂观察：'+today+' · 合成课堂',exact=True)).to_be_visible(timeout=10000)
 assert next(r for r in call('GET','/prd/process?view=board')['inbox']['requests'] if r['id']==lid)['claimed_by'] is None
 again=call('GET','/prd/state');assert next(k for k in again['knowledge'] if k['input']['id']==derived['input']['id'])['source_observation']['teacher_notes']==teacher_note
 history=call('GET','/prd/lessons/'+lid+'/history');assert len([r for r in history if r['event']=='observation'])==1
 flags={'due':False,'fail':False}
 def board_fixture(route):
  if flags['fail']:route.fulfill(status=503,json={'error':'合成提醒暂不可用'});return
  response=route.fetch();data=response.json();tomorrow=(__import__('datetime').date.fromisoformat(today)+__import__('datetime').timedelta(days=1)).isoformat()
  for s in data['schedules']:
   s['reminders']=s['object_kind']=='goal' and s['object_id']==hi['id']
   if s['reminders']:s.update({'due_date':today if flags['due'] else tomorrow,'eligible':True,'mode':'regular'})
  route.fulfill(response=response,json=data)
 a.route('**/api/prd/process?view=board*',board_fixture);a.evaluate("window.dispatchEvent(new Event('focus'))");expect(a.get_by_role('button',name='今日复盘：合成关注互动',exact=True)).to_have_count(0)
 flags['due']=True;a.evaluate("window.dispatchEvent(new Event('focus'))");expect(a.get_by_role('button',name='今日复盘：合成关注互动',exact=True)).to_be_visible(timeout=10000);snap('05-today-due-and-new-classroom-390',a.get_by_role('button',name='今日复盘：合成关注互动',exact=True))
 flags['fail']=True;a.evaluate("window.dispatchEvent(new Event('focus'))");expect(a.get_by_text('提醒更新失败，当前显示可能尚未更新。',exact=True)).to_be_visible(timeout=10000)
 flags['fail']=False;a.evaluate("window.dispatchEvent(new Event('focus'))");expect(a.get_by_text('提醒更新失败，当前显示可能尚未更新。',exact=True)).to_have_count(0,timeout=10000)
 click('查看课记：合成课堂 · 合成授课老师');expect(a.get_by_text(re.compile('上次已处理；新补充待回看'))).to_be_visible();expect(a.get_by_role('button',name='查看课记修改记录',exact=True)).to_have_count(0);snap('06-current-classroom-without-history-390')
 assert not errors,errors
 report={'synthetic':True,'production_accessed':False,'deployed':False,'viewports':[320,390],'screenshots':shots,'checks':['dynamic long focus','merged default tree and management','many-to-many methods and scoped knowledge','activity-only plan unchanged by tree browsing','observation auto-review across family members','normal goal/method creation returns to organization','explicit idea conversion without false course coverage','subsequent observation reopens review and preserves source snapshot','no classroom history control or appended observation history','due date and focus refresh with controlled board fixture','reminder read failure and recovery','no browser errors or horizontal overflow']}
 (OUT.parent/'checks.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n');print('PASS combined map, read-only plan, classroom conversion and snapshots, cross-member reminders, due-day/focus/failure behavior');browser.close()
