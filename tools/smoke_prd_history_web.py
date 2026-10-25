"""Historical goal hierarchy, 90-day bars and unbounded recent-30 UI; synthetic localhost only."""
import os,re
from datetime import datetime,timedelta
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import sync_playwright,expect
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox']);ctx=browser.new_context(viewport={'width':360,'height':740});page=ctx.new_page();errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 def button(name):return page.get_by_role('button',name=name,exact=True)
 def click(name):button(name).click()
 def call(method,path,body=None):
  r=ctx.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'});assert r.ok,r.text();return r.json()
 def entity(kind,value):
  id=str(uuid4());r={'id':id,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value}};r.update({'method_create_mode':'quick'} if kind=='method' else {});return call('PUT','/prd/records/'+id,r)['value']
 page.goto(BASE);page.get_by_role('textbox',name='账号',exact=True).fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录');expect(button('记录上课')).to_be_enabled()
 today=page.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date())")
 parent=entity('goal',{'title':'旧中期','level':'medium'});other=entity('goal',{'title':'新中期','level':'medium'});child=entity('goal',{'title':'轮流游戏','level':'short','parent_id':parent['id']});method=entity('method',{'title':'示范后等待','goal_ids':[child['id']]})
 sessions=[]
 for offset in [0,7,30,100]:
  day=(datetime.fromisoformat(today)-timedelta(days=offset)).date().isoformat();id=str(uuid4());target={'goal_id':child['id'],'role':'main','note':''};summary={'targets':[{'goal_id':child['id'],'performed':True,'observation':'测试观察 '+str(offset)}],'confirmation':'used','methods':[{'id':method['id'],'version':1}],'tags':[],'context':'','signals':[],'new_knowledge':[],'new_actions':[],'action_outcomes':[]};input={'business_date':day,'time_slot':'晚上','plan':'','targets':[target],'methods':[],'focus_goal_ids':[],'tags':[],'context':'','lesson_sources':[],'action_ids':[],'summary':summary}
  sessions.append(call('PUT','/prd/sessions/'+id,{'id':id,'operation_id':str(uuid4()),'expected_version':None,'action':'complete','input':input,'change_note':''}))
 # Reparenting today must not move any historical evidence to the new parent.
 call('PUT','/prd/records/'+child['id'],{'id':child['id'],'operation_id':str(uuid4()),'expected_version':1,'change_note':'测试修改层级','mutation':{'kind':'goal','value':{**child['input'],'parent_id':other['id']}}})
 page.reload();expect(button('记录上课')).to_be_enabled();click('记录');click('家庭干预');click('按目标查看时间分布')
 expect(button('查看时间分布：轮流游戏')).to_be_visible();expect(page.get_by_text('覆盖 3 次 · 最近 '+today+'（0天前）',exact=True)).to_be_visible();click('查看时间分布：轮流游戏');expect(page.get_by_text('轮流游戏的记录',exact=True)).to_be_visible()
 page.get_by_role('combobox',name='时间明细范围',exact=True).select_option('30');old=sessions[-1]['input']['business_date'];expect(page.get_by_text(f'最近4次，实际跨度 {old} 至 {today}',exact=True)).to_be_visible()
 button(re.compile('^查看记录依据：')).first.click();expect(page.get_by_role('dialog',name='时间分布记录依据',exact=True)).to_be_visible();expect(page.get_by_text('实际方法：示范后等待（第1版）',exact=True)).to_be_visible();page.go_back();expect(page.get_by_role('dialog',name='记录时间分布',exact=True)).to_be_visible();expect(page.get_by_role('combobox',name='时间明细范围',exact=True)).to_have_value('30')
 page.get_by_role('combobox',name='并列目标层级',exact=True).select_option('medium');click('查看时间分布：旧中期');expect(page.get_by_text('旧中期的记录',exact=True)).to_be_visible();expect(button(re.compile('^查看记录依据：'))).to_have_count(4)
 click('查看时间分布：新中期');expect(page.get_by_text('尚无实际记录',exact=True)).to_be_visible();page.go_back();expect(page.get_by_role('dialog')).to_have_count(0)
 print('PASS 13 weekly bars and count/date, recent records across 90-day boundary, source method versions and historical parent invariance',flush=True)
 click('方法');click('查看方法：示范后等待');click('查看覆盖时间分布');expect(page.get_by_role('dialog',name='记录时间分布',exact=True)).to_be_visible();page.get_by_role('combobox',name='时间明细范围',exact=True).select_option('30');expect(button(re.compile('^查看记录依据：'))).to_have_count(4);button(re.compile('^查看记录依据：')).first.click();click('打开这次干预记录');expect(page.get_by_role('dialog',name='干预详情',exact=True)).to_be_visible()
 page.go_back();expect(page.get_by_role('dialog',name='记录时间分布',exact=True)).to_be_visible();page.go_back();expect(page.get_by_role('dialog',name='查看详情与修改记录',exact=True)).to_be_visible();page.go_back();expect(page.get_by_role('dialog')).to_have_count(0)
 for width in [320,430]:page.set_viewport_size({'width':width,'height':740});assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
 assert not errors,errors;print('PASS method-detail nested history/source navigation, browser Back and mobile layout',flush=True);browser.close()
