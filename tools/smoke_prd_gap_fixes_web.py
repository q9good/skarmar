"""G01–G14 interaction regressions. Fresh local synthetic records only."""
import os,re
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import sync_playwright,expect
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
with sync_playwright() as p:
 b=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox']);ctx=b.new_context(viewport={'width':360,'height':740});page=ctx.new_page();errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 def btn(n):return page.get_by_role('button',name=n,exact=True)
 def click(n):btn(n).click()
 def field(n):return page.get_by_role('textbox',name=n,exact=True)
 def call(method,path,body=None):
  r=ctx.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'});assert r.ok,r.text();return r.json()
 def entity(kind,value):
  id=str(uuid4());return call('PUT','/prd/records/'+id,{'id':id,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value}})['value']
 def kb(kind,title,state):
  id=str(uuid4());return call('POST','/prd/process',{'kind':'knowledge','value':{'id':id,'operation_id':str(uuid4()),'action':'create','input':{'id':id,'kind':kind,'content':title,'goal_ids':[g['id']],'method_ids':[m['id']],'stage':'helpful' if kind=='experience' else None,'next_review_date':today if kind=='difficulty' and state=='following' else None},'details':{'title':title,'state':state}}})
 def sync():
  with page.expect_response(re.compile('/api/prd/state$')):page.evaluate("window.dispatchEvent(new Event('focus'))")
 page.goto(BASE);field('账号').fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录');expect(btn('记录上课')).to_be_enabled()
 today=page.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date())")
 g=entity('goal',{'title':'合成次数本领','level':'short','template':'count'});m=entity('method',{'title':'合成方法','goal_ids':[g['id']]})
 for i in range(5):kb('difficulty','合成困难'+str(i),'following')
 kb('difficulty','已解决不提示','resolved');kb('difficulty','暂停不提示','paused');kb('experience','退役不提示','retired')
 k=kb('experience','保留反例经验','active');eid=str(uuid4());aid=str(uuid4())
 call('POST','/prd/process',{'kind':'feedback','value':{'id':eid,'operation_id':str(uuid4()),'action':'record','input':{'knowledge_id':k['input']['id'],'source':{'kind':'independent','id':eid,'version':1},'business_date':today,'result':'unhelpful','observation':'更早的反例仍要看到'},'next_actions':[{'id':aid,'content':'原待做行动','goal_ids':[g['id']]}]}})
 for i in range(32):
  id=str(uuid4());call('POST','/prd/process',{'kind':'feedback','value':{'id':id,'operation_id':str(uuid4()),'action':'record','input':{'knowledge_id':k['input']['id'],'source':{'kind':'independent','id':id,'version':1},'business_date':today,'result':'helpful','observation':'较新的有帮助观察'+str(i)}}})
 page.reload();expect(btn('记录上课')).to_be_enabled();click('记录一次家庭干预');click('主要练：合成次数本领');field('准备怎么做').fill('计划输入必须保留')
 for title in ['已解决不提示','暂停不提示','退役不提示']:expect(page.get_by_role('dialog').get_by_text(title,exact=True)).to_have_count(0)
 expect(page.get_by_text(re.compile('尚无记录；从'))).to_be_visible()
 newer=entity('goal',{'title':'同步新增候选','level':'short'});sync();expect(btn('核对并刷新候选项')).to_be_visible();click('选择本次目标');expect(page.get_by_role('checkbox',name='选择目标：同步新增候选',exact=True)).to_have_count(0);click('关闭选择本次目标');click('核对并刷新候选项');expect(field('准备怎么做')).to_have_value('计划输入必须保留');click('选择本次目标');expect(page.get_by_role('checkbox',name='选择目标：同步新增候选',exact=True)).to_be_visible();click('关闭选择本次目标')
 page.get_by_role('checkbox',name='纳入这条下一步：原待做行动',exact=True).click();click('修改这条下一步：原待做行动');expect(page.get_by_role('dialog',name='维护下一步',exact=True)).to_be_visible();field('下一步内容').fill('修改后的待做行动');field('下一步修改说明').fill('先示范再等待');click('保存跟进');expect(page.get_by_role('dialog',name='维护下一步',exact=True)).to_have_count(0);expect(field('准备怎么做')).to_have_value('计划输入必须保留');expect(page.get_by_text('行动已有新版本：修改后的待做行动。当前安排和原选择仍保留。',exact=True)).to_be_visible();click('采用这条行动的新版本');expect(page.get_by_role('checkbox',name='纳入这条下一步：修改后的待做行动',exact=True)).to_be_checked();click('保存共享草稿并退出');expect(page.get_by_role('dialog')).to_have_count(0)
 plan=next(s for s in call('GET','/prd/state')['sessions'] if s['status']=='planned');assert plan['action_snapshots'][0]['input']['content']=='修改后的待做行动'
 print('PASS closed knowledge filter, reasons, frozen candidates and explicit refresh preserve text; action child edit returns and adopts the current version exactly once',flush=True)
 click('填写今天的总结');ob=field('实际表现：合成次数本领');assert len(ob.get_attribute('placeholder'))<=15;assert ob.get_attribute('aria-describedby');expect(ob).to_have_value('');click('查看填写示例：实际表现：合成次数本领');expect(page.get_by_text(re.compile('合成示例 · 例如：今天5次轮流机会'))).to_be_visible();ob.fill('5次机会，提醒后3次做到')
 shown=page.get_by_role('button',name=re.compile(r'^这次没遇到：合成困难'));expect(shown).to_have_count(3);visible=shown.all_text_contents();hidden=next('这次没遇到：合成困难'+str(i) for i in range(5) if not any('合成困难'+str(i) in text for text in visible));click('展开全部相关困难');expect(shown).to_have_count(5);click(hidden);click('收起相关困难');expect(shown).to_have_count(4);expect(page.get_by_text('已选：这次没遇到。本次没遇到不算改善，也不计可比观察。',exact=True)).to_be_visible()
 page.get_by_role('combobox',name='实际方法确认',exact=True).select_option('used');page.get_by_role('checkbox',name='实际方法：合成方法',exact=True).click();page.get_by_role('combobox',name='下一步结果：修改后的待做行动',exact=True).select_option('done');click('正式提交干预总结');expect(page.get_by_role('dialog')).to_have_count(0)
 click('方法');click('查看方法：合成方法');expect(page.get_by_text('家庭1次／机构0次；最近使用：'+today,exact=True)).to_be_visible();click('查看方法第1版的使用记录');expect(page.get_by_text('只看方法第1版的实际记录。',exact=True)).to_be_visible();click('关闭记录时间分布');click('关闭查看详情与修改记录')
 click('记录');click('家庭干预');day=btn(today+' · 已完成1次');day.scroll_into_view_if_needed();assert day.bounding_box()['width']>=44;day.click();expect(page.get_by_role('dialog',name=today+' 家庭干预',exact=True)).to_be_visible();expect(page.get_by_role('dialog').get_by_text('5次机会，提醒后3次做到',exact=True)).to_be_visible();click('关闭'+today+' 家庭干预');expect(btn('显示整月记录')).to_be_visible();page.set_viewport_size({'width':320,'height':740});assert day.bounding_box()['width']>=44;assert not page.evaluate('document.documentElement.scrollWidth > innerWidth');click('按目标查看时间分布');expect(page.get_by_role('combobox',name='时间分布粒度',exact=True)).to_have_count(0);page.set_viewport_size({'width':701,'height':740});expect(page.get_by_role('combobox',name='时间分布粒度',exact=True)).to_be_visible();click('关闭记录时间分布')
 print('PASS goal-template examples, first-three difficulties retain chosen feedback; actual method-version usage, direct 44px calendar and >700px daily distribution',flush=True)
 page.set_viewport_size({'width':360,'height':740});click('跟进');click('经验库与困难');click('查看知识：保留反例经验');expect(page.get_by_text('更早的反例仍要看到',exact=False).first).to_be_visible();expect(page.get_by_text(re.compile('最近没帮上的观察'))).to_be_visible();click('关闭经验与困难详情')
 click('要处理的');btn(re.compile('^查看共享复盘队列')).click();expect(page.get_by_text('本轮复盘进度',exact=True)).to_be_visible();click('开始复盘：合成困难0');expect(page.get_by_text(re.compile('本次保存后的下次回看：\\d{4}'))).to_be_visible();field('复盘依据与说明').fill('这轮调整后继续观察');click('新增复盘下一步');field('复盘下一步 1').fill('本轮明确下一步');click('保存并下一项');expect(field('复盘依据与说明')).to_have_value('');click('关闭逐项复盘');expect(page.get_by_text(re.compile('已完成1／顺延0'))).to_be_visible();expect(page.get_by_text('本轮已处理与下一步',exact=True)).to_be_visible();page.reload();expect(btn('记录上课')).to_be_enabled();click('跟进');btn(re.compile('^查看共享复盘队列')).click();expect(page.get_by_text(re.compile('已完成1／顺延0'))).to_be_visible();click('用本轮待做的下一步新建计划');expect(field('准备怎么做')).to_have_value('本轮明确下一步');assert next(a for a in call('GET','/prd/state')['actions'] if a['input']['content']=='本轮明确下一步')['status']=='pending';click('关闭填写干预计划')
 print('PASS latest counterexample survives first-page limit; review progress/recap resumes after reload and explicitly creates an unreserved local plan',flush=True)
 for i in range(16):entity('goal',{'title':'长选项'+str(i),'level':'short'})
 page.reload();expect(btn('记录上课')).to_be_enabled();click('方法');click('新建方法');field('方法名称').fill('长列表方式');click('搜索选择方法适用目标（已选0）');field('搜索方法适用目标').fill('长选项15');expect(page.get_by_role('checkbox',name='长选项15',exact=True)).to_be_visible();page.get_by_role('checkbox',name='长选项15',exact=True).click();click('关闭选择方法适用目标');expect(page.get_by_role('checkbox',name='长选项15',exact=True)).to_be_checked();expect(field('方法名称')).to_have_value('长列表方式');assert not page.evaluate('document.documentElement.scrollWidth > innerWidth');assert not errors,errors
 print('PASS >15 relation choices use accessible search and preserve chosen IDs and other form content without browser errors',flush=True);b.close()
