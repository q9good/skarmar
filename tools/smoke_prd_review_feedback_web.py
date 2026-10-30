"""User's 2026-10-29 annotations: actual linked writes and retained optional input.
Only a fresh localhost database, synthetic records and synthetic screenshots.
"""
import json,os,re
from datetime import date,timedelta
from pathlib import Path
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import expect,sync_playwright
from prd_ui_helpers import reveal_control
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/review-feedback-2026-10-30');OUT.mkdir(parents=True,exist_ok=True)
shots=[]
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 ctx=browser.new_context(viewport={'width':390,'height':844});page=ctx.new_page();errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 def btn(n,scope=None):return (scope or page).get_by_role('button',name=n,exact=True)
 def click(n):reveal_control(page,'button',n).click()
 def field(n):return reveal_control(page,'textbox',n)
 def select(n,v):reveal_control(page,'combobox',n).select_option(v)
 def check(n):reveal_control(page,'checkbox',n).click()
 def call(method,path,body=None,context=ctx):
  r=context.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'});assert r.ok,(r.status,r.text());return r.json()
 def state():return call('GET','/prd/state')
 def entity(kind,value):
  id=str(uuid4());return call('PUT','/prd/records/'+id,{'id':id,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value},**({'method_create_mode':'full'} if kind=='method' else {})})['value']
 def knowledge(kind,content,goal,method):
  id=str(uuid4());return call('POST','/prd/process',{'kind':'knowledge','value':{'id':id,'operation_id':str(uuid4()),'expected_version':None,'action':'create','input':{'id':id,'kind':kind,'content':content,'goal_ids':[goal],'method_ids':[method],'stage':'try' if kind=='experience' else None},'details':{'title':content},'derived_from':[],'note':''}})
 def close_all():
  for _ in range(8):
   if not page.get_by_role('dialog').count():return
   page.get_by_role('dialog').get_by_role('button',name=re.compile('^关闭')).first.click()
  assert not page.get_by_role('dialog').count()
 def shot(name,focus=None):
  if focus is None:page.evaluate("()=>{for(const e of document.querySelectorAll('*'))if(['auto','scroll'].includes(getComputedStyle(e).overflowY))e.scrollTop=0}")
  else:focus.scroll_into_view_if_needed()
  page.wait_for_timeout(80);assert not page.evaluate('document.documentElement.scrollWidth>innerWidth+1');page.screenshot(path=str(OUT/(name+'.png')));shots.append(name+'.png')
 def login(pg,user='owner'):
  pg.goto(BASE);pg.get_by_role('textbox',name='账号',exact=True).fill(user);pg.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');pg.get_by_role('button',name='登录',exact=True).click();expect(pg.get_by_role('button',name='记录课堂',exact=True)).to_be_enabled()
 login(page);assert not state()['goals']
 long=entity('goal',{'title':'合成方向：交往与互动','level':'long','long_priority':0})
 other=entity('goal',{'title':'合成方向：主动探索','level':'long'})
 a=entity('goal',{'title':'合成本领：轮流滚球','level':'short','parent_id':long['id'],'criteria':'轮流机会中主动递球，必要时示范一次'})
 b=entity('goal',{'title':'合成本领：换新材料','level':'short','parent_id':other['id']})
 m=entity('method',{'title':'合成方法：示范后等待','steps':'跟随兴趣，示范一步后等待回应','goal_ids':[a['id'],long['id']]})
 exp=knowledge('experience','合成经验：先选喜欢的材料',a['id'],m['id']);diff=knowledge('difficulty','合成困难：换材料时需要支持',a['id'],m['id'])
 c=entity('catalog',{'kind':'course','name':'合成课程：认知','active':True});t=entity('catalog',{'kind':'teacher','name':'合成老师：唐唐','active':True})
 today=page.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date())")
 for days in [1,3]:
  id=str(uuid4());call('PUT','/prd/sessions/'+id,{'id':id,'operation_id':str(uuid4()),'expected_version':None,'action':'complete','input':{'business_date':(date.fromisoformat(today)-timedelta(days=days)).isoformat(),'time_slot':'晚上','targets':[{'goal_id':a['id'],'role':'main'}],'summary':{'targets':[{'goal_id':a['id'],'performed':True,'observation':'合成历史：示范后愿意轮流'}],'confirmation':'used','methods':[{'id':m['id'],'version':1}]}},'change_note':''})
 page.reload();expect(btn('记录课堂')).to_be_enabled()
 def assert_peers(label,left,right,selected):
  group=page.get_by_role('group',name=label,exact=True);expect(group).to_be_visible()
  controls=[group.get_by_role('button',name=n,exact=True) for n in [left,right]]
  boxes=[control.bounding_box() for control in controls];assert abs(boxes[0]['width']-boxes[1]['width'])<=1 and abs(boxes[0]['height']-boxes[1]['height'])<=1;assert all(b['height']>=44 for b in boxes)
  styles=[control.evaluate("e=>{const s=getComputedStyle(e),t=getComputedStyle(e.querySelector('div:last-child'));return [s.backgroundColor,t.color,t.fontWeight,t.fontSize]}") for control in controls];assert styles[0]==styles[1],styles
  for control,name in zip(controls,[left,right]):assert control.get_attribute('aria-pressed')==str(name==selected).lower()
  return group
 classroom=btn('记录课堂');family=btn('记录家庭');assert classroom.evaluate('e=>getComputedStyle(e).backgroundColor')==family.evaluate('e=>getComputedStyle(e).backgroundColor')
 assert classroom.bounding_box()['width']==family.bounding_box()['width'];assert classroom.bounding_box()['height']==family.bounding_box()['height']
 assert classroom.evaluate('e=>getComputedStyle(e).borderLeftColor')!=family.evaluate('e=>getComputedStyle(e).borderLeftColor')
 shot('01-today')
 click('查看长期关注');click('调整关注优先级：'+long['input']['title']);select('长期关注优先级','2');shot('15-long-priority');click('保存');expect(page.get_by_role('dialog')).to_have_count(0)
 assert next(g for g in state()['goals'] if g['id']==long['id'])['input']['long_priority']==2
 page.reload();expect(btn('查看长期关注')).to_contain_text(long['input']['title']);titles=btn('查看长期关注').inner_text();assert titles.index(long['input']['title'])<titles.index(other['input']['title'])
 call('POST','/auth/users',{'username':'family','display_name':'合成第二位家人','password':'synthetic-prd-ui-password','role':'member'})
 ctx2=browser.new_context();pg2=ctx2.new_page();login(pg2,'family');assert pg2.get_by_role('button',name='查看长期关注',exact=True).inner_text().index(long['input']['title'])<pg2.get_by_role('button',name='查看长期关注',exact=True).inner_text().index(other['input']['title']);ctx2.close()
 print('PASS equal sibling entry surfaces and category accents and shared long-focus priority after reload and second-family login',flush=True)
 click('记录家庭');expect(page.get_by_test_id('plan-coverage-'+a['id'])).to_contain_text('家庭 2 次');expect(page.get_by_test_id('plan-coverage-'+b['id'])).to_contain_text('家庭 0 次');shot('02-plan-coverage');click('查看全部方向的近况');click('调整统计范围与图表');select('近况展示方式','ring');select('统计来源','combined');click('调整统计范围与图表');shot('16-balance-with-data');click('关闭全部方向的近况')
 click('选择本次目标');check('选择目标：'+a['input']['title']);click('完成目标选择')
 check('计划方法：'+m['input']['title']);expect(page.get_by_text('留意困难：合成困难：换材料时需要支持',exact=True).first).to_be_visible();expect(page.get_by_text('可参考经验：合成经验：先选喜欢的材料',exact=True).first).to_be_visible()
 field('准备怎么做').fill('合成安排：用喜欢的球示范轮流，再尝试一个新球')
 shot('03-plan-target-method',field('准备怎么做'));click('保存共享草稿并退出');expect(page.get_by_role('dialog')).to_have_count(0)
 click('记录家庭：填写今天的总结');field('实际表现：'+a['input']['title']).fill('合成表现：示范后愿意滚球，换球时需要休息');select('实际方法确认','used');check('实际方法：'+m['input']['title'])
 expect(btn('新增本次经验')).to_have_count(0);expect(btn('新增本次困难')).to_have_count(0);shot('04-summary-basic')
 check('补充经验和困难（可选）');click('新增本次经验');field('经验内容 1').fill('合成新增经验：先让他选球');select('经验阶段 1','try')
 click('新增本次困难');field('实际情况／必填 2').fill('合成新增困难：新球需要更多适应时间');field('成人回应／可选 2').fill('先休息再邀请')
 check('补充经验和困难（可选）');expect(field('实际表现：'+a['input']['title'])).to_have_value('合成表现：示范后愿意滚球，换球时需要休息');check('补充经验和困难（可选）');expect(field('经验内容 1')).to_have_value('合成新增经验：先让他选球')
 shot('05-summary-new-knowledge',field('经验内容 1'));click('正式提交干预总结');expect(page.get_by_role('dialog')).to_have_count(0)
 s=state();new=next(s for s in s['sessions'] if s['input']['business_date']==today);assert len(s['knowledge'])==4;assert all(k['input']['goal_ids']==[a['id']] for k in s['knowledge'] if k['source_session_id']==new['id'])
 click('查看家庭干预：合成表现：示范后愿意滚球，换球时需要休息');click('修改已完成总结');expect(field('经验内容 1')).to_have_value('合成新增经验：先让他选球');field('经验内容 1').fill('合成新增经验：选球后留出等待');field('本次修订说明').fill('合成补充条件');click('保存总结修订');expect(page.get_by_role('dialog')).to_have_count(0)
 print('PASS plan report shows uneven/zero real counts, selected method knowledge, optional summary both kinds and saved-summary reentry',flush=True)
 click('记录课堂');expect(page.get_by_role('dialog').get_by_role('button',name='合成课程：认知',exact=True)).to_have_count(0)
 expect(page.get_by_role('dialog').get_by_role('button',name='今天',exact=True)).to_have_count(0)
 click('课程名称（必填）：请选择课程');click(c['input']['name']);click('授课老师（必填）：请选择老师');click(t['input']['name']);shot('06-lesson-basic');click('提交记录');expect(page.get_by_role('dialog')).to_have_count(0)
 expect(btn('补充课堂观察')).to_be_visible();before=len(state()['lessons']);click('补充课堂观察');check('老师交代的练习或建议');field('老师交代的练习或建议').fill('合成建议：在家练轮流')
 check('我看到需要支持或提高的');field('我看到需要支持或提高的').fill('合成观察：换球时需要支持')
 check('我看到的好方法');field('我看到的好方法').fill('合成好方法：老师先请孩子选球');check('我看到的好方法');check('我看到的好方法');expect(field('我看到的好方法')).to_have_value('合成好方法：老师先请孩子选球')
 shot('07-lesson-observation',field('老师交代的练习或建议'));click('提交记录');expect(page.get_by_role('dialog')).to_have_count(0)
 page.wait_for_function("async()=>{const s=await (await fetch('/api/prd/state')).json();return s.lessons.some(l=>l.observation.helpful_notes==='合成好方法：老师先请孩子选球')}");assert len(state()['lessons'])==before;lesson=state()['lessons'][0];assert lesson['observation']['helpful_notes']=='合成好方法：老师先请孩子选球'
 click('完成');click('查看课记：合成课程：认知 · 合成老师：唐唐');click('整理这条记录');expect(page.get_by_text('把课堂线索用于下一次家庭干预',exact=True)).to_be_visible();check(a['input']['title']);check('实际方法：'+m['input']['title']);shot('08-lesson-links');click('保存整理');expect(page.get_by_role('dialog')).to_have_count(0)
 click('目标');click('查看目标：'+a['input']['title']);expect(page.get_by_role('heading',name='关联方法',exact=True)).to_be_visible();expect(page.get_by_role('heading',name='可参考的经验',exact=True)).to_be_visible();expect(page.get_by_role('heading',name='需要跟进的困难',exact=True)).to_be_visible();shot('09-goal-groups');close_all()
 click('方法');expect(btn('查看方法关联目标：'+a['input']['title'])).to_be_visible();shot('10-method-goals');click('查看方法关联目标：'+a['input']['title']);expect(page.get_by_role('dialog')).to_have_count(1);close_all()
 click('跟进');assert_peers('跟进内容','要处理的','经验库与困难','要处理的');expect(btn('新建经验')).to_have_count(0);click('经验库与困难');assert_peers('跟进内容','要处理的','经验库与困难','经验库与困难');assert_peers('资料类型','经验','困难','经验');expect(btn('新建经验')).to_be_visible();expect(btn('新建困难')).to_have_count(0);expect(page.get_by_text('困难：合成困难：换材料时需要支持',exact=True)).to_have_count(0);shot('11-experience-tab')
 click('困难');assert_peers('资料类型','经验','困难','困难');expect(btn('新建困难')).to_be_visible();expect(btn('新建经验')).to_have_count(0);expect(page.get_by_text('经验：合成经验：先选喜欢的材料',exact=True)).to_have_count(0);shot('12-difficulty-tab')
 click('目标');click('查看目标：'+a['input']['title']);click('现在复盘这个对象');expect(page.get_by_text('当前观察标准：'+a['input']['criteria'],exact=True)).to_be_visible();click('复盘要解决什么');shot('13-review-purpose');click('复盘要解决什么')
 field('复盘依据与说明').fill('合成决定：熟悉材料继续轮流，新材料先示范');click('新增复盘下一步');field('复盘下一步 1').fill('合成下一步：只换一种新球');click('保存复盘');expect(page.get_by_role('dialog',name='逐项复盘',exact=True)).to_have_count(0);close_all()
 s=state();assert any(a['input']['content']=='合成下一步：只换一种新球' for a in s['actions'])
 click('记录');assert_peers('记录类型','机构课程','家庭干预','机构课程');assert_peers('机构记录视图','课堂记录','课时核对','课堂记录');click('课时核对');assert_peers('机构记录视图','课堂记录','课时核对','课时核对');assert_peers('课时汇总口径','按老师汇总','按课程汇总','按老师汇总');click('下个月');click('按课程汇总');assert_peers('课时汇总口径','按老师汇总','按课程汇总','按课程汇总');click(c['input']['name']+'：1 节');expect(page.get_by_role('heading',name='课程明细：'+c['input']['name'],exact=True)).to_be_visible();expect(page.get_by_text(re.compile('金额线下计算'))).to_have_count(0);shot('14-total-detail')
 click('家庭干预');assert_peers('记录类型','机构课程','家庭干预','家庭干预');expect(page.get_by_role('button',name=re.compile(r'已完成\d+次'))).to_have_count(0);shot('17-family-calendar')
 assert not errors,errors
 (OUT/'checks.json').write_text(json.dumps({'synthetic':True,'screenshots':shots,'checks':['shared_priority','entry_colours','real_coverage_zero_and_nonzero','plan_knowledge','optional_summary_and_revision','minimal_lesson','three_observation_prompts','lesson_goal_method_link','grouped_goal_detail','method_goal_navigation','separate_library_tabs','review_decision_and_action','labelled_total_details','calendar_labels'],'errors':errors},ensure_ascii=False,indent=2)+'\n')
 print('PASS minimal lesson, three optional observation dimensions, real links, grouped detail, distinct library tabs, useful review/action and labelled totals',flush=True)
 browser.close()
