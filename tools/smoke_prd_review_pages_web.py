"""Thirty-item review pages must not consume unseen records; synthetic localhost only."""
import os,re
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import sync_playwright,expect
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
with sync_playwright() as p:
 b=p.chromium.launch(executable_path='/usr/bin/chromium',headless=True,args=['--no-sandbox']);ctx=b.new_context(viewport={'width':360,'height':740});page=ctx.new_page();errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 def btn(name):return page.get_by_role('button',name=name,exact=True)
 def click(name):btn(name).click()
 def call(method,path,body=None):
  r=ctx.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'});assert r.ok,r.text();return r.json()
 page.goto(BASE);page.get_by_role('textbox',name='账号',exact=True).fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录');expect(btn('记录上课')).to_be_enabled();gid=str(uuid4());call('PUT','/prd/records/'+gid,{'id':gid,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':'goal','value':{'title':'分页复盘目标','level':'short'}}});today=page.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date())")
 def moment(n):
  id=str(uuid4());return call('POST','/prd/process',{'kind':'observe','value':{'id':id,'operation_id':str(uuid4()),'expected_version':None,'kind':'moment','action':'save','input':{'business_date':today,'text':'分页验证观察'+str(n),'goal_ids':[gid]}}})
 for n in range(31):moment(n)
 page.reload();expect(btn('记录上课')).to_be_enabled();click('目标');click('查看目标：分页复盘目标');click('现在复盘这个对象');expect(page.get_by_text('可回看的新依据 31 条，当前已加载 3 条。',exact=True)).to_be_visible();expect(btn('继续查看复盘依据')).to_be_visible()
 moment(31);click('继续查看复盘依据');expect(page.get_by_role('alert')).to_contain_text('依据或安排有更新');expect(page.get_by_text('可回看的新依据 31 条，当前已加载 3 条。',exact=True)).to_be_visible();click('刷新复盘依据（保留本次说明）');expect(page.get_by_text('可回看的新依据 32 条，当前已加载 3 条。',exact=True)).to_be_visible();click('继续查看复盘依据');expect(page.get_by_text('可回看的新依据 32 条，当前已加载 30 条。',exact=True)).to_be_visible();click('继续查看复盘依据');expect(page.get_by_text('可回看的新依据 32 条，当前已加载 32 条。',exact=True)).to_be_visible();expect(btn('继续查看复盘依据')).to_have_count(0)
 click('保存并下一项');expect(page.get_by_role('dialog',name='逐项复盘',exact=True)).to_have_count(0);current=call('GET','/prd/process?view=review_context&object_kind=goal&id='+gid);assert current['total']==0;assert not errors,errors
 print('PASS paginated review UI detects concurrent changes, refresh keeps form, loads all evidence and consumes only displayed versions',flush=True);b.close()
