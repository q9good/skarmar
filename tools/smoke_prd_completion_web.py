"""Four PRD completion flows; synthetic localhost DB, no remote accounts/data."""
import os,re
from datetime import datetime,timedelta
from urllib.parse import urlparse
from uuid import uuid4
from pathlib import Path
from playwright.sync_api import sync_playwright,expect

BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/prd-completion-ui-screenshots');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
 browser=p.chromium.launch(executable_path='/usr/bin/chromium',headless=True,args=['--no-sandbox'])
 ctx=browser.new_context(viewport={'width':1280,'height':900});page=ctx.new_page();errors=[]
 page.on('pageerror',lambda e:errors.append(str(e)))
 def btn(name):return page.get_by_role('button',name=name,exact=True)
 def click(name):btn(name).click()
 def field(name):return page.get_by_role('textbox',name=name,exact=True)
 def call(method,path,body=None):
  r=ctx.request.fetch(BASE+'/api'+path,method=method,data=body,headers={'X-Skarma-Request':'1','X-Skarma-Contract':'skarmar.prd.v1'})
  assert r.ok,r.text();return r.json()
 def entity(kind,value):
  id=str(uuid4());return call('PUT','/prd/records/'+id,{'id':id,'operation_id':str(uuid4()),'expected_version':None,'change_note':'','mutation':{'kind':kind,'value':value}})['value']
 def process(kind,value):return call('POST','/prd/process',{'kind':kind,'value':value})
 def observe(kind,id,value,version=None):return process('observe',{'id':id,'operation_id':str(uuid4()),'expected_version':version,'kind':kind,'action':'save','input':value,'note':'合成观察修订' if version else ''})
 page.goto(BASE);field('账号').fill('owner');page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password');click('登录');expect(btn('记录上课')).to_be_enabled()
 today=page.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit'}).format(new Date())")
 def ago(n):return (datetime.fromisoformat(today)-timedelta(days=n)).date().isoformat()
 g=entity('goal',{'title':'轮流游戏','level':'short','template':'count'});g2=entity('goal',{'title':'主动表达','level':'short'})
 entity('goal',{'title':'生活参与','level':'medium'});entity('goal',{'title':'长期共同注意','level':'long'})
 m=entity('method',{'title':'轮流示范','goal_ids':[g['id'],g2['id']]})
 kid=str(uuid4());process('knowledge',{'id':kid,'operation_id':str(uuid4()),'expected_version':None,'action':'create','input':{'id':kid,'kind':'experience','content':'先跟随再示范','goal_ids':[g['id']],'method_ids':[m['id']],'stage':'helpful'},'details':{'title':'先跟随'}})
 course=entity('catalog',{'kind':'course','name':'合成游戏课','active':True});teacher=entity('catalog',{'kind':'teacher','name':'合成老师','active':True})
 lid=str(uuid4());obs={'business_date':today,'course_id':course['id'],'teacher_id':teacher['id'],'notes':'合成课堂原观察','course_started':True}
 def lesson(version,kind,value):return call('PUT','/prd/lessons/'+lid,{'id':lid,'operation_id':str(uuid4()),'expected_version':version,'action':{'kind':kind,'value':value}})
 lesson(None,'observation',obs)
 metric_id=str(uuid4());metric={'goal_id':g['id'],'goal_version':1,'name':'轮替机会','template':'count','criterion':'5次机会','support':'提醒一次'};observe('metric',metric_id,metric)
 for date,n,k in [(ago(4),0,0),(ago(3),5,2),(ago(1),5,3)]:observe('measurement',str(uuid4()),{'metric_id':metric_id,'metric_version':1,'business_date':date,'support':'提醒一次','opportunities':n,'achieved':k})
 observe('metric',metric_id,{**metric,'support':'示范一次'},1)
 observe('measurement',str(uuid4()),{'metric_id':metric_id,'metric_version':2,'business_date':today,'support':'示范一次','opportunities':4,'achieved':2})
 sid=str(uuid4());call('PUT','/prd/sessions/'+sid,{'id':sid,'operation_id':str(uuid4()),'expected_version':None,'action':'complete','input':{'business_date':today,'time_slot':'晚上','targets':[{'goal_id':g['id'],'role':'main'}],'summary':{'targets':[{'goal_id':g['id'],'performed':True,'observation':'合成家庭实际表现'}],'confirmation':'used','methods':[{'id':m['id'],'version':1}],'first_try_ids':[m['id']],'context':'合成家人原文：当天换了材料'}}})
 page.reload();expect(btn('记录上课')).to_be_enabled();click('目标')
 page.get_by_role('combobox',name='目标期限筛选',exact=True).select_option('long');expect(btn('查看目标：长期共同注意')).to_be_visible();expect(btn('查看目标：轮流游戏')).to_have_count(0)
 page.get_by_role('combobox',name='目标期限筛选',exact=True).select_option('short');page.get_by_role('combobox',name='目标展示',exact=True).select_option('progress')
 expect(btn('查看测量趋势：轮流游戏')).to_be_visible();expect(page.get_by_text('实际进展来自各指标测量',exact=False)).to_be_visible();click('查看测量趋势：轮流游戏')
 expect(page.get_by_role('dialog',name='目标观察与测量',exact=True)).to_be_visible();expect(page.get_by_role('img',name=re.compile('轮替机会测量图'))).to_have_count(2)
 expect(page.get_by_text('轮替机会 · 指标第1版 · 目标第1版',exact=True)).to_be_visible();expect(page.get_by_text('轮替机会 · 指标第2版 · 目标第1版',exact=True)).to_be_visible()
 expect(page.get_by_text('未计算做到比例（没有机会或未记录机会数）',exact=True)).to_be_visible()
 page.set_viewport_size({'width':320,'height':740});expect(page.get_by_role('img',name=re.compile('轮替机会测量图'))).to_have_count(2);assert not page.evaluate('document.documentElement.scrollWidth > innerWidth');page.get_by_role('img',name=re.compile('轮替机会测量图')).last.scroll_into_view_if_needed();page.screenshot(path=str(OUT/'measurement-mobile.png'));page.set_viewport_size({'width':1280,'height':900});page.go_back()
 print('PASS goal horizon filters, coverage/progress switch and real version-separated measurement charts; zero opportunities remain unknown',flush=True)
 click('记录');btn(re.compile('^查看课记：')).first.click();click('补充／修改课堂观察');expect(page.get_by_role('checkbox',name='这门课程刚开始上（家人明确标记，可选）',exact=True)).to_be_checked();page.go_back();btn(re.compile('^查看课记：')).first.click();click('整理这条记录')
 page.get_by_role('checkbox',name='轮流游戏',exact=True).click();page.get_by_role('checkbox',name='实际方法：轮流示范',exact=True).click();click('补充已有经验反馈');click('本课有帮助：先跟随再示范');field('本课反馈依据：先跟随再示范').fill('合成老师先跟随后愿意再试')
 # Other family member edits the observation while this organization uses v1.
 lesson(1,'observation',{**obs,'notes':'合成另一位家人的新观察'})
 def lose(route):
  response=route.fetch();assert response.ok,response.text();route.abort('failed')
 page.route('**/api/prd/lessons/'+lid,lose,times=1);click('保存整理');expect(btn('重试同一次课记提交')).to_be_visible();click('重试同一次课记提交');expect(page.get_by_role('dialog')).to_have_count(0)
 events=call('GET','/prd/process?view=lesson_feedback&id='+lid);assert len(events)==1 and events[0]['input']['observation_version']==1
 assert events[0]['source_snapshot']['referenced_observation']['value']['notes']=='合成课堂原观察';assert events[0]['source_snapshot']['observation']['notes']=='合成另一位家人的新观察'
 btn(re.compile('^查看课记：')).first.click();expect(page.get_by_text('合成老师先跟随后愿意再试',exact=False)).to_be_visible();click('整理这条记录');click('补充已有经验反馈');expect(btn('本课有帮助：先跟随再示范')).to_have_count(0);page.go_back()
 print('PASS inline lesson experience feedback, cross-partition observation concurrency, exact retry and duplicate prevention',flush=True)
 click('目标');click('查看目标：轮流游戏');click('现在复盘这个对象')
 expect(page.get_by_text('同期做法与情境（已加载依据）',exact=True)).to_be_visible();expect(page.get_by_text('合成家人原文：当天换了材料',exact=True)).to_be_visible();expect(page.get_by_text(re.compile('明确课程新开始'))).to_be_visible()
 page.get_by_role('combobox',name='这次安排对家人的负担（可选）',exact=True).select_option('stretched');field('复盘依据与说明').fill('合成复盘：先保持安排');click('保存并下一项');expect(page.get_by_role('dialog',name='逐项复盘',exact=True)).to_have_count(0)
 latest=call('GET','/prd/process?view=review_context&object_kind=goal&id='+g['id'])['last_review'];assert latest['caregiver_state']=='stretched'
 page.go_back();expect(page.get_by_role('dialog')).to_have_count(0);click('跟进');click('查看已完成复盘');expect(page.get_by_text('家人当时感受：吃力',exact=True)).to_be_visible();click('查看复盘修改记录：轮流游戏');expect(page.get_by_role('dialog',name='复盘修改历史',exact=True).get_by_text('家人当时感受：吃力',exact=True)).to_be_visible();page.go_back();click('更正复盘说明：轮流游戏');page.get_by_role('combobox',name='这次安排对家人的负担（可选）',exact=True).select_option('okay');field('复盘依据与说明').fill('合成我的未提交修订')
 current=call('GET','/prd/process?view=review_context&object_kind=goal&id='+g['id']);schedule=current['schedule']
 process('review',{'id':latest['id'],'operation_id':str(uuid4()),'expected_version':1,'object_kind':'goal','object_id':g['id'],'object_version':0,'schedule_version':0,'generation':latest['generation'],'due_date':None,'action':'revise','decision':latest['decision'],'note':'合成另一位家人的修订','evidence':[],'caregiver_state':'easy'})
 click('保存复盘说明修订');expect(btn('备份填写并载入最新跟进')).to_be_visible();expect(field('复盘依据与说明')).to_have_value('合成我的未提交修订');click('备份填写并载入最新跟进');expect(page.get_by_role('combobox',name='这次安排对家人的负担（可选）',exact=True)).to_have_value('easy');expect(page.get_by_text('此前家人感受：还行',exact=True)).to_be_visible();field('复盘依据与说明').fill('合成核对后保留最新感受');click('保存复盘说明修订');expect(page.get_by_role('dialog',name='逐项复盘',exact=True)).to_have_count(0)
 after=call('GET','/prd/process?view=review_context&object_kind=goal&id='+g['id']);assert after['schedule']==schedule and after['last_review']['caregiver_state']=='easy';page.go_back()
 # Explicit due difficulties make deterministic queue groups, no date inference.
 for title,goals,methods in [('困难A',[g['id']],[]),('困难B',[g2['id']],[]),('跨目标困难',[],[m['id']])]:
  id=str(uuid4());process('knowledge',{'id':id,'operation_id':str(uuid4()),'expected_version':None,'action':'create','input':{'id':id,'kind':'difficulty','content':title,'goal_ids':goals,'method_ids':methods,'next_review_date':today},'details':{'title':title}})
 page.reload();expect(btn('记录上课')).to_be_enabled();click('跟进');btn(re.compile('^查看共享复盘队列')).click()
 expect(page.get_by_role('heading',name='围绕目标：轮流游戏',exact=True)).to_be_visible();expect(page.get_by_role('heading',name='围绕目标：主动表达',exact=True)).to_be_visible();expect(btn('开始复盘：跨目标困难')).to_have_count(1);expect(page.get_by_text(re.compile('还关联：.*不重复排期'))).to_be_visible();page.go_back()
 print('PASS grouped deduplicated queue, explicit concurrent context and optional caregiver state persisted',flush=True)
 click('记录');click('家庭干预');click('按目标查看时间分布');expect(page.get_by_role('combobox',name='时间分布粒度',exact=True)).to_have_value('day');expect(page.get_by_role('img',name=re.compile('近90天日分布'))).to_have_count(2)
 page.screenshot(path=str(OUT/'history-desktop.png'));click('查看时间分布：轮流游戏');expect(btn(re.compile('^查看记录依据：'))).to_have_count(1)
 page.set_viewport_size({'width':320,'height':740});expect(page.get_by_role('combobox',name='时间分布粒度',exact=True)).to_have_count(0);expect(page.get_by_text(re.compile('近90天 · 从左到右'))).to_be_visible();expect(btn(re.compile('^查看记录依据：'))).to_have_count(1)
 assert not page.evaluate('document.documentElement.scrollWidth > innerWidth');assert not errors,errors
 print('PASS desktop daily dots, mobile weekly bars, preserved record detail and narrow layout',flush=True)
 browser.close()
