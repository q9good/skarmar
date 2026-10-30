"""Phone task order + retained optional input, through real local APIs only."""
import os,re
from pathlib import Path
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import expect,sync_playwright

BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/prd-simplified-ui');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 setup=browser.new_context();admin=setup.new_page();errors=[]
 def btn(page,name):return page.get_by_role('button',name=name,exact=True)
 def click(page,name):btn(page,name).click()
 def field(page,name):return page.get_by_role('textbox',name=name,exact=True)
 def login(page,user='owner'):
  page.goto(BASE);field(page,'账号').fill(user);page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click(page,'登录');expect(btn(page,'记录课堂')).to_be_enabled()
 def call(ctx,method,path,body=None):
  r=ctx.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'});assert r.ok,(r.status,r.text());return r.json()
 def entity(kind,value):
  id=str(uuid4());body={'id':id,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value}}
  if kind=='method':body['method_create_mode']='full'
  return call(setup,'PUT','/prd/records/'+id,body)['value']
 def shot(name):
  page.evaluate("()=>{for(const e of document.querySelectorAll('*'))if(['auto','scroll'].includes(getComputedStyle(e).overflowY))e.scrollTop=0;}");page.wait_for_timeout(100);page.screenshot(path=str(OUT/(name+'.png')))
 login(admin)
 call(setup,'POST','/auth/users',{'username':'family','display_name':'合成界面家人','password':'synthetic-prd-ui-password','role':'member'})
 goals=[entity('goal',{'title':'合成长期关注','level':'long'})]+[entity('goal',{'title':'合成本领'+str(i),'level':'short'}) for i in range(10)]
 course=entity('catalog',{'kind':'course','name':'合成课程','active':True});teacher=entity('catalog',{'kind':'teacher','name':'合成老师','active':True})
 for kind,content in [('difficulty','合成换材料时需要支持'),('experience','合成先示范并留出等待时间')]:
  id=str(uuid4());call(setup,'POST','/prd/process',{'kind':'knowledge','value':{'id':id,'operation_id':str(uuid4()),'expected_version':None,'action':'create','input':{'id':id,'kind':kind,'content':content,'goal_ids':[goals[1]['id']],'method_ids':[],'stage':'try' if kind=='experience' else None,'next_review_date':None},'details':{'title':content,'state':'pending' if kind=='difficulty' else 'active'},'derived_from':[],'note':''}})

 ctx=browser.new_context(viewport={'width':390,'height':844});page=ctx.new_page();page.on('pageerror',lambda e:errors.append(str(e)))
 login(page,'family');click(page,'目标')
 for width,height in [(390,844),(360,640),(1440,900)]:
  page.set_viewport_size({'width':width,'height':height})
  expect(btn(page,'新建目标')).to_be_in_viewport(ratio=1)
  first=page.locator('[data-testid^="prd-goal-"]').first
  expect(first.locator('div').first).to_be_in_viewport()
  expect(page.get_by_role('combobox',name='目标期限筛选',exact=True)).to_have_count(0)
  assert not page.evaluate('document.documentElement.scrollWidth>innerWidth')
 page.set_viewport_size({'width':390,'height':844});shot('goals')
 click(page,'筛选目标');page.get_by_role('combobox',name='目标期限筛选',exact=True).select_option('short');click(page,'筛选目标')
 expect(btn(page,'查看目标：合成长期关注')).to_have_count(0)
 click(page,'筛选目标');expect(page.get_by_role('combobox',name='目标期限筛选',exact=True)).to_have_value('short');click(page,'筛选目标')

 click(page,'方法');click(page,'新建方法')
 expect(field(page,'具体做法／必填')).to_be_in_viewport(ratio=1)
 expect(page.get_by_role('checkbox',name=goals[1]['input']['title'],exact=True)).to_have_count(0)
 field(page,'方法名称').fill('合成小步示范');field(page,'具体做法／必填').fill('先示范，再等待孩子尝试')
 click(page,'选择方法适用目标（已选0）');page.get_by_role('checkbox',name=goals[1]['input']['title'],exact=True).click();click(page,'关闭选择方法适用目标')
 click(page,'补充方法说明');field(page,'观察意图／可选').fill('观察是否主动再试');click(page,'补充方法说明');click(page,'补充方法说明')
 expect(field(page,'观察意图／可选')).to_have_value('观察是否主动再试');click(page,'补充方法说明')
 shot('method');click(page,'保存');expect(page.get_by_role('dialog')).to_have_count(0)
 method=call(ctx,'GET','/prd/state')['methods'][0];assert method['input']['goal_ids']==[goals[1]['id']] and method['input']['observe']=='观察是否主动再试'
 print('PASS compact goal first screens, retained filters and explicit method association/optional input',flush=True)

 click(page,'今天');click(page,'记录家庭');click(page,'选择本次目标')
 page.get_by_role('checkbox',name='选择目标：'+goals[1]['input']['title'],exact=True).click();click(page,'完成目标选择')
 expect(page.get_by_test_id('plan-coverage-'+goals[1]['id'])).to_be_visible()
 expect(page.get_by_text('留意困难：合成换材料时需要支持',exact=True)).to_be_visible();expect(page.get_by_text('可参考经验：合成先示范并留出等待时间',exact=True)).to_be_visible()
 expect(page.get_by_role('button',name=re.compile('^主要练：'))).to_have_count(0)
 field(page,'准备怎么做').fill('合成计划：一起尝试生活小步骤')
 expect(btn(page,'查看全部方向的近况')).to_be_visible();click(page,'更多干预操作');expect(btn(page,'使用说明与手机桌面入口')).to_have_count(0);click(page,'更多干预操作')
 click(page,'目标笔记与参考：'+goals[1]['input']['title']);field(page,'计划笔记：'+goals[1]['input']['title']).fill('合成笔记不会因折叠清空');click(page,'目标笔记与参考：'+goals[1]['input']['title'])
 shot('plan');click(page,'保存共享草稿并退出');expect(page.get_by_role('dialog')).to_have_count(0)
 saved=call(ctx,'GET','/prd/state')['sessions'][0];assert saved['status']=='planned' and saved['input']['targets'][0]['note']=='合成笔记不会因折叠清空'
 click(page,'记录家庭：填写今天的总结')
 for width,height in [(390,844),(360,640)]:
  page.set_viewport_size({'width':width,'height':height});expect(field(page,'实际表现：'+goals[1]['input']['title'])).to_be_in_viewport(ratio=1)
 page.set_viewport_size({'width':390,'height':844});shot('summary')
 expect(page.get_by_role('combobox',name='目标角色：'+goals[1]['input']['title'],exact=True)).to_have_count(0)
 expect(page.get_by_role('combobox',name='实际方法确认',exact=True)).to_have_value('unconfirmed')
 field(page,'实际表现：'+goals[1]['input']['title']).fill('合成表现：示范后主动再试了一次')
 page.get_by_role('combobox',name='实际方法确认',exact=True).select_option('unknown')
 click(page,'正式提交干预总结');expect(page.get_by_role('dialog')).to_have_count(0)
 saved=call(ctx,'GET','/prd/state')['sessions'][0]
 assert saved['status']=='completed' and saved['input']['summary']['confirmation']=='unknown' and not saved['actual_methods']
 print('PASS selected goals lead the plan; actual observation leads the summary on both phone sizes; no inferred method confirmation',flush=True)

 # Fill the day with existing records so the success message cannot rely on list length.
 today=saved['input']['business_date']
 for _ in range(4):
  id=str(uuid4());call(ctx,'PUT','/prd/lessons/'+id,{'id':id,'operation_id':str(uuid4()),'expected_version':None,'action':{'kind':'observation','value':{'business_date':today,'course_id':course['id'],'teacher_id':teacher['id']}}})
 page.reload();expect(btn(page,'记录课堂')).to_be_enabled();click(page,'记录课堂')
 click(page,'课程名称（必填）：请选择课程');click(page,'合成课程');click(page,'授课老师（必填）：请选择老师');click(page,'合成老师');click(page,'提交记录')
 expect(page.get_by_role('dialog')).to_have_count(0);expect(btn(page,'补充课堂观察')).to_be_in_viewport(ratio=1)
 expect(page.get_by_text(re.compile('^已记下：'+today))).to_be_in_viewport(ratio=1)
 shot('lesson-confirmed');click(page,'完成')

 click(page,'跟进');click(page,'经验库与困难');click(page,'新建经验')
 expect(page.get_by_role('dialog',name='记一条经验',exact=True)).to_be_visible()
 expect(field(page,'修改依据或原因')).to_have_count(0)
 expect(page.get_by_role('combobox',name='经验状态',exact=True)).to_have_count(0)
 field(page,'经验名称（必填）').fill('合成示范心得');field(page,'做法与依据').fill('合成熟悉场景中，示范后愿意尝试')
 click(page,'选择关联目标与方法');
 target=page.get_by_role('checkbox',name='关联目标：'+goals[1]['input']['title'],exact=True)
 target.click();click(page,'关闭关联目标与方法')
 page.get_by_role('combobox',name='经验阶段',exact=True).select_option('try')
 click(page,'补充条件、成人回应与比较口径');field(page,'依据和条件（可选）').fill('合成条件：熟悉材料');click(page,'补充条件、成人回应与比较口径')
 click(page,'保存跟进');expect(page.get_by_role('dialog')).to_have_count(0)
 k=next(k for k in call(ctx,'GET','/prd/state')['knowledge'] if k['details']['title']=='合成示范心得');assert k['details']['evidence']=='合成条件：熟悉材料' and k['input']['goal_ids']==[goals[1]['id']]

 click(page,'目标');click(page,'查看目标：'+goals[2]['input']['title']);click(page,'现在复盘这个对象')
 expect(page.get_by_role('combobox',name='本次复盘决定',exact=True)).to_be_in_viewport(ratio=1)
 expect(btn(page,'保存复盘')).to_be_visible();expect(btn(page,'保存并下一项')).to_have_count(0)
 shot('review')
 click(page,'复盘日程与家人感受');page.get_by_role('combobox',name='这次安排对家人的负担（可选）',exact=True).select_option('stretched');click(page,'复盘日程与家人感受')
 field(page,'复盘依据与说明').fill('合成主动回看，之后按原节奏观察');click(page,'保存复盘')
 expect(page.get_by_role('dialog',name='逐项复盘',exact=True)).to_have_count(0)
 reviews=call(ctx,'GET','/prd/process?view=reviews')['reviews'];assert reviews[0]['caregiver_state']=='stretched'
 assert not errors,errors
 print('PASS first-screen lesson confirmation, explicit experience stage and retained optional evidence; concise standalone review keeps caregiver input',flush=True)
 browser.close()
