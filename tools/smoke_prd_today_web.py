"""R01 Today business date, first-screen priority and cross-account saved-record metadata."""
from prd_ui_helpers import reveal_control
import os,re
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import expect,sync_playwright

BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}

with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 owner_context=browser.new_context(viewport={'width':360,'height':640},timezone_id='America/Los_Angeles')
 owner=owner_context.new_page()
 errors=[]
 owner.on('pageerror',lambda error:errors.append(str(error)))
 def button(page,name):
  return page.get_by_role('button',name=name,exact=True)
 def login(page,user,password):
  page.goto(BASE);reveal_control(page,'textbox','账号').fill(user);page.get_by_label('密码',exact=True).fill(password);button(page,'登录').click();expect(button(page,'记录课堂')).to_be_enabled()
 def call(context,method,path,body=None,status=200):
  response=context.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'})
  assert response.status==status,(response.status,response.text())
  return response.json()
 def entity(context,kind,value):
  identifier=str(uuid4())
  body={'id':identifier,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value}};body.update({'method_create_mode':'quick'} if kind=='method' else {});return call(context,'PUT','/prd/records/'+identifier,body)['value']
 def first_screen():
  lesson=button(owner,'记录课堂');family=owner.get_by_role('button',name=re.compile(r'^(记录家庭|记录家庭：填写今天的总结)$'))
  for item in [lesson,family]:
   expect(item).to_be_in_viewport(ratio=1)
   box=item.bounding_box();assert box['width']>=44 and box['height']>=44
  assert lesson.bounding_box()['y']<family.bounding_box()['y']
  assert not owner.evaluate('document.documentElement.scrollWidth > innerWidth')

 login(owner,'owner','synthetic-prd-ui-password')
 expected=owner.evaluate("""()=>{
  const parts=new Intl.DateTimeFormat('en-CA',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).formatToParts(new Date());
  const part=name=>parts.find(value=>value.type===name).value;
  const date=part('year')+'-'+part('month')+'-'+part('day');
  return date+' '+['周日','周一','周二','周三','周四','周五','周六'][new Date(date+'T00:00:00Z').getUTCDay()];
 }""")
 expect(owner.get_by_label('家庭业务日期',exact=True)).to_have_text(expected)
 first_screen()

 long_goals=[]
 for count in [3,6]:
  while len(long_goals)<count:
   long_goals.append(entity(owner_context,'goal',{'title':'合成长线'+str(len(long_goals)+1),'level':'long'}))
  owner.reload();expect(button(owner,'记录课堂')).to_be_enabled();first_screen()
 expect(button(owner,'查看长期关注')).to_contain_text('另有 3 项')

 member=call(owner_context,'POST','/auth/users',{'username':'family','display_name':'测试家人','password':'s','role':'member'})
 member_context=browser.new_context(viewport={'width':430,'height':932},timezone_id='Asia/Tokyo')
 member_page=member_context.new_page();login(member_page,'family','s')
 today=expected[:10]
 goal=entity(member_context,'goal',{'title':'合成当天目标','level':'short'})
 course=entity(member_context,'catalog',{'kind':'course','name':'合成语言课','active':True})
 teacher=entity(member_context,'catalog',{'kind':'teacher','name':'合成王老师','active':True})
 lesson_id=str(uuid4())
 call(member_context,'PUT','/prd/lessons/'+lesson_id,{'id':lesson_id,'operation_id':str(uuid4()),'expected_version':None,'action':{'kind':'observation','value':{'business_date':today,'course_id':course['id'],'teacher_id':teacher['id'],'notes':'合成课堂观察'}}})
 call(member_context,'PUT','/prd/lessons/'+lesson_id,{'id':lesson_id,'operation_id':str(uuid4()),'expected_version':1,'action':{'kind':'organization','value':{'seen_observation_version':1,'targets':[{'goal_id':goal['id'],'note':'属于其他目标的课堂表现'}],'focus_goal_ids':[long_goals[0]['id']]}}})
 session_id=str(uuid4())
 call(member_context,'PUT','/prd/sessions/'+session_id,{'id':session_id,'operation_id':str(uuid4()),'expected_version':None,'action':'complete','input':{'business_date':today,'time_slot':'晚上','focus_goal_ids':[long_goals[0]['id']],'summary':{'targets':[{'goal_id':goal['id'],'performed':True,'observation':'合成家庭观察'}],'confirmation':'none','signals':[{'kind':'主动开始','goal_ids':[long_goals[0]['id']]},{'kind':'投入愉快','goal_ids':[]}]}}})
 moment_id=str(uuid4())
 call(member_context,'POST','/prd/process',{'kind':'observe','value':{'id':moment_id,'operation_id':str(uuid4()),'expected_version':None,'kind':'moment','action':'save','input':{'business_date':today,'text':'合成瞬间观察','goal_ids':[goal['id']]},'note':''}})

 # A normal shared refresh reloads the existing state and observation endpoints; no extra poller is introduced.
 owner.reload();expect(button(owner,'记录课堂')).to_be_enabled();first_screen()
 expect(owner.get_by_role('heading',name='今天已记录',exact=True)).to_be_visible()
 expect(owner.get_by_text('合成语言课 · 合成王老师',exact=True)).to_be_visible()
 expect(owner.get_by_text('合成家庭观察',exact=True)).to_be_visible()
 expect(owner.get_by_text('合成瞬间观察',exact=True)).to_be_visible()
 expect(owner.get_by_text(re.compile(r'^机构课程 · \d{2}:\d{2}$'))).to_be_visible()
 expect(owner.get_by_text('家庭干预 · 晚上',exact=True)).to_be_visible()
 expect(owner.get_by_text(re.compile(r'^瞬间观察 · \d{2}:\d{2}$'))).to_be_visible()
 expect(owner.get_by_text('记录者：测试家人 · 刚保存 · 已同步',exact=True)).to_have_count(3)
 assert member['display_name']=='测试家人'
 # A long focus flag is not an observation about every other target in a record.
 button(owner,'查看长期关注').click()
 directions=owner.get_by_role('dialog',name='长期关注',exact=True)
 expect(directions).to_be_visible()
 expect(directions.get_by_text(re.compile(r'^最近家庭观察：.*合成家庭观察'))).to_have_count(0)
 expect(directions.get_by_text(re.compile(r'^最近课堂观察：.*合成课堂观察'))).to_have_count(0)
 expect(directions.get_by_text(re.compile(r'^最近家庭观察：.*明确关注过这个方向，未单独填写文字观察'))).to_have_count(1)
 expect(directions.get_by_text(re.compile(r'^最近课堂观察：.*明确关注过这个方向，未单独填写文字观察'))).to_have_count(1)
 expect(directions.get_by_text('明确关联的参与信号：主动开始',exact=True)).to_be_visible()
 expect(directions.get_by_text('明确关联的参与信号：投入愉快',exact=True)).to_have_count(0)
 button(owner,'关闭长期关注').click()

 # Two pending plans require an explicit selection; opening one edits only a
 # local draft and cannot complete or overwrite either shared server record.
 plans=[]
 for slot,name in [('上午','合成待总结A'),('晚上','合成待总结B')]:
  identifier=str(uuid4())
  plans.append(call(member_context,'PUT','/prd/sessions/'+identifier,{'id':identifier,'operation_id':str(uuid4()),'expected_version':None,'action':'save_draft','input':{'business_date':today,'time_slot':slot,'plan':name,'targets':[{'goal_id':goal['id'],'role':'main'}]}}))
 owner.reload();expect(button(owner,'记录家庭：填写今天的总结')).to_be_enabled()
 button(owner,'记录家庭：填写今天的总结').click()
 choice=owner.get_by_role('dialog',name='家庭干预',exact=True)
 expect(choice.get_by_role('button',name=re.compile('^继续总结：'))).to_have_count(2)
 button(owner,'继续总结：上午 · 合成待总结A').click()
 expect(owner.get_by_role('dialog',name='填写干预总结',exact=True)).to_be_visible()
 identity=call(owner_context,'GET','/auth/me')
 owner.wait_for_function("({actor,id})=>{const k=Object.keys(localStorage).find(k=>k.includes(':'+actor+':')&&k.endsWith('session-drafts:v1'));return k&&JSON.parse(localStorage.getItem(k))[id]?.stage==='summary';}",arg={'actor':identity['user']['id'],'id':plans[0]['id']})
 button(owner,'关闭填写干预总结').click()
 state=call(owner_context,'GET','/prd/state')
 for plan in plans:assert next(s for s in state['sessions'] if s['id']==plan['id'])==plan
 assert not errors,errors
 print('PASS Shanghai date, compact actions, author metadata, scoped long-focus evidence and explicit two-plan selection without shared writes',flush=True)
 browser.close()
