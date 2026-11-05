"""Actual saved goal catalogue and all-level tree. Disposable localhost data only."""
import os,re,json
from pathlib import Path
from uuid import uuid4
from urllib.parse import urlparse
from playwright.sync_api import sync_playwright,expect
from prd_ui_helpers import reveal_control
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/goal-tree-2026-11-05/screenshots');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 ctx=browser.new_context(viewport={'width':390,'height':844});page=ctx.new_page();errors=[];shots=[];page.on('pageerror',lambda e:errors.append(str(e)))
 def button(n):return reveal_control(page,'button',n)
 def click(n):button(n).click()
 def field(n):return reveal_control(page,'textbox',n)
 def checkbox(n):return reveal_control(page,'checkbox',n)
 def call(method,path,body=None,status=200,context=ctx):
  r=context.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','X-Skarma-Data-Epoch':'formal-20261101'});assert r.status==status,(path,r.status,r.text());return r.json()
 def entity(kind,value):
  i=str(uuid4());return call('PUT','/prd/records/'+i,{'id':i,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value},**({'method_create_mode':'full'} if kind=='method' else {})})['value']
 def state():return call('GET','/prd/state')
 def login(pg,user='owner'):
  pg.goto(BASE);pg.get_by_role('textbox',name='账号',exact=True).fill(user);pg.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');pg.get_by_role('button',name='登录',exact=True).click();expect(pg.get_by_role('button',name='记录课堂',exact=True)).to_be_enabled()
 def shot(name,focus=None):
  if focus:focus.scroll_into_view_if_needed()
  else:page.evaluate("()=>{for(const e of document.querySelectorAll('*'))if(['auto','scroll'].includes(getComputedStyle(e).overflowY))e.scrollTop=0}")
  page.wait_for_timeout(80);assert not page.evaluate('document.documentElement.scrollWidth>innerWidth+1');page.screenshot(path=str(OUT/(name+'.png')));shots.append(name+'.png')
 def open_picker():
  click('选择本次目标');expect(button('刷新目标目录')).to_be_enabled()
 login(page)
 roots=[entity('goal',{'title':title+'（合成）','level':'long'}) for title in ['共同活动与交往','学习时获取关键信息','表达经历与需要','主动探索','面对困难的信心','生活中的自主']]
 hidden=entity('goal',{'title':'已收起旧短期（合成）','level':'short','archived':True})
 method=entity('method',{'title':'先跟随再回应（合成）','steps':'先跟着孩子的玩法，再回应他的邀请。','goal_ids':[roots[0]['id']]})
 call('POST','/auth/users',{'username':'family','display_name':'合成另一位家人','password':'synthetic-prd-ui-password','role':'member'})
 page.reload();expect(button('记录课堂')).to_be_enabled();click('目标')
 for g in roots:expect(button('查看目标：'+g['input']['title'])).to_be_visible()
 expect(page.get_by_role('button',name='查看目标：'+hidden['input']['title'],exact=True)).to_have_count(0)
 click('筛选目标');page.get_by_role('combobox',name='目标期限筛选',exact=True).select_option('short');click('筛选目标');expect(button('显示全部目标')).to_be_visible();click('显示全部目标');expect(button('查看目标：'+roots[0]['input']['title'])).to_be_visible()
 click('今天');click('记录家庭');field('准备做什么').fill('合成：和孩子玩喜欢的活动，跟随兴趣调整');click('保存计划并退出');click('记录家庭：填写今天的总结');open_picker()
 picker=page.get_by_role('dialog',name='选择本次目标',exact=True)
 assert picker.get_by_role('checkbox',name=re.compile('^选择目标：')).count()==len(roots)
 for g in roots:expect(checkbox('选择目标：'+g['input']['title'])).to_be_enabled()
 expect(picker.get_by_role('checkbox',name='选择目标：'+hidden['input']['title'],exact=True)).to_have_count(0)
 checkbox('选择目标：'+roots[0]['input']['title']).click();click('查看目标来源：'+roots[0]['input']['title']);expect(page.get_by_role('dialog',name='查看详情与修改记录',exact=True)).to_be_visible();page.go_back();expect(checkbox('选择目标：'+roots[0]['input']['title'])).to_be_checked();expect(button('完成目标选择')).to_be_in_viewport(ratio=1);shot('01-all-long-goals-selectable');click('完成目标选择')
 field('实际表现：'+roots[0]['input']['title']).fill('合成：孩子主动邀请家人加入喜欢的玩法');page.get_by_role('combobox',name='实际方法确认',exact=True).select_option('used');checkbox('实际方法：'+method['input']['title']).click();checkbox('补充经验和困难（可选）').click();click('新增本次经验');field('经验内容 1').fill('合成经验：先跟随玩法更愿意邀请互动');reveal_control(page,'combobox','经验阶段 1').select_option('try');shot('02-long-goal-only-summary',field('实际表现：'+roots[0]['input']['title']));click('正式提交干预总结');expect(page.get_by_role('dialog')).to_have_count(0)
 first=state()['sessions'][0];assert first['status']=='completed' and [t['goal']['id'] for t in first['actual_targets']]==[roots[0]['id']] and not first['planned_targets'];assert len(state()['goals'])==7 and first['actual_methods'][0]['id']==method['id'] and state()['knowledge'][0]['input']['goal_ids']==[roots[0]['id']]
 print('PASS saved catalogue has all selectable long goals, excludes archived defaults, clears hidden page filters; long-only actual summary saves without inventing a short goal',flush=True)
 # Shared writes occur while this form is open; another actor then updates a target. Reopening reads the current catalogue.
 second=browser.new_context();pg=second.new_page();login(pg,'family')
 click('记录家庭');click('更多干预操作');click('已经做完：直接记总结');open_picker();checkbox('选择目标：'+roots[0]['input']['title']).click();click('完成目标选择');field('实际表现：'+roots[0]['input']['title']).fill('合成：先主动分享，随后尝试轮流')
 mid=entity('goal',{'title':'来回共同玩（合成）','level':'medium','parent_id':roots[0]['id']});leaf=entity('goal',{'title':'轮流滚球（合成）','level':'short','parent_id':mid['id']});sibling=entity('goal',{'title':'轮流积木（合成）','level':'short','parent_id':mid['id']})
 # Write under the member identity, proving shared saved state, not a local reference list.
 remote=entity('goal',{'title':'另一位家人新增方向（合成）','level':'long'})
 call('PUT','/prd/records/'+remote['id'],{'id':remote['id'],'operation_id':str(uuid4()),'expected_version':remote['version'],'change_note':'合成家人更新','mutation':{'kind':'goal','value':{**remote['input'],'title':'家人刚更新的目标（合成）'}}},context=second)
 open_picker();expect(checkbox('选择目标：家人刚更新的目标（合成）')).to_be_visible()
 
 if page.get_by_role('button',name='展开下级目标：'+roots[0]['input']['title'],exact=True).count():click('展开下级目标：'+roots[0]['input']['title'])
 click('展开下级目标：'+mid['input']['title']);checkbox('选择目标：'+leaf['input']['title']).click()
 expect(checkbox('选择目标：'+mid['input']['title'])).to_have_attribute('aria-checked','mixed');expect(checkbox('选择目标：'+roots[0]['input']['title'])).to_be_checked()
 checkbox('选择目标：'+mid['input']['title']).click();expect(checkbox('选择目标：'+mid['input']['title'])).to_be_checked();checkbox('选择目标：'+mid['input']['title']).click();expect(checkbox('选择目标：'+mid['input']['title'])).to_have_attribute('aria-checked','mixed')
 checkbox('选择目标：'+sibling['input']['title']).click();checkbox('选择目标：'+leaf['input']['title']).click();expect(checkbox('选择目标：'+mid['input']['title'])).to_have_attribute('aria-checked','mixed');checkbox('选择目标：'+sibling['input']['title']).click();expect(checkbox('选择目标：'+mid['input']['title'])).not_to_be_checked();expect(checkbox('选择目标：'+roots[0]['input']['title'])).to_be_checked()
 checkbox('选择目标：'+leaf['input']['title']).click();page.get_by_test_id('actual-goal-branch-'+roots[0]['id']).evaluate("e=>e.scrollIntoView({block:'start'})");shot('03-tree-and-automatic-parents',checkbox('选择目标：'+roots[0]['input']['title']))
 for width,height in [(320,740),(430,932),(1440,900)]:
  page.set_viewport_size({'width':width,'height':height});assert not page.evaluate('document.documentElement.scrollWidth>innerWidth+1');assert picker.evaluate('e=>e.scrollWidth<=e.clientWidth+1');expect(button('完成目标选择')).to_be_in_viewport(ratio=1)
 page.set_viewport_size({'width':390,'height':844})
 click('收起全部目标');expect(page.get_by_role('checkbox',name='选择目标：'+leaf['input']['title'],exact=True)).to_have_count(0);field('搜索目标').fill('轮流滚球');expect(page.get_by_role('button',name='收起全部目标',exact=True)).to_have_count(0);expect(checkbox('选择目标：'+leaf['input']['title'])).to_be_checked();expect(checkbox('选择目标：'+roots[0]['input']['title'])).to_be_checked();field('搜索目标').fill('');click('展开全部目标')
 checkbox('显示已收起目标').click();expect(checkbox('选择目标：'+hidden['input']['title'])).to_be_visible();checkbox('显示已收起目标').click();expect(page.get_by_role('checkbox',name='选择目标：'+hidden['input']['title'],exact=True)).to_have_count(0)
 click('完成目标选择');expect(page.get_by_role('button',name='核对并刷新候选项',exact=True)).to_have_count(0);expect(field('实际表现：'+roots[0]['input']['title'])).to_have_value('合成：先主动分享，随后尝试轮流');expect(page.get_by_role('textbox',name='实际表现：'+mid['input']['title'],exact=True)).to_have_count(0);field('实际表现：'+leaf['input']['title']).fill('合成：提醒一次后等待同伴，再把球推回来');shot('04-direct-and-inherited-observations',field('实际表现：'+leaf['input']['title']))
 # A failed directory read neither changes selections nor overwrites existing observation text.
 page.route('**/api/prd/state',lambda route:route.fulfill(status=503,content_type='application/json',body='{"error":"合成目录暂不可用"}'),times=1);open_picker();expect(picker.get_by_role('alert')).to_contain_text('保留已读取的目录');expect(checkbox('选择目标：'+leaf['input']['title'])).to_be_checked();shot('05-catalogue-failure-retains-selection');click('刷新目标目录');expect(picker.get_by_role('alert')).to_have_count(0);click('完成目标选择');expect(field('实际表现：'+leaf['input']['title'])).to_have_value('合成：提醒一次后等待同伴，再把球推回来')
 open_picker();checkbox('选择目标：'+roots[0]['input']['title']).click();expect(checkbox('选择目标：'+roots[0]['input']['title'])).to_have_attribute('aria-checked','mixed');click('完成目标选择');expect(page.get_by_role('textbox',name='实际表现：'+roots[0]['input']['title'],exact=True)).to_have_count(0)
 checkbox('补充经验和困难（可选）').click();expect(page.get_by_role('dialog',name='填写干预总结',exact=True).get_by_text('可参考经验：合成经验：先跟随玩法更愿意邀请互动',exact=True)).to_be_visible()
 open_picker();checkbox('选择目标：'+roots[0]['input']['title']).click();click('完成目标选择');expect(field('实际表现：'+roots[0]['input']['title'])).to_have_value('合成：先主动分享，随后尝试轮流')
 page.get_by_role('combobox',name='实际方法确认',exact=True).select_option('unknown')
 def lose(route):
  response=route.fetch();assert response.ok,response.text();route.abort('failed')
 page.route('**/api/prd/sessions/*',lose,times=1);click('正式提交干预总结');expect(button('重试同一次干预保存')).to_be_visible();page.reload();expect(button('记录课堂')).to_be_enabled();click(re.compile('^继续干预填写：'));click('重试同一次干预保存');expect(page.get_by_role('dialog')).to_have_count(0)
 s=state();assert len(s['sessions'])==2;new=next(r for r in s['sessions'] if r['id']!=first['id']);assert {t['goal']['id'] for t in new['actual_targets']}=={roots[0]['id'],leaf['id']};assert len(call('GET','/prd/sessions/'+new['id']+'/history'))==1
 assert {g['id'] for g in next(t for t in new['actual_targets'] if t['goal']['id']==leaf['id'])['ancestors']}=={roots[0]['id'],mid['id']}
 report=call('GET','/prd/coverage?days=30&source=family');row=next(r for r in report['goals'] if r['id']==roots[0]['id']);assert report['records']['family']==2 and row['linked']['family']==2 and row['direct']['family']==2 and row['descendants']['family']==1
 click('目标');expect(page.get_by_test_id('goal-overview').get_by_role('img')).to_have_attribute('aria-label',re.compile('共4次目标命中'));shot('06-goal-overview-deduplicated')
 for width,height in [(320,740),(430,932),(1440,900)]:
  page.set_viewport_size({'width':width,'height':height});assert not page.evaluate('document.documentElement.scrollWidth>innerWidth+1')
 assert not errors,errors
 (OUT/'checks.json').write_text(json.dumps({'synthetic':True,'screenshots':shots,'checks':['all_saved_horizons_selectable','long_only_completion','no_invented_goal','shared_current_catalogue','archived_matches_goals','collapsible_searchable_tree','automatic_parent_mixed_state','explicit_parent_independent','sibling_parent_retention','no_duplicate_parent_note','directory_failure_preserves_form','exact_retry_and_immutable_snapshots','deduplicated_ancestor_coverage','320_430_1440_layout'],'errors':errors},ensure_ascii=False,indent=2)+'\n')
 print('PASS all-level actual tree: shared current catalogue, bounded ancestor association, search/collapse, retained observations and failure/retry, real save, historical parent snapshots and deduplicated coverage',flush=True)
 second.close();browser.close()
