"""R01–R06 user-feedback acceptance; disposable localhost only."""
import json,os,re
from pathlib import Path
from uuid import uuid4
from urllib.parse import urlparse
from playwright.sync_api import sync_playwright,expect
from prd_ui_helpers import reveal_control
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/review-fixes-2026-11-08/screenshots');OUT.mkdir(parents=True,exist_ok=True);(OUT.parent/'checks.json').unlink(missing_ok=True)
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 ctx=browser.new_context(viewport={'width':390,'height':844});ctx.add_init_script("localStorage.setItem('skarmar:device:large-text','1')")
 a=ctx.new_page();errors=[];shots=[];results={};a.on('pageerror',lambda e:errors.append(str(e)))
 def click(n,page=a):reveal_control(page,'button',n).click()
 def field(n,page=a):return reveal_control(page,'textbox',n)
 def call(method,path,body=None,status=200,context=ctx):
  r=context.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','X-Skarma-Data-Epoch':'formal-20261101'});assert r.status==status,(path,r.status,r.text());return r.json()
 def entity(kind,value):
  i=str(uuid4());return call('PUT','/prd/records/'+i,{'id':i,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value},**({'method_create_mode':'full'} if kind=='method' else {})})['value']
 def state():return call('GET','/prd/state')
 def snap(name,focus=None):
  if focus is not None:focus.scroll_into_view_if_needed()
  assert not a.evaluate('document.documentElement.scrollWidth>innerWidth+1')
  a.screenshot(path=str(OUT/(name+'.png')));shots.append(name+'.png')
 def ready():expect(a.get_by_role('button',name='记录课堂',exact=True)).to_be_visible()
 a.goto(BASE);field('账号').fill('owner');a.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录');ready()
 long=entity('goal',{'title':'合成共同交往','level':'long','long_priority':0});medium=entity('goal',{'title':'合成轮流活动','level':'medium','parent_id':long['id']});short=entity('goal',{'title':'合成等待一轮','level':'short','parent_id':medium['id']});other=entity('goal',{'title':'合成学习支持','level':'long','long_priority':1})
 method=entity('method',{'title':'合成示范再等待','steps':'先示范，再等待回应','goal_ids':[short['id'],other['id']],'priority':1})
 for kind,title,goals,methods,details in [('experience','合成方法相关经验',[],[method['id']],{}),('experience','合成其他目标私有经验',[other['id']],[],{}),('difficulty','合成已解决困难',[],[method['id']],{'state':'resolved'})]:
  kid=str(uuid4());call('POST','/prd/process',{'kind':'knowledge','value':{'id':kid,'operation_id':str(uuid4()),'expected_version':None,'action':'create','input':{'id':kid,'kind':kind,'content':title,'goal_ids':goals,'method_ids':methods,'stage':'try' if kind=='experience' else None},'details':{'title':title,**details}}})
 course=entity('catalog',{'kind':'course','name':'合成认知','active':True});teacher=entity('catalog',{'kind':'teacher','name':'合成唐老师','active':True})
 a.reload();ready();today=a.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai'}).format(new Date())")
 # R01: real recording, default full example, collapse, edit, server request contains only input.
 widget=a.get_by_role('button',name='查看长期关注',exact=True);expect(widget).to_contain_text('合成共同交往');expect(widget).to_contain_text('合成学习支持');assert widget.inner_text().index('合成学习支持')<widget.inner_text().index('合成共同交往')
 click('记录课堂');click('课程名称（必填）：请选择课程');click('合成认知');click('授课老师（必填）：请选择老师');click('合成唐老师');click('补充观察');reveal_control(a,'checkbox','老师交代的练习或建议').click();click('添加一条老师交代的练习或建议')
 example=a.get_by_text(re.compile('合成示例.*目标是.*老师猜测.*逐步增加步骤'))
 expect(example).to_be_visible();expect(field('老师交代的练习或建议')).to_have_value('');snap('01-full-classroom-example',example)
 click('收起填写示例：老师交代的练习或建议');expect(example).to_have_count(0);click('查看填写示例：老师交代的练习或建议');expect(example).to_be_visible()
 field('老师交代的练习或建议').fill('合成观察：老师先示范，再等孩子回应。')
 click('提交记录');expect(a.get_by_role('dialog')).to_have_count(0);expect(a.get_by_role('button',name='补充课堂观察',exact=True)).to_be_visible(timeout=15000)
 lesson=state()['lessons'][0];lid=lesson['id'];assert lesson['observation']['teacher_notes']=='合成观察：老师先示范，再等孩子回应。'
 results['R01']='PASS: full wrapping example defaults open in large-text mode, toggles, stays out of saved value'
 # R02: an edit to an outbox-confirmed lesson must still have a Continue entry.
 click('补充课堂观察');field('老师交代的练习或建议').fill('合成尚未提交的草稿');click('关闭记录课堂');click('继续课记：合成认知');click('放弃这份本机草稿');expect(a.get_by_role('dialog',name='确认放弃课记草稿',exact=True)).to_be_visible();snap('02-visible-discard-confirmation')
 click('继续填写');expect(field('老师交代的练习或建议')).to_have_value('合成尚未提交的草稿');click('放弃这份本机草稿')
 a.evaluate("""()=>{const old=Storage.prototype.setItem;window.restoreStorage=()=>{Storage.prototype.setItem=old};Storage.prototype.setItem=function(k,v){if(k.includes('lesson-drafts:v1'))throw new DOMException('Synthetic failure','QuotaExceededError');return old.call(this,k,v)}}""")
 click('确认放弃未提交填写');expect(a.get_by_role('dialog',name='确认放弃课记草稿',exact=True).get_by_text(re.compile('草稿未能清除，填写仍保留'))).to_be_visible();a.evaluate('window.restoreStorage()');click('继续填写');expect(field('老师交代的练习或建议')).to_have_value('合成尚未提交的草稿')
 click('放弃这份本机草稿');click('确认放弃未提交填写');expect(a.get_by_role('dialog')).to_have_count(0);expect(a.get_by_role('button',name='继续课记：合成认知',exact=True)).to_have_count(0)
 a.reload();ready();expect(a.get_by_role('button',name='继续课记：合成认知',exact=True)).to_have_count(0);assert state()['lessons'][0]['observation']==lesson['observation']
 results['R02']='PASS: Continue after confirmed creation, cancel retains, failure keeps text, explicit retry removes draft across refresh without server edits'
 # R03: actual institution coverage is optional, hierarchical and source-separated.
 click('整理：合成认知');expect(a.get_by_role('button',name=re.compile('^本课实际覆盖'))).to_be_visible();click(re.compile('^本课实际覆盖'))
 click('展开全部目标');reveal_control(a,'checkbox','选择目标：合成等待一轮').click();expect(a.get_by_role('checkbox',name='选择目标：合成轮流活动',exact=True)).to_have_attribute('aria-checked','mixed');reveal_control(a,'checkbox','选择目标：合成共同交往').click();reveal_control(a,'checkbox','实际方法：合成示范再等待').click();snap('03-explicit-institution-coverage',a.get_by_text('本课实际使用的方法（可不选）',exact=True))
 click('保存整理');expect(a.get_by_role('dialog')).to_have_count(0)
 def counts():return {g['id']:g['counts'] for g in call('GET','/prd/coverage?days=30&source=combined')['goals']}
 c=counts();assert c[short['id']]['lesson']==1 and c[short['id']]['family']==0 and c[short['id']]['selected']==1,c;assert c[medium['id']]['lesson']==1 and c[long['id']]['lesson']==1,c
 saved=next(l for l in state()['lessons'] if l['id']==lid);assert saved['organization']['methods'][0]['id']==method['id']
 click('目标');reveal_control(a,'combobox','目标覆盖来源').select_option('lesson');row=a.get_by_role('button',name='查看总览目标：合成等待一轮',exact=True);expect(row).to_contain_text('1 次');expect(row).to_contain_text('家庭0／机构1');snap('09-institution-coverage-visible',a.get_by_test_id('goal-overview'));reveal_control(a,'combobox','目标覆盖来源').select_option('combined');expect(row).to_contain_text('1 次');reveal_control(a,'combobox','目标覆盖来源').select_option('family');expect(row).to_contain_text('0 次');click('今天')
 click('整理：合成认知');click('保存整理');expect(a.get_by_role('dialog')).to_have_count(0);assert counts()[short['id']]['lesson']==1
 click('整理：合成认知');reveal_control(a,'checkbox','选择目标：合成等待一轮').click();click('保存整理');expect(a.get_by_role('dialog')).to_have_count(0);report=call('GET','/prd/coverage?days=30&source=combined');assert next(g for g in report['goals'] if g['id']==long['id'])['linked']['lesson']==1
 click('整理：合成认知');reveal_control(a,'checkbox','选择目标：合成共同交往').click();reveal_control(a,'checkbox','实际方法：合成示范再等待').click();click('保存整理');expect(a.get_by_role('dialog')).to_have_count(0);assert counts()[short['id']]['lesson']==0 and counts()[long['id']]['lesson']==0
 results['R03']='PASS: Today optional actual goals/methods, inherited parents, persisted lesson/combined counts, family unchanged, repeat no duplicate, unlink clears coverage'
 # R04/R05: actual graph node + priority actions, persistence, one shared method and preserved plan.
 click('目标与方法');graph=a.get_by_test_id('goal-method-map');click('全局图');expect(graph.get_by_test_id('relationship-diagram')).to_be_visible();a.evaluate("()=>{for(const e of document.querySelectorAll('*'))if(['auto','scroll'].includes(getComputedStyle(e).overflowY))e.scrollTop=0}");snap('10-diagram-initial-screen');click('展开全部关系');expect(graph.get_by_test_id('flow-node-method-'+method['id'])).to_have_count(1)
 snap('04-mobile-node-arrow-diagram',graph.get_by_test_id('relationship-diagram'))
 reveal_control(a,'checkbox','显示关联经验和困难').click();click('关系方法：合成示范再等待');node=a.get_by_role('dialog',name='关系节点',exact=True);expect(node).to_contain_text('合成方法相关经验');expect(node).not_to_contain_text('合成已解决困难');click('关闭关系节点');click('关系目标：合成等待一轮');node=a.get_by_role('dialog',name='关系节点',exact=True);expect(node).to_contain_text('合成方法相关经验');expect(node).not_to_contain_text('合成其他目标私有经验');click('关闭关系节点')
 click('关系目标：合成共同交往');click('调整优先级：合成共同交往');expect(a.get_by_role('dialog',name='调整优先级',exact=True)).to_be_visible();field('目标数字优先级').fill('100');snap('05-direct-priority-control');click('保存');expect(a.get_by_role('dialog')).to_have_count(0);assert next(g for g in state()['goals'] if g['id']==long['id'])['input']['priority_score']==100
 click('关系方法：合成示范再等待');click('调整优先级：合成示范再等待');field('方法基础优先级').fill('0')
 def lose_priority_response(route):
  route.fetch();route.abort('failed')
 a.route('**/api/prd/records/'+method['id'],lose_priority_response);click('保存');expect(a.get_by_role('button',name='重试同一次保存',exact=True)).to_be_visible();a.unroute('**/api/prd/records/'+method['id'],lose_priority_response);click('重试同一次保存');expect(a.get_by_role('dialog')).to_have_count(0)
 assert next(m for m in state()['methods'] if m['id']==method['id'])['version']==method['version']+1
 call('POST','/auth/users',{'username':'reviewer','display_name':'合成第二位家人','password':'s','role':'member'})
 otherctx=browser.new_context(viewport={'width':390,'height':844});b=otherctx.new_page();b.goto(BASE);field('账号',b).fill('reviewer');b.get_by_label('密码',exact=True).fill('s');click('登录',b);click('目标与方法',b);click('全局图',b);click('展开全部关系',b);click('关系方法：合成示范再等待',b);expect(b.get_by_role('dialog',name='关系节点',exact=True)).to_contain_text('基础 0');click('关闭关系节点',b)
 click('关系方法：合成示范再等待');click('调整优先级：合成示范再等待');field('方法基础优先级').fill('50')
 current=next(m for m in state()['methods'] if m['id']==method['id'])
 call('PUT','/prd/records/'+method['id'],{'id':method['id'],'operation_id':str(uuid4()),'expected_version':current['version'],'method_change':'correction','change_note':'合成并发调整','mutation':{'kind':'method','value':{**current['input'],'priority_score':100}}},context=otherctx)
 click('保存');expect(a.get_by_text('另一位家人已修改这条记录，你的填写仍保留。',exact=True)).to_be_visible();expect(a.get_by_role('textbox',name='方法基础优先级',exact=True)).to_have_value('50');assert next(m for m in state()['methods'] if m['id']==method['id'])['input']['priority_score']==100
 click('放弃这份填写');click('确认放弃这份填写');expect(a.get_by_role('dialog')).to_have_count(0)
 current=next(m for m in state()['methods'] if m['id']==method['id']);call('PUT','/prd/records/'+method['id'],{'id':method['id'],'operation_id':str(uuid4()),'expected_version':current['version'],'method_change':'correction','change_note':'合成恢复低关注','mutation':{'kind':'method','value':{**current['input'],'priority_score':0}}})
 otherctx.close()
 assert next(m for m in state()['methods'] if m['id']==method['id'])['input']['priority_score']==0
 a.reload();ready();widget=a.get_by_role('button',name='查看长期关注',exact=True);expect(widget).to_contain_text('合成共同交往');expect(widget).to_contain_text('合成学习支持');assert widget.inner_text().index('合成共同交往')<widget.inner_text().index('合成学习支持');click('目标与方法');click('全局图');click('展开全部关系');click('关系方法：合成示范再等待');expect(a.get_by_role('dialog',name='关系节点',exact=True)).to_contain_text('基础 0');click('关闭关系节点')
 click('方法');expect(a.get_by_role('button',name='调整优先级：合成示范再等待',exact=True)).to_be_visible();click('编辑方法：合成示范再等待');expect(a.get_by_role('textbox',name='方法基础优先级',exact=True)).to_be_visible();click('保留草稿并退出')
 click('今天');click('记录家庭');field('准备做什么').fill('合成一起玩滚球，按孩子状态调整');plan=a.get_by_test_id('plan-reminders');expect(plan).to_be_visible();click('最近新增方法：合成示范再等待');expect(a.get_by_role('dialog',name='方法参考',exact=True)).to_be_visible();expect(a.get_by_role('button',name='调整优先级：合成示范再等待',exact=True)).to_have_count(0);click('关闭方法参考');expect(field('准备做什么')).to_have_value('合成一起玩滚球，按孩子状态调整');click('保存计划并退出');expect(a.get_by_role('dialog')).to_have_count(0)
 assert not state()['sessions'][0]['input']['targets'] and not state()['sessions'][0]['input']['methods']
 for width in [320,390,1440]:
  a.set_viewport_size({'width':width,'height':900 if width>1000 else 844});click('目标与方法');click('全局图');click('横向' if width>1000 else '纵向');click('展开全部关系');snap('06-diagram-'+str(width),a.get_by_test_id('relationship-diagram'));assert a.get_by_test_id('relationship-diagram').locator('svg path[marker-end]').count()>0
 results['R04']='PASS: direct priority dialog from real goal/method graph nodes and maintenance list; native edit field visible, ranking and second family visible; lost response retries once, concurrent edit blocks overwrite, refresh retains'
 results['R05']='PASS: local SVG node/arrow graph, shared method unique, vertical/horizontal, 320/390/1440 contained panning, plan read-only and actuals unchanged'
 a.set_viewport_size({'width':390,'height':844});click('纵向');snap('08-mobile-shared-method',a.get_by_test_id('flow-node-method-'+method['id']))
 # R06 current month for records and totals, explicit navigation and return reset.
 a.set_viewport_size({'width':390,'height':844});click('记录');expect(a.get_by_text(today[:7],exact=True)).to_be_visible();click('课时核对');expect(a.get_by_text(today[:7],exact=True)).to_be_visible();expect(a.get_by_role('button',name='合成唐老师：1 节',exact=True)).to_be_visible();click('上个月');expect(a.get_by_text(today[:7],exact=True)).to_have_count(0);click('今天');click('记录');expect(a.get_by_text(today[:7],exact=True)).to_be_visible();click('课时核对');expect(a.get_by_role('button',name='合成唐老师：1 节',exact=True)).to_be_visible();snap('07-current-month-institution-totals')
 results['R06']='PASS: classroom and totals default current business month; previous month works; returning from Today resets current month'
 assert not errors,errors
 (OUT.parent/'checks.json').write_text(json.dumps({'synthetic':True,'production_accessed':False,'deployed':False,'viewports':[320,390,1440],'large_text':True,'results':results,'screenshots':shots},ensure_ascii=False,indent=2)+'\n')
 for k,v in results.items():print(k,v)
 browser.close()
