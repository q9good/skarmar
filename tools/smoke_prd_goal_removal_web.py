"""Cache eviction + retained in-progress observations; synthetic localhost only."""
import os,re
from uuid import uuid4
from urllib.parse import urlparse
from playwright.sync_api import sync_playwright,expect
from prd_ui_helpers import reveal_control
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
with sync_playwright() as p:
 b=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox']);ctx=b.new_context(viewport={'width':390,'height':844});page=ctx.new_page();errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 def click(n):reveal_control(page,'button',n).click()
 def field(n):return reveal_control(page,'textbox',n)
 def call(method,path,body=None):
  r=ctx.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1','X-Skarma-Data-Epoch':'formal-20261101'});assert r.ok,r.text();return r.json()
 def goal(title,level):
  i=str(uuid4());return call('PUT','/prd/records/'+i,{'id':i,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':'goal','value':{'title':title,'level':level}}})['value']
 page.goto(BASE);field('账号').fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录');expect(page.get_by_role('button',name='记录课堂',exact=True)).to_be_enabled()
 removed=goal('合成待移除缓存目标','short');keep=goal('合成保留长期目标','long');old_id=str(uuid4());today=page.evaluate("new Date().toLocaleDateString('sv-SE')");call('PUT','/prd/sessions/'+old_id,{'id':old_id,'operation_id':str(uuid4()),'expected_version':None,'action':'complete','change_note':'','input':{'business_date':today,'time_slot':'下午','summary':{'targets':[{'goal_id':removed['id'],'performed':True,'observation':'合成旧记录观察'}],'confirmation':'none'}}});page.reload();expect(page.get_by_role('button',name='记录课堂',exact=True)).to_be_enabled()
 click('记录家庭');click('更多干预操作');click('已经做完：直接记总结');click('选择本次目标');reveal_control(page,'checkbox','选择目标：'+removed['input']['title']).click();click('完成目标选择');field('实际表现：'+removed['input']['title']).fill('合成原观察必须保留');click('关闭填写干预总结');click('目标');expect(page.get_by_role('button',name='查看目标：'+removed['input']['title'],exact=True)).to_be_visible()
 # Isolate the browser contract: the service's confirmed tombstone response is
 # simulated; actual DB deletion and reference checks are rehearsed on real DOs.
 def tombstone(route):
  r=route.fetch();data=r.json();data['goals']=[g for g in data['goals'] if g['id']!=removed['id']];data['removed_goal_ids']=[removed['id']];route.fulfill(response=r,json=data)
 page.route('**/api/prd/state',tombstone);page.evaluate("window.dispatchEvent(new Event('focus'))")
 expect(page.get_by_role('button',name='查看目标：'+removed['input']['title'],exact=True)).to_have_count(0);expect(page.get_by_role('button',name='查看目标：'+keep['input']['title'],exact=True)).to_be_visible()
 click('今天');click(re.compile('^继续干预填写：'));expect(page.get_by_role('dialog').get_by_text('原观察：合成原观察必须保留',exact=True)).to_be_visible();expect(page.get_by_role('dialog').get_by_text(re.compile('原选目标已删除'))).to_be_visible();click('解除未读到目标的关联 1')
 click('选择本次目标');expect(page.get_by_role('checkbox',name='选择目标：'+removed['input']['title'],exact=True)).to_have_count(0);reveal_control(page,'checkbox','选择目标：'+keep['input']['title']).click();click('完成目标选择');field('实际表现：'+keep['input']['title']).fill('合成重新确认实际长期表现');reveal_control(page,'combobox','实际方法确认').select_option('none');click('正式提交干预总结');expect(page.get_by_role('dialog')).to_have_count(0)
 state=call('GET','/prd/state');s=next(row for row in state['sessions'] if row['id']!=old_id);assert s['actual_targets'][0]['goal']['id']==keep['id'];assert next(t for t in s['input']['summary']['targets'] if t['goal_id']==removed['id'])=={'goal_id':removed['id'],'performed':False,'observation':'合成原观察必须保留'}
 page.reload();expect(page.get_by_role('button',name='记录课堂',exact=True)).to_be_enabled();click('目标');expect(page.get_by_role('button',name='查看目标：'+removed['input']['title'],exact=True)).to_have_count(0)
 click('今天');click(re.compile('.*合成旧记录观察.*'));click('修改已完成总结');expect(page.get_by_text('此目标已从目录删除；这里保留原记录目标，可更正当时的表现。',exact=True)).to_be_visible();field('实际表现：'+removed['input']['title']).fill('合成旧记录修订观察');field('本次修订说明').fill('合成更正观察');click('保存总结修订');expect(page.get_by_role('dialog')).to_have_count(0)
 updated=next(row for row in call('GET','/prd/state')['sessions'] if row['id']==old_id);assert updated['version']==2;assert updated['actual_targets'][0]['goal']['id']==removed['id'];assert updated['actual_targets'][0]['input']['note']=='合成旧记录修订观察'
 assert not errors,errors;print('PASS confirmed goal-removal metadata evicts old cached catalogue choices, preserves original observations and other goals, supports explicit unlink/reselection/save and survives refresh');b.close()
