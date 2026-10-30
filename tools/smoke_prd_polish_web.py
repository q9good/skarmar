"""Final PRD interactions: three-click plan, inline knowledge/basis, shared help and arbitrary lesson dates."""
from prd_ui_helpers import reveal_control
import os,re
from pathlib import Path
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import sync_playwright,expect
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/');assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/prd-polish-ui');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
 b=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox']);ctx=b.new_context(viewport={'width':360,'height':740});page=ctx.new_page();errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 def btn(name):return page.get_by_role('button',name=name,exact=True)
 def click(name):reveal_control(page,'button',name).click()
 def field(name):return reveal_control(page,'textbox',name)
 def call(method,path,body=None):
  r=ctx.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'});assert r.ok,r.text();return r.json()
 def entity(kind,value):
  id=str(uuid4());body={'id':id,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value}};body.update({'method_create_mode':'quick'} if kind=='method' else {});return call('PUT','/prd/records/'+id,body)['value']
 def close_all():
  for _ in range(8):
   if page.get_by_role('dialog').count()==0:return
   btn(re.compile('^关闭')).last.click()
  raise AssertionError('Dialogs did not drain')
 page.goto(BASE);field('账号').fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录');expect(btn('记录课堂')).to_be_enabled()
 g=entity('goal',{'title':'测试轮流','level':'short'});m=entity('method',{'title':'测试等待','goal_ids':[g['id']]});c=entity('catalog',{'kind':'course','name':'测试课','active':True});t=entity('catalog',{'kind':'teacher','name':'测试老师','active':True});page.reload();expect(btn('记录家庭')).to_be_enabled()
 # Exactly three button presses after today is shown; typing is not counted.
 click('记录家庭');expect(page.get_by_role('dialog',name='填写干预计划',exact=True)).to_be_visible();click('主要练：测试轮流');field('准备怎么做').fill('测试：轮流滚球');expect(btn('保存共享草稿并退出')).to_be_in_viewport(ratio=1);click('保存共享草稿并退出');expect(page.get_by_role('dialog')).to_have_count(0);expect(btn('记录家庭：填写今天的总结')).to_be_visible();assert call('GET','/prd/state')['sessions'][0]['status']=='planned'
 print('PASS default date/time plus quick main target saves plan in three clicks with one sentence; footer always visible',flush=True)
 click('方法');click('查看方法：测试等待');click('记一条心得');field(re.compile(r'^(?:经验|困难)名称（必填）$')).fill('测试等待心得');field('做法与依据').fill('测试：熟悉游戏先等待');reveal_control(page,'combobox','经验阶段').select_option('try');click('保存跟进');expect(page.get_by_role('dialog',name=re.compile(r'^(?:记一条|编辑|删除)(?:经验|困难)$'),exact=True)).to_have_count(0)
 k=call('GET','/prd/state')['knowledge'][0];assert k['input']['method_ids']==[m['id']];page.get_by_role('dialog',name='查看详情与修改记录',exact=True).get_by_role('button',name='查看关联知识：测试等待心得',exact=True).click();click('据此调整方法：测试等待');field('具体做法／必填').fill('测试：先等待，再视需要示范');field('修改说明').fill('测试：根据熟悉场景的心得调整');click('保存');expect(page.get_by_role('dialog',name='填写方法',exact=True)).to_have_count(0);saved=call('GET','/prd/state')['methods'][0];assert saved['version']==2 and saved['basis_snapshots'][0]['id']==k['input']['id'];close_all()
 print('PASS method-context experience links both ways and explicit method adjustment keeps exact knowledge basis',flush=True)
 click('目标');click('新建目标');field('本领名称').fill('测试有例子的目标');click('补充目标说明（可选）');field('观察标准').fill('测试：保留自己的描述');click('查看填写示例：观察标准');expect(page.get_by_text(re.compile('合成示例 · 例如：'))).to_be_visible();expect(field('观察标准')).to_have_value('测试：保留自己的描述');click('关闭填写目标')
 click('我的账号');click('使用说明与手机桌面入口');expect(page.get_by_text('怎样理解方法的证据',exact=True)).to_be_visible();expect(page.get_by_text('添加到手机桌面',exact=True)).to_be_visible();click('关闭使用与方法说明');click('返回')
 click('今天');click('记录课堂');expect(btn('课程名称（必填）：请选择课程')).to_be_visible();expect(btn('授课老师（必填）：请选择老师')).to_be_visible();expect(page.get_by_text('选好课程和老师即可提交，观察可以稍后补。',exact=True)).to_have_count(0);click('关闭记录课堂');click('目标');expect(btn('继续课记：未选课程')).to_be_visible();click('继续课记：未选课程');expect(page.get_by_text('选好课程和老师即可提交，观察可以稍后补。',exact=True)).to_have_count(0);click('关闭记录课堂')
 print('PASS examples remain readable without replacing input, static evidence/install help and global lesson-draft recovery',flush=True)
 today=page.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date())");lid=str(uuid4());call('PUT','/prd/lessons/'+lid,{'id':lid,'operation_id':str(uuid4()),'expected_version':None,'action':{'kind':'observation','value':{'business_date':today,'course_id':c['id'],'teacher_id':t['id'],'notes':'测试课堂','teacher_notes':'测试：换个球再试'}}});
 with page.expect_response(re.compile('/api/prd/state$')):page.evaluate("window.dispatchEvent(new Event('focus'))")
 click('记录');click('课时核对');click('自定义课时核对日期');page.get_by_label('课时起始日期：选择日期',exact=True).fill(today);page.get_by_label('课时截止日期：选择日期',exact=True).fill(today);expect(btn('测试老师：1 节')).to_be_visible();click('测试老师：1 节');expect(btn(re.compile('^查看课记：'))).to_have_count(1)
 click('课堂记录');field('搜索课堂观察和老师建议').fill('换个球');reveal_control(page,'checkbox','只看老师建议').click();expect(page.get_by_text('匹配来源：老师建议',exact=True)).to_be_visible()
 page.set_viewport_size({'width':320,'height':740});assert not page.evaluate('document.documentElement.scrollWidth > innerWidth');page.screenshot(path=str(OUT/'lesson-filter-mobile.png'));assert not errors,errors
 print('PASS custom settlement dates and teacher-based totals, search matching-source label and narrow layout',flush=True);b.close()
