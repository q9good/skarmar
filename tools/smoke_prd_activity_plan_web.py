"""Synthetic localhost: activity-only plans, human actual confirmation and goal rings."""
import json,os,re
from pathlib import Path
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import sync_playwright,expect
from prd_ui_helpers import reveal_control
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/activity-plan-2026-11-04/screenshots');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox']);ctx=browser.new_context(viewport={'width':390,'height':844});page=ctx.new_page();errors=[];shots=[]
 page.on('pageerror',lambda e:errors.append(str(e)))
 def btn(name,pg=page):return reveal_control(pg,'button',name)
 def click(name,pg=page):btn(name,pg).click()
 def field(name,pg=page):return reveal_control(pg,'textbox',name)
 def call(method,path,body=None,status=200,context=ctx):
  r=context.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','X-Skarma-Data-Epoch':'formal-20261101'});assert r.status==status,(method,path,r.status,r.text());return r.json()
 def entity(kind,value):
  i=str(uuid4());return call('PUT','/prd/records/'+i,{'id':i,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value},**({'method_create_mode':'full'} if kind=='method' else {})})['value']
 def knowledge(kind,title,goals=[],methods=[],**details):
  i=str(uuid4());return call('POST','/prd/process',{'kind':'knowledge','value':{'id':i,'operation_id':str(uuid4()),'expected_version':None,'action':'create','input':{'id':i,'kind':kind,'content':title,'goal_ids':goals,'method_ids':methods,'stage':'try' if kind=='experience' else None},'details':{'title':title,**details}}})
 def shot(name,pg=page,focus=None):
  if focus:focus.scroll_into_view_if_needed()
  else:pg.evaluate("()=>{for(const e of document.querySelectorAll('*'))if(['auto','scroll'].includes(getComputedStyle(e).overflowY))e.scrollTop=0}")
  pg.wait_for_timeout(80);assert not pg.evaluate('document.documentElement.scrollWidth>innerWidth+1');pg.screenshot(path=str(OUT/(name+'.png')));shots.append(name+'.png')
 def login(pg,user):
  pg.goto(BASE);field('账号',pg).fill(user);pg.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录',pg);expect(btn('记录课堂',pg)).to_be_enabled()
 login(page,'owner')
 call('POST','/auth/users',{'username':'family','display_name':'合成家人','password':'synthetic-prd-ui-password','role':'member'})
 long=entity('goal',{'title':'共同活动与交往（合成）','level':'long','long_priority':2})
 g=entity('goal',{'title':'轮流玩球（合成）','level':'short','parent_id':long['id'],'criteria':'能等待，并愿意邀请对方继续玩'})
 actual=entity('goal',{'title':'表达求助（合成）','level':'short','parent_id':long['id'],'frequency':'none','criteria':'遇到困难时，用语言或手势主动求助'})
 other=entity('goal',{'title':'暂停中的其他方向（合成）','level':'short','state':'paused'})
 done=entity('goal',{'title':'已经达成的本领（合成）','level':'short','state':'achieved'})
 m=entity('method',{'title':'先示范再等待（合成）','steps':'先加入孩子喜欢的玩法，留出回应时间。','goal_ids':[g['id'],other['id']]})
 used=entity('method',{'title':'回应求助（合成）','steps':'看见孩子表达需要时先回应，再一起完成。','goal_ids':[actual['id']]})
 direct=knowledge('experience','一步示范后等待回应（合成）',[g['id']],[m['id']])
 knowledge('experience','跟随兴趣，体验愉快（合成）',[long['id']])
 knowledge('experience','这个目标的独有细节不应串入（合成）',[other['id']])
 difficulty=knowledge('difficulty','换玩法时容易离开（合成）',[g['id']],[m['id']],priority=True)
 knowledge('difficulty','已解决的问题不再推送（合成）',[g['id']],state='resolved')
 course=entity('catalog',{'kind':'course','name':'认知（合成）','active':True});teacher=entity('catalog',{'kind':'teacher','name':'唐唐（合成）','active':True})
 today=page.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date())")
 lid=str(uuid4());lesson=call('PUT','/prd/lessons/'+lid,{'id':lid,'operation_id':str(uuid4()),'expected_version':None,'action':{'kind':'observation','value':{'business_date':today,'course_id':course['id'],'teacher_id':teacher['id'],'notes':'合成课堂：先示范，再一起轮流'}}})
 call('PUT','/prd/lessons/'+lid,{'id':lid,'operation_id':str(uuid4()),'expected_version':1,'action':{'kind':'organization','value':{'seen_observation_version':1,'targets':[{'goal_id':g['id'],'note':'合成课堂表现'}],'method_ids':[m['id']],'focus_goal_ids':[],'processing':'complete','request_generation':0,'outcome':'','new_knowledge':[],'new_actions':[]}}})
 page.reload();expect(btn('记录家庭')).to_be_enabled();click('记录家庭');plan=page.get_by_role('dialog',name='填写干预计划',exact=True)
 expect(plan.get_by_role('textbox')).to_have_count(1);expect(btn('保存计划并退出')).to_be_disabled();expect(plan.get_by_role('checkbox')).to_have_count(0)
 for name in ['选择本次目标','选择计划方法','快捷新增计划方法','补充其他信息（可选）']:expect(plan.get_by_role('button',name=name,exact=True)).to_have_count(0)
 field('准备做什么').fill('合成计划：准备玩球，愿意的话再试试别的玩法')
 expect(page.get_by_test_id('plan-reference-goal-'+g['id'])).to_be_visible();assert not page.get_by_test_id('plan-reference-goal-'+actual['id']).count()
 expect(page.get_by_test_id('reference-knowledge-'+direct['input']['id'])).to_have_count(1)
 expect(page.get_by_test_id('reference-knowledge-'+direct['input']['id'])).to_contain_text('直接关联：'+g['input']['title']);expect(page.get_by_test_id('reference-knowledge-'+direct['input']['id'])).to_contain_text('经方法「'+m['input']['title']+'」')
 assert not page.get_by_text('这个目标的独有细节不应串入（合成）',exact=True).count();assert not page.get_by_text('已解决的问题不再推送（合成）',exact=True).count()
 shot('01-activity-only-plan');shot('02-reference-knowledge',focus=page.get_by_test_id('plan-reference-knowledge'))
 click('查看目标参考：'+g['input']['title']);expect(page.get_by_role('dialog',name='目标参考',exact=True)).to_be_visible();expect(page.get_by_role('dialog').get_by_role('checkbox')).to_have_count(0);shot('03-read-only-reference');page.go_back();expect(plan).to_be_visible();expect(field('准备做什么')).to_have_value('合成计划：准备玩球，愿意的话再试试别的玩法')
 def lose(route):
  body=route.request.post_data_json;assert body['input']['targets']==[] and body['input']['methods']==[] and body['input']['summary'] is None
  r=route.fetch();assert r.ok;r.body();route.abort('failed')
 page.route('**/api/prd/sessions/*',lose,times=1);click('保存计划并退出');expect(btn('重试同一次干预保存')).to_be_visible();assert len(call('GET','/prd/state')['sessions'])==1
 page.reload();page.get_by_role('button',name=re.compile('^继续干预填写：')).click();click('重试同一次干预保存');expect(page.get_by_role('dialog')).to_have_count(0)
 saved=call('GET','/prd/state')['sessions'][0];sid=saved['id'];assert saved['status']=='planned' and saved['planned_targets']==[] and saved['planned_methods']==[] and saved['input']['targets']==[]
 assert call('GET','/prd/coverage?days=30&source=family')['records']['selected']==0
 print('PASS one activity field, read-only direct/method/ancestor references, excluded unrelated/solved knowledge and exact plan retry without selected goals',flush=True)
 ctx2=browser.new_context(viewport={'width':390,'height':844});pg=ctx2.new_page();pg.on('pageerror',lambda e:errors.append(str(e)));login(pg,'family');click('记录家庭：填写今天的总结',pg)
 expect(pg.get_by_role('textbox',name=re.compile('^实际表现：'))).to_have_count(0);expect(pg.get_by_role('combobox',name='实际方法确认',exact=True)).to_have_value('unconfirmed');shot('04-summary-before-selection',pg)
 click('选择本次目标',pg);expect(pg.get_by_role('dialog').get_by_text(long['input']['title']+' · 长期方向',exact=True)).to_be_visible();pg.get_by_role('checkbox',name='选择目标：'+actual['input']['title'],exact=True).click();click('完成目标选择',pg)
 field('实际表现：'+actual['input']['title'],pg).fill('合成实际：没有玩球，换活动时主动用手势求助')
 click('选择本次目标',pg);pg.get_by_role('checkbox',name='选择目标：'+actual['input']['title'],exact=True).click();click('完成目标选择',pg);expect(pg.get_by_role('textbox',name='实际表现：'+actual['input']['title'],exact=True)).to_have_count(0)
 click('选择本次目标',pg);pg.get_by_role('checkbox',name='选择目标：'+actual['input']['title'],exact=True).click();click('完成目标选择',pg);expect(field('实际表现：'+actual['input']['title'],pg)).to_have_value('合成实际：没有玩球，换活动时主动用手势求助')
 pg.get_by_role('combobox',name='实际方法确认',exact=True).select_option('used');reveal_control(pg,'checkbox','实际方法：'+used['input']['title']).click();shot('05-actual-goals-and-methods',pg)
 click('暂存总结并退出',pg);expect(pg.get_by_role('dialog')).to_have_count(0);state=call('GET','/prd/state');pending=next(s for s in state['sessions'] if s['id']==sid);assert pending['status']=='summary_draft' and pending['planned_targets']==[] and pending['actual_targets']==[]
 click('记录家庭：填写今天的总结',pg);expect(field('实际表现：'+actual['input']['title'],pg)).to_have_value('合成实际：没有玩球，换活动时主动用手势求助');click('正式提交干预总结',pg);expect(pg.get_by_role('dialog')).to_have_count(0)
 state=call('GET','/prd/state');completed=next(s for s in state['sessions'] if s['id']==sid);assert completed['planned_targets']==[] and completed['planned_methods']==[] and [t['goal']['id'] for t in completed['actual_targets']]==[actual['id']] and [m['id'] for m in completed['actual_methods']]==[used['id']]
 counts=call('GET','/prd/coverage?days=30&source=family');assert next(r for r in counts['goals'] if r['id']==actual['id'])['counts']['family']==1;assert next(r for r in counts['goals'] if r['id']==g['id'])['counts']['family']==0
 print('PASS another family member explicitly selects actual goals/methods, retains unchecked observations and draft, and never retrospectively creates a planned target',flush=True)
 click('记录');expect(page.get_by_role('button',name='记录课堂',exact=True)).to_have_count(0);expect(page.get_by_role('button',name=re.compile('^继续课记：'))).to_have_count(0);expect(btn('课堂记录')).to_be_visible();shot('06-classroom-results')
 click('查看课记：认知（合成） · '+today);expect(btn('补充／修改课堂观察')).to_be_visible();expect(btn('整理这条记录')).to_be_visible();click('关闭课记详情')
 click('今天');expect(btn('记录课堂')).to_be_enabled()
 print('PASS Records has no new-classroom/draft entry while results, statistics, detail maintenance and Today entry remain available',flush=True)
 # Deliberate report failure must remain unknown; retry resolves without inventing zeros.
 page.route('**/api/prd/coverage?days=30&source=family',lambda route:route.fulfill(status=503,content_type='application/json',body='{"error":"合成读取失败"}'),times=1)
 click('目标');expect(btn('重新读取目标总览')).to_be_visible();expect(page.get_by_test_id('goal-overview').get_by_role('img')).to_have_attribute('aria-label',re.compile('数据尚未读到'));click('重新读取目标总览')
 expect(page.get_by_test_id('goal-overview').get_by_role('img')).to_have_attribute('aria-label',re.compile('共1次目标命中'));shot('07-all-goal-coverage')
 click('查看总览目标：'+actual['input']['title']);expect(page.get_by_role('dialog',name='查看详情与修改记录',exact=True)).to_be_visible();page.go_back();expect(page.get_by_test_id('goal-overview')).to_be_visible()
 click('推进状态');expect(page.get_by_test_id('goal-overview').get_by_role('img')).to_have_attribute('aria-label',re.compile('已达成1项'));shot('08-goal-states')
 click('查看状态目标：已达成');expect(btn('查看状态目标详情：'+done['input']['title'])).to_be_visible();click('查看状态目标详情：'+done['input']['title']);page.go_back();expect(page.get_by_role('dialog',name='这一状态的目标',exact=True)).to_be_visible();page.go_back();click('覆盖分布')
 # A second browser's actual record refreshes the chart on focus without a reload.
 xid=str(uuid4());call('PUT','/prd/sessions/'+xid,{'id':xid,'operation_id':str(uuid4()),'expected_version':None,'action':'complete','input':{'business_date':today,'time_slot':'下午','summary':{'targets':[{'goal_id':g['id'],'performed':True,'observation':'合成另一位家人的实际记录'}],'confirmation':'unknown','methods':[]}}})
 page.evaluate("window.dispatchEvent(new Event('focus'))");expect(page.get_by_test_id('goal-overview').get_by_role('img')).to_have_attribute('aria-label',re.compile('共2次目标命中'))
 click('筛选目标');reveal_control(page,'combobox','目标覆盖来源').select_option('combined');click('筛选目标');expect(page.get_by_test_id('goal-overview').get_by_role('img')).to_have_attribute('aria-label',re.compile('共3次目标命中'))
 # Old plans remain readable and no longer pre-confirm their intended goal.
 legacy=str(uuid4());call('PUT','/prd/sessions/'+legacy,{'id':legacy,'operation_id':str(uuid4()),'expected_version':None,'action':'save_draft','input':{'business_date':today,'time_slot':'晚上','plan':'合成旧计划：保留原目标及笔记','targets':[{'goal_id':g['id'],'role':'main','note':'合成原笔记'}],'methods':[{'id':m['id'],'version':1}]}})
 page.reload();expect(btn('记录家庭：填写今天的总结')).to_be_visible();click('记录家庭：填写今天的总结');expect(reveal_control(page,'checkbox','这次做了：'+g['input']['title'])).not_to_be_checked();expect(page.get_by_role('textbox',name='实际表现：'+g['input']['title'],exact=True)).to_have_count(0);click('关闭填写干预总结')
 assert next(s for s in call('GET','/prd/state')['sessions'] if s['id']==legacy)['input']['targets'][0]['note']=='合成原笔记'
 for w,h in [(320,740),(430,932),(1440,900)]:
  page.set_viewport_size({'width':w,'height':h});click('目标');assert not page.evaluate('document.documentElement.scrollWidth>innerWidth+1')
 assert not errors,errors
 (OUT/'checks.json').write_text(json.dumps({'synthetic':True,'screenshots':shots,'checks':['activity_only_plan','read_only_bounded_reference_paths','deduplication','exact_receipt_retry','two_member_actual_confirmation','preserved_unchecked_observation','summary_draft_and_actual_snapshots','records_results_only_entry','all_goal_coverage_and_state_rings','unknown_is_not_zero','drilldown_back','other_member_refresh','separate_sources','legacy_plan_not_auto_performed','320_430_1440_layout'],'errors':errors},ensure_ascii=False,indent=2)+'\n')
 print('PASS complete scope: activity plans and actuals separated, results-only records, goal rings/drilldown/source sync and legacy confirmation',flush=True);browser.close()
