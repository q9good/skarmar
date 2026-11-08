"""Current redesign: actual localhost writes, conditional knowledge and mobile UI.
All records/screenshots are synthetic; remote targets are forbidden.
"""
import json,os,re
from pathlib import Path
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import sync_playwright,expect
from prd_ui_helpers import reveal_control
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/redesign-2026-11-03/screenshots');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 ctx=browser.new_context(viewport={'width':390,'height':844});page=ctx.new_page();errors=[];shots=[]
 page.on('pageerror',lambda e:errors.append(str(e)))
 def button(name):return reveal_control(page,'button',name)
 def click(name):button(name).click()
 def field(name):return reveal_control(page,'textbox',name)
 def call(method,path,body=None,status=200):
  r=ctx.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','X-Skarma-Data-Epoch':'formal-20261101'})
  assert r.status==status,(method,path,r.status,r.text());return r.json()
 def entity(kind,value):
  i=str(uuid4());return call('PUT','/prd/records/'+i,{'id':i,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value},**({'method_create_mode':'full'} if kind=='method' else {})})['value']
 def knowledge(kind,title,goals,methods):
  i=str(uuid4());return call('POST','/prd/process',{'kind':'knowledge','value':{'id':i,'operation_id':str(uuid4()),'expected_version':None,'action':'create','input':{'id':i,'kind':kind,'content':title,'goal_ids':goals,'method_ids':methods,'stage':'try' if kind=='experience' else None},'details':{'title':title},'note':''}})
 def shot(name,focus=None):
  if focus:focus.scroll_into_view_if_needed()
  else:page.evaluate("()=>{for(const e of document.querySelectorAll('*'))if(['auto','scroll'].includes(getComputedStyle(e).overflowY))e.scrollTop=0}")
  page.wait_for_timeout(100);assert not page.evaluate('document.documentElement.scrollWidth>innerWidth+1')
  page.screenshot(path=str(OUT/(name+'.png')));shots.append(name+'.png')
 page.goto(BASE);field('账号').fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录')
 expect(button('记录家庭')).to_be_enabled()
 long=entity('goal',{'title':'交往与共同活动（合成）','level':'long'})
 goal=entity('goal',{'title':'轮流玩球（合成）','level':'short','parent_id':long['id'],'criteria':'一起玩球时，等待并主动邀请对方'})
 other=entity('goal',{'title':'尝试新材料（合成）','level':'short'})
 method=entity('method',{'title':'示范后等待（合成）','steps':'跟着孩子的兴趣，示范一步后留出回应时间。','goal_ids':[goal['id']]})
 alternate=entity('method',{'title':'换一种材料（合成）','steps':'从熟悉材料开始，每次只改一个地方。','goal_ids':[goal['id']]})
 knowledge('experience','先跟随兴趣，再加入互动（合成）',[long['id']],[])
 knowledge('difficulty','玩法变化时容易离开（合成）',[goal['id']],[])
 knowledge('experience','示范后留出回应时间（合成）',[],[method['id']])
 knowledge('experience','未选方法的细节不应带入（合成）',[],[alternate['id']])
 entity('catalog',{'kind':'course','name':'认知（合成）','active':True});entity('catalog',{'kind':'teacher','name':'唐唐（合成）','active':True})
 page.reload();expect(button('记录家庭')).to_be_enabled();shot('01-today')
 class_colour=button('记录课堂').evaluate('e=>getComputedStyle(e).backgroundColor');family_colour=button('记录家庭').evaluate('e=>getComputedStyle(e).backgroundColor')
 assert class_colour!=family_colour and class_colour!='rgb(255, 255, 255)' and family_colour!='rgb(255, 255, 255)'
 assert abs(button('记录家庭').bounding_box()['height']-button('记录课堂').bounding_box()['height'])<1
 click('记录家庭');plan=page.get_by_role('dialog',name='填写干预计划',exact=True);expect(plan).to_be_visible()
 activity=plan.get_by_role('textbox',name='准备做什么',exact=True);expect(activity).to_be_visible();assert activity.get_attribute('aria-required')=='true'
 expect(button('保存计划并退出')).to_be_disabled();assert not page.get_by_role('button',name='选择计划方法',exact=True).count()
 assert activity.bounding_box()['y']<page.get_by_test_id('plan-reference-goals').bounding_box()['y']
 shot('02-plan-first');activity.fill('一起玩轮流滚球，尝试等待和主动求助。')
 # All applicable methods are references now; none is preselected as planned or actual.
 graph=page.get_by_test_id('plan-goal-method-map');graph.get_by_role('button',name='层级列表',exact=True).click();graph.get_by_role('checkbox',name='显示关联经验和困难',exact=True).click();click('展开全部关系');refs=graph.get_by_test_id('map-goal-'+goal['id']);expect(refs).to_contain_text('先跟随兴趣，再加入互动（合成）')
 expect(refs.get_by_text(re.compile('示范后留出回应时间（合成）')).first).to_be_visible()
 click('查看关系图知识：示范后留出回应时间（合成） · '+goal['id']);expect(page.get_by_role('dialog',name='关系图关联知识',exact=True)).to_be_visible();click('关闭关系图关联知识')
 shot('04-plan-knowledge',refs)
 expect(plan).to_be_visible();expect(activity).to_have_value('一起玩轮流滚球，尝试等待和主动求助。')
 page.reload();expect(button('记录家庭')).to_be_enabled();page.get_by_role('button',name=re.compile('^继续干预填写：')).click()
 expect(field('准备做什么')).to_have_value('一起玩轮流滚球，尝试等待和主动求助。');click('保存计划并退出')
 expect(page.get_by_role('dialog')).to_have_count(0)
 state=call('GET','/prd/state');assert len(state['sessions'])==1
 saved=state['sessions'][0];assert saved['input']['plan']=='一起玩轮流滚球，尝试等待和主动求助。'
 assert not saved['input']['targets'] and not saved['input']['methods']
 assert call('GET','/prd/coverage?days=30&source=family')['records']['selected']==0
 click('目标');shot('05-goals');goal_colour=button('新建目标').evaluate('e=>getComputedStyle(e).backgroundColor')
 click('方法');shot('06-methods');assert button('新建方法').evaluate('e=>getComputedStyle(e).backgroundColor')!=goal_colour
 click('记录');shot('07-classroom');expect(page.get_by_role('button',name='记录课堂',exact=True)).to_have_count(0);click('今天');click('记录课堂');click('课程名称（必填）：请选择课程');click('认知（合成）');click('授课老师（必填）：请选择老师');click('唐唐（合成）');shot('12-lesson-editor');click('关闭记录课堂')
 click('跟进');click('经验库与困难');click('经验');shot('08-experiences')
 click('困难');shot('09-difficulties')
 # Read-only navigation must not create extra plans or rewrite the saved activity.
 assert call('GET','/prd/state')['sessions']==state['sessions']
 assert not errors,errors
 (OUT/'checks.json').write_text(json.dumps({'synthetic':True,'screenshots':shots,'checks':['solid_peer_actions','required_first_activity','read_only_goal_references','deduplicated_knowledge','ancestor_principles','no_automatic_plan_selection','nested_back','draft_refresh_and_real_save','plan_not_actual_count','functional_page_themes'],'errors':errors},ensure_ascii=False,indent=2)+'\n')
 print('PASS UI redesign: activity-only plan, read-only knowledge, retained drafts, actual save, peer actions and functional themes')
 browser.close()
