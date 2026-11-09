"""Seven explicit user requirements, synthetic localhost, including the Followup entry path."""
import os,json,re
from pathlib import Path
from uuid import uuid4
from urllib.parse import urlparse
from playwright.sync_api import sync_playwright,expect
from prd_ui_helpers import reveal_control
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/iteration-2026-11-09/screenshots');OUT.mkdir(parents=True,exist_ok=True);(OUT.parent/'checks.json').unlink(missing_ok=True)
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox']);ctx=browser.new_context(viewport={'width':390,'height':844});a=ctx.new_page();errors=[];shots=[];results={};a.on('pageerror',lambda e:errors.append(str(e)))
 def click(n,page=a):reveal_control(page,'button',n).click()
 def field(n,page=a):return reveal_control(page,'textbox',n)
 def call(method,path,body=None,context=ctx,status=200):
  r=context.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','X-Skarma-Data-Epoch':'formal-20261101'});assert r.status==status,(path,r.status,r.text());return r.json()
 def entity(kind,value):
  i=str(uuid4());return call('PUT','/prd/records/'+i,{'id':i,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value},**({'method_create_mode':'full'} if kind=='method' else {})})['value']
 def state():return call('GET','/prd/state')
 def shot(name,focus=None):
  if focus is not None:focus.scroll_into_view_if_needed()
  assert not a.evaluate('document.documentElement.scrollWidth>innerWidth+1');a.screenshot(path=str(OUT/(name+'.png')));shots.append(name+'.png')
 def ready():expect(a.get_by_role('button',name='记录课堂',exact=True)).to_be_visible()
 a.goto(BASE);field('账号').fill('owner');a.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录');ready();today=a.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai'}).format(new Date())")
 g=entity('goal',{'title':'合成互动方向','level':'long','priority_score':60});g2=entity('goal',{'title':'合成学习支持','level':'long','priority_score':55});short=entity('goal',{'title':'合成等待轮流','level':'short','parent_id':g['id'],'priority_score':50});m=entity('method',{'title':'合成示范与等待','steps':'先示范，留出回应时间','goal_ids':[g['id'],g2['id'],short['id']],'priority_score':60});m2=entity('method',{'title':'合成轮流玩法','steps':'一起选玩具，轮流尝试','goal_ids':[short['id']],'priority_score':50})
 c1=entity('catalog',{'kind':'course','name':'合成认知','active':True});c2=entity('catalog',{'kind':'course','name':'合成社交','active':True});teacher=entity('catalog',{'kind':'teacher','name':'合成唐老师','active':True})
 a.reload();ready();click('目标与方法');expect(a.get_by_test_id('relationship-browser')).to_be_visible();click('选择目标或方法作为入口');click('选择入口：合成互动方向');expect(a.get_by_test_id('relationship-current')).to_contain_text('合成互动方向');click('切换到方法：合成示范与等待');expect(a.get_by_test_id('relationship-current')).to_contain_text('合成示范与等待');expect(a.get_by_role('button',name='切换到目标：合成学习支持',exact=True)).to_be_visible();click('切换到目标：合成等待轮流');click('切换到方法：合成轮流玩法');click('返回上一入口');expect(a.get_by_test_id('relationship-current')).to_contain_text('合成等待轮流');shot('01-interactive-mobile',a.get_by_test_id('relationship-browser'))
 a.set_viewport_size({'width':320,'height':844});shot('02-interactive-320',a.get_by_test_id('relationship-current'));a.set_viewport_size({'width':390,'height':844});results['U01']='PASS: goal→method→another goal→method, Back, direct-only links, mobile 320/390 with no canvas panning'
 # Shared numeric policy, response loss and exact replay.
 click('数字优先级规则');field('基础分最大值').fill('200');field('每个关联目标加分').fill('5');field('关联加分上限').fill('12')
 def lose_policy(route):route.fetch();route.abort('failed')
 a.route('**/api/prd/process',lose_policy);click('保存优先级规则');expect(a.get_by_role('button',name='重试同一次规则保存',exact=True)).to_be_visible();a.unroute('**/api/prd/process',lose_policy);a.route('**/api/prd/state',lambda route:route.fulfill(status=503,json={'error':'合成暂时无法读取列表'}));click('重试同一次规则保存');expect(a.get_by_role('dialog')).to_have_count(0);a.unroute('**/api/prd/state');a.evaluate("window.dispatchEvent(new Event('focus'))");policy=state()['priority_policy'];assert policy['maximum']==200 and policy['method_per_goal']==5 and policy['version']==1
 click('切换到方法：合成示范与等待');expect(a.get_by_test_id('relationship-current')).to_contain_text('72（基础60＋关联12）');shot('03-numeric-association-priority',a.get_by_test_id('relationship-current'));results['U05']='PASS: numeric policy 0–200, relation add 5 capped 12, method60→72, saved shared policy, lost response replays once'
 call('POST','/auth/users',{'username':'second','display_name':'合成另一位家人','password':'s','role':'member'});second=browser.new_context(viewport={'width':390,'height':844});b=second.new_page();b.goto(BASE);field('账号',b).fill('second');b.get_by_label('密码',exact=True).fill('s');click('登录',b);click('目标与方法',b);click('数字优先级规则',b);expect(field('基础分最大值',b)).to_have_value('200');expect(field('关联加分上限',b)).to_have_value('12');second.close()
 # Both classes really recorded via UI/upload queue, with two stable items in the first.
 click('今天')
 def record(course,kind,contents):
  click('记录课堂');click('课程名称（必填）：请选择课程');click(course);click('授课老师（必填）：请选择老师');click('合成唐老师');click('补充观察');reveal_control(a,'checkbox',kind).click()
  for index,text in enumerate(contents,1):click('添加一条'+kind);field(kind+' '+str(index)).fill(text)
  shot('04-independent-observation-items' if course=='合成认知' else '05-second-class',field(kind+' 1'));click('提交记录');expect(a.get_by_role('dialog')).to_have_count(0);expect(a.get_by_role('button',name='补充课堂观察',exact=True)).to_be_visible(timeout=15000);click('完成')
 record('合成认知','老师交代的练习或建议',['合成第一件事：先等待准备信号','合成第二件事：调整材料后再试']);record('合成社交','我看到的好方法',['合成第三件事：轮流时先让他选材料'])
 lessons=state()['lessons'];l1=next(l for l in lessons if l['course']['id']==c1['id']);l2=next(l for l in lessons if l['course']['id']==c2['id']);original_items=l1['observation']['items'];assert len(original_items)==2 and len({i['id'] for i in original_items})==2
 click('查看课记：合成认知 · 合成唐老师');click('补充／修改课堂观察');field('老师交代的练习或建议 2').fill('合成第二件事：调整材料后再试，已经编辑');click('添加一条老师交代的练习或建议');field('老师交代的练习或建议 3').fill('合成临时第三条，应在保存前移除');click('移除老师交代的练习或建议 3');click('提交记录');expect(a.get_by_role('dialog')).to_have_count(0);l1=next(l for l in state()['lessons'] if l['id']==l1['id']);assert [i['id'] for i in l1['observation']['items']]==[i['id'] for i in original_items];assert len(l1['observation']['items'])==2 and '已经编辑' in l1['observation']['items'][1]['content'];original_items=l1['observation']['items']
 # Output entry points really create goal and method and return to organization.
 click('跟进');click('整理课堂材料：'+today+' · 合成认知');outputs=a.get_by_test_id('lesson-output-entrypoints');expect(outputs.get_by_role('button',name='根据观察新建目标',exact=True)).to_be_visible();expect(outputs.get_by_role('button',name='根据观察新建方法',exact=True)).to_be_visible();expect(outputs.get_by_role('button',name='记一条经验',exact=True)).to_be_visible();expect(outputs.get_by_role('button',name='记一条困难',exact=True)).to_be_visible();shot('06-four-output-entrypoints',outputs)
 click('根据观察新建目标');field('本领名称').fill('合成课堂提出的目标');reveal_control(a,'combobox','上级目标').select_option(g['id']);click('保存');expect(a.get_by_role('dialog',name='整理课堂记录',exact=True)).to_be_visible()
 newg=next(x for x in state()['goals'] if x['input']['title']=='合成课堂提出的目标');assert newg['input']['priority_until']>=today
 click('根据观察新建方法');field('方法名称').fill('合成课堂提出的方法');field('具体做法／必填').fill('先观察准备信号，再给一个步骤');click('选择方法适用目标（已选0）');reveal_control(a,'checkbox','合成课堂提出的目标').click();click('关闭选择方法适用目标');click('保存');expect(a.get_by_role('dialog',name='整理课堂记录',exact=True)).to_be_visible();assert next(l for l in state()['lessons'] if l['id']==l1['id'])['organization']['targets']==[]
 click('记一条经验');field('经验内容 1').fill('合成从第一条观察留下的想法');reveal_control(a,'combobox','经验阶段 1').select_option('idea');click('调整关联 1');reveal_control(a,'checkbox','合成互动方向').click();click('关闭调整经验或困难的关联');click('保存整理');expect(a.get_by_role('dialog')).to_have_count(0)
 derived=next(k for k in state()['knowledge'] if k['source_lesson_id']==l1['id']);assert derived['source_observation']['items']==original_items;results['U03']='PASS: four peer output actions, real new goal/method saves return to original organization; proposals do not create course coverage'
 # Two local organization drafts, exact Followup discard path and one aggregate entry.
 for course,text in [('合成认知','合成暂存整理一'),('合成社交','合成暂存整理二')]:
  click('整理课堂材料：'+today+' · '+course);field('处理说明（可选）').fill(text);click('关闭整理课堂记录')
 click('今天');expect(a.get_by_role('button',name='继续课记',exact=True)).to_have_count(1);click('继续课记');expect(a.get_by_role('button',name=re.compile('^继续：.*合成认知'))).to_have_count(1);expect(a.get_by_role('button',name=re.compile('^继续：.*合成社交'))).to_have_count(1);shot('07-one-continue-picker');click('关闭选择继续的课记')
 click('跟进');click('整理课堂材料：'+today+' · 合成认知');expect(field('处理说明（可选）')).to_have_value('合成暂存整理一');old_input=field('处理说明（可选）').element_handle();click('放弃这份本机草稿')
 a.evaluate("""()=>{const original=navigator.locks.request.bind(navigator.locks);window.restoreDraftLocks=()=>{navigator.locks.request=original;};navigator.locks.request=(name,fn)=>original(name,async lock=>{if(name.includes('lesson-drafts'))await new Promise(resolve=>window.releaseDraftDelete=resolve);return fn(lock)})}""")
 click('确认放弃未提交填写');expect(a.get_by_role('button',name='正在放弃…',exact=True)).to_be_visible();a.wait_for_function("typeof window.releaseDraftDelete==='function'");old_input.evaluate("el=>{const prototype=el.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype;Object.getOwnPropertyDescriptor(prototype,'value').set.call(el,'合成迟到的输入事件，不得复活已放弃草稿');el.dispatchEvent(new Event('input',{bubbles:true}))}");a.evaluate('window.releaseDraftDelete();window.restoreDraftLocks()');expect(a.get_by_role('dialog')).to_have_count(0)
 a.reload();ready();click('继续课记');expect(a.get_by_role('button',name=re.compile('^继续：.*合成认知'))).to_have_count(0);expect(a.get_by_role('button',name=re.compile('^继续：.*合成社交'))).to_have_count(1);click('关闭选择继续的课记');results['U07']='PASS: two real UI courses/organization drafts expose only one Continue entry, picker names courses, one discard leaves the other across refresh'
 click('跟进');click('关闭课堂材料：'+today+' · 合成认知');expect(a.get_by_role('dialog',name='关闭课堂待办',exact=True)).to_be_visible();shot('08-close-request-not-record')
 def lose_close(route):route.fetch();route.abort('failed')
 a.route('**/api/prd/lessons/'+l1['id'],lose_close);click('确认关闭本次待办');expect(a.get_by_role('button',name='重试同一次课记提交',exact=True)).to_be_visible();a.unroute('**/api/prd/lessons/'+l1['id'],lose_close);click('重试同一次课记提交');expect(a.get_by_role('dialog')).to_have_count(0);expect(a.get_by_role('button',name='整理课堂材料：'+today+' · 合成认知',exact=True)).to_have_count(0)
 after=next(l for l in state()['lessons'] if l['id']==l1['id']);assert after['status']=='recorded' and after['organization']['state']=='done' and after['organization_version']==3;assert after['observation']['items']==original_items
 updated={**after['observation'],'items':[original_items[0],{**original_items[1],'content':'合成更新第二件事'}]};call('PUT','/prd/lessons/'+l1['id'],{'id':l1['id'],'operation_id':str(uuid4()),'expected_version':after['observation_version'],'action':{'kind':'observation','value':updated}});a.evaluate("window.dispatchEvent(new Event('focus'))");expect(a.get_by_test_id('classroom-inbox-'+l1['id']).get_by_role('button',name=re.compile('^待处理课堂：'))).to_be_visible(timeout=10000)
 assert next(k for k in state()['knowledge'] if k['input']['id']==derived['input']['id'])['source_observation']['items']==original_items
 results['U02']='PASS: discard from Followup removes only chosen local input and ignores a late input during delayed persistence; close-request replays once, preserves lesson/knowledge, later observation reopens'
 results['U04']='PASS: two teacher observations added/edited/removed through UI, stable IDs and independent values survive; later edit keeps frozen old items; two classes stay two records'
 # New long focus and numeric values + temporary opt-out persist, plans still reference-only.
 click('目标');click('新建目标');field('本领名称').fill('合成新建两周方向');reveal_control(a,'combobox','目标类型').select_option('long');field('目标数字优先级').fill('180');expect(a.get_by_role('checkbox',name='临时优先展示两周',exact=True)).to_have_attribute('aria-checked','true');click('保存');expect(a.get_by_role('dialog')).to_have_count(0);click('今天');widget=a.get_by_role('button',name='查看长期关注',exact=True);expect(widget).to_contain_text('合成新建两周方向');expect(widget).to_contain_text('方法');shot('09-combined-today-focus',widget)
 click('查看长期关注');click('调整关注优先级：合成新建两周方向');expect(field('目标数字优先级')).to_have_value('180');reveal_control(a,'checkbox','临时优先展示两周').click();click('保存');expect(a.get_by_role('dialog')).to_have_count(0);assert next(g for g in state()['goals'] if g['input']['title']=='合成新建两周方向')['input'].get('priority_until') is None
 results['U06']='PASS: Today interleaves typed goals/methods by numeric scores, a new long goal has 14-day boost, can cancel without changing 180; date expiry separately unit-tested'
 click('记录家庭');field('准备做什么').fill('合成一起探索滚球，按孩子状态调整');plan=a.get_by_test_id('plan-reminders');expect(plan).to_be_visible();click('计划提醒目标：合成互动方向');expect(a.get_by_role('button',name='调整入口优先级',exact=True)).to_have_count(0);click('关闭目标参考');click('最近新增方法：合成示范与等待');expect(a.get_by_role('dialog',name='方法参考',exact=True)).to_be_visible();click('关闭方法参考');expect(field('准备做什么')).to_have_value('合成一起探索滚球，按孩子状态调整');click('保存计划并退出');expect(a.get_by_role('dialog')).to_have_count(0);assert not state()['sessions'][0]['input']['targets'] and not state()['sessions'][0]['input']['methods']
 assert not errors,errors
 (OUT.parent/'checks.json').write_text(json.dumps({'synthetic':True,'production_accessed':False,'deployed':False,'viewports':[320,390],'results':results,'screenshots':shots},ensure_ascii=False,indent=2)+'\n')
 for k,v in sorted(results.items()):print(k,v)
 browser.close()
