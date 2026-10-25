"""R01 Today business date, first-screen priority and cross-account saved-record metadata."""
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
  page.goto(BASE);page.get_by_role('textbox',name='账号',exact=True).fill(user);page.get_by_label('密码',exact=True).fill(password);button(page,'登录').click();expect(button(page,'记录上课')).to_be_enabled()
 def call(context,method,path,body=None,status=200):
  response=context.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'})
  assert response.status==status,(response.status,response.text())
  return response.json()
 def entity(context,kind,value):
  identifier=str(uuid4())
  return call(context,'PUT','/prd/records/'+identifier,{'id':identifier,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value}})['value']
 def first_screen():
  lesson=button(owner,'记录上课');family=owner.get_by_role('button',name=re.compile(r'^(记录一次家庭干预|填写今天的总结)$'))
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
  owner.reload();expect(button(owner,'记录上课')).to_be_enabled();first_screen()
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
 session_id=str(uuid4())
 call(member_context,'PUT','/prd/sessions/'+session_id,{'id':session_id,'operation_id':str(uuid4()),'expected_version':None,'action':'complete','input':{'business_date':today,'time_slot':'晚上','summary':{'targets':[{'goal_id':goal['id'],'performed':True,'observation':'合成家庭观察'}],'confirmation':'none'}}})
 moment_id=str(uuid4())
 call(member_context,'POST','/prd/process',{'kind':'observe','value':{'id':moment_id,'operation_id':str(uuid4()),'expected_version':None,'kind':'moment','action':'save','input':{'business_date':today,'text':'合成瞬间观察','goal_ids':[goal['id']]},'note':''}})

 # A normal shared refresh reloads the existing state and observation endpoints; no extra poller is introduced.
 owner.reload();expect(button(owner,'记录上课')).to_be_enabled();first_screen()
 expect(owner.get_by_role('heading',name='今天已记录',exact=True)).to_be_visible()
 expect(owner.get_by_text('合成语言课 · 合成王老师',exact=True)).to_be_visible()
 expect(owner.get_by_text('合成家庭观察',exact=True)).to_be_visible()
 expect(owner.get_by_text('合成瞬间观察',exact=True)).to_be_visible()
 expect(owner.get_by_text(re.compile(r'^机构课程 · \d{2}:\d{2}$'))).to_be_visible()
 expect(owner.get_by_text('家庭干预 · 晚上',exact=True)).to_be_visible()
 expect(owner.get_by_text(re.compile(r'^瞬间观察 · \d{2}:\d{2}$'))).to_be_visible()
 expect(owner.get_by_text('记录者：测试家人 · 刚保存 · 已同步',exact=True)).to_have_count(3)
 assert member['display_name']=='测试家人'
 assert not errors,errors
 print('PASS Shanghai business date, 0/3/6 first-screen actions, unified lesson/family/moment metadata and cross-account author refresh',flush=True)
 browser.close()
