"""Measurements, moments, shared needs, revision conflicts and response-loss recovery; synthetic localhost only."""
import os,re
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import sync_playwright,expect
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox']);ctx=browser.new_context(viewport={'width':360,'height':740});page=ctx.new_page();errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 def btn(name):return page.get_by_role('button',name=name,exact=True)
 def click(name):btn(name).click()
 def field(name):return page.get_by_role('textbox',name=name,exact=True)
 def call(method,path,body=None):
  r=ctx.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'});assert r.ok,r.text();return r.json()
 def observe(kind):return call('GET','/prd/process?view=observations&object_kind='+kind)['items']
 page.goto(BASE);field('账号').fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录');expect(btn('记录上课')).to_be_enabled()
 gid=str(uuid4());call('PUT','/prd/records/'+gid,{'id':gid,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':'goal','value':{'title':'轮流游戏','level':'short','template':'count'}}});page.reload();expect(btn('记一个瞬间')).to_be_visible()
 click('记一个瞬间');field('想记下的瞬间').fill('测试：拿着球邀请家人');click('关联：轮流游戏');click('保存观察');expect(page.get_by_role('dialog')).to_have_count(0);expect(page.get_by_text('测试：拿着球邀请家人',exact=True)).to_be_visible()
 assert not call('GET','/prd/state')['sessions'];assert call('GET','/prd/coverage?days=30&source=family')['records']['selected']==0
 click('目标');click('查看目标：轮流游戏');click('查看观察与实际测量');expect(page.get_by_role('dialog',name='目标观察与测量',exact=True)).to_be_visible();expect(page.get_by_text('测试：拿着球邀请家人',exact=True)).to_be_visible();click('新增观察指标');field('指标名称').fill('轮替次数');field('预期帮助条件（可选）').fill('提醒一次');click('保存观察');expect(page.get_by_role('dialog',name='填写观察指标',exact=True)).to_have_count(0)
 click('记录一次测量：轮替次数');field('机会数').fill('0');field('做到数').fill('1');click('保存观察');expect(page.get_by_role('alert')).to_be_visible();assert not observe('measurement');field('做到数').fill('0');field('实际观察说明').fill('测试：没有合适机会');click('保存观察');expect(page.get_by_text('未计算做到比例（没有机会或未记录机会数）',exact=True)).to_be_visible()
 click('记录一次测量：轮替次数');field('机会数').fill('5');field('做到数').fill('3');field('实际帮助条件').fill('不提醒');click('保存观察');expect(page.get_by_text('做到 3／5 次，60%（该次机会内的比例）',exact=True)).to_be_visible();assert len(observe('measurement'))==2
 click('调整指标：轮替次数');field('预期帮助条件（可选）').fill('示范一次');field('观察修订说明').fill('测试：改为另一种帮助条件');click('保存观察');expect(page.get_by_text('轮替次数 · 指标第 2 版',exact=True)).to_be_visible();assert all(r['metric']['version']==1 for r in observe('measurement'))
 print('PASS linked moments excluded from training, zero opportunities remain unknown, invalid values rejected and measurements retain original metric version',flush=True)
 # Response lost after commit: persist and retry the identical measurement operation.
 click('记录一次测量：轮替次数');field('机会数').fill('4');field('做到数').fill('2')
 def lose(route):
  response=route.fetch();assert response.ok;route.abort('failed')
 page.route('**/api/prd/process',lose,times=1);click('保存观察');expect(btn('重试同一次观察保存')).to_be_visible();assert len(observe('measurement'))==3
 page.reload();expect(btn(re.compile('^继续观察填写：目标测量'))).to_be_visible();btn(re.compile('^继续观察填写：目标测量')).click();click('重试同一次观察保存');expect(page.get_by_role('dialog')).to_have_count(0);assert len(observe('measurement'))==3
 print('PASS lost measurement response survives reload and retries without additional record',flush=True)
 click('我的账号');click('共享需要与帮助说明');field('沟通方式').fill('测试：用手势表达休息');field('帮助平静的做法').fill('测试：安静陪伴');click('保存观察');expect(page.get_by_role('dialog')).to_have_count(0);assert observe('needs')[0]['input']['communication']=='测试：用手势表达休息'
 click('共享需要与帮助说明');field('沟通方式').fill('测试：我的未提交说明');current=observe('needs')[0];call('POST','/prd/process',{'kind':'observe','value':{'id':current['id'],'operation_id':str(uuid4()),'expected_version':1,'kind':'needs','action':'save','input':{**current['input'],'communication':'测试：另一位家人更新'},'note':'测试并发'}});click('保存观察');expect(btn('备份填写并采用最新观察')).to_be_visible();expect(field('沟通方式')).to_have_value('测试：我的未提交说明');click('备份填写并采用最新观察');expect(field('沟通方式')).to_have_value('测试：另一位家人更新');expect(page.get_by_text('此前输入备份（可复制）：',exact=True)).to_be_visible();field('观察修订说明').fill('测试：核对后保留');click('保存观察');expect(page.get_by_role('dialog',name='填写需要与帮助',exact=True)).to_have_count(0);assert observe('needs')[0]['version']==3
 print('PASS shared needs versioned saves and conflict reconciliation keep both inputs',flush=True)
 click('返回');click('记录');click('家庭干预');expect(page.get_by_text('瞬间观察（不计干预次数）',exact=True)).to_be_visible();moment=observe('moment')[0];click('编辑瞬间观察：'+moment['id'][:8]);field('想记下的瞬间').fill('测试：修改瞬间');field('观察修订说明').fill('测试修订');click('保存观察');expect(page.get_by_text('测试：修改瞬间',exact=True)).to_be_visible();click('查看观察修订：'+moment['id'][:8]);expect(page.get_by_text('想记下的瞬间：测试：拿着球邀请家人',exact=True)).to_have_count(0) # Label is actual observation in history.
 expect(page.get_by_text('实际观察：测试：拿着球邀请家人',exact=True)).to_be_visible();page.go_back();click('删除瞬间观察：'+moment['id'][:8]);field('观察修订说明').fill('测试清理');click('确认删除这条观察');expect(page.get_by_text('测试：修改瞬间',exact=True)).to_have_count(0);assert observe('moment')[0]['deleted']
 for width in [320,430]:page.set_viewport_size({'width':width,'height':740});assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
 assert not errors,errors;print('PASS moment editing, traceable history, explicit admin delete and narrow layout',flush=True);browser.close()
