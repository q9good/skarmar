"""New-model UI saves and conflicts on a disposable localhost PRD database."""
import os
from pathlib import Path
from urllib.parse import urlparse
from playwright.sync_api import expect, sync_playwright
BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/prd-ui-connect-screenshots');OUT.mkdir(parents=True,exist_ok=True)
with sync_playwright() as p:
    browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
    first=browser.new_context(viewport={'width':360,'height':640});second=browser.new_context(viewport={'width':430,'height':932})
    a,b=first.new_page(),second.new_page();errors=[]
    for page in [a,b]:page.on('pageerror',lambda e:errors.append(str(e)))
    def login(page,user,password):
        page.goto(BASE);page.get_by_role('textbox',name='账号',exact=True).fill(user);page.get_by_label('密码',exact=True).fill(password);page.get_by_role('button',name='登录',exact=True).click()
        page.get_by_role('button',name='目标',exact=True).click()
        expect(page.get_by_role('button',name='新建目标',exact=True)).to_be_visible()
    def write(path,body):
        response=first.request.post(BASE+'/api'+path,data=body,headers={'X-Skarma-Request':'1'});assert response.ok,response.text();return response.json()
    def state():
        response=first.request.get(BASE+'/api/prd/state');assert response.ok,response.text();return response.json()
    def save(page):
        page.get_by_role('button',name='保存',exact=True).click();expect(page.get_by_role('dialog')).to_have_count(0)
    def nav(page,title):
        if title=='课程和老师':
            page.get_by_role('button',name='我的账号',exact=True).click();page.get_by_role('button',name='维护课程和老师选项',exact=True).click()
        else:page.get_by_role('button',name=title,exact=True).click()
    login(a,'owner','synthetic-prd-ui-password')
    assert not state()['goals'],'Use a fresh disposable PRD database'
    write('/auth/users',{'username':'family','display_name':'测试家人','password':'s','role':'member'})
    a.get_by_role('button',name='新建目标',exact=True).click()
    a.get_by_role('textbox',name='本领名称',exact=True).fill('测试用筷子')
    a.get_by_role('button',name='补充目标说明（可选）',exact=True).click()
    a.get_by_role('textbox',name='生活意义',exact=True).fill('测试：更独立地吃饭')
    a.get_by_role('combobox',name='观察方式',exact=True).select_option('count')
    criterion=a.get_by_role('textbox',name='观察标准',exact=True);expect(criterion).to_have_value('')
    criterion.fill('测试：5次机会中有3次只需提醒')
    a.get_by_role('combobox',name='观察方式',exact=True).select_option('duration');expect(criterion).to_have_value('测试：5次机会中有3次只需提醒')
    a.get_by_role('textbox',name='预期帮助条件（可选）',exact=True).fill('测试：先不提示，需要时再示范')
    a.get_by_role('button',name='领域／可选：选择或自定义',exact=True).click()
    domain_panel=a.get_by_role('dialog',name='选择领域',exact=True);domain_panel.get_by_role('textbox',name='自定义领域',exact=True).fill('测试沟通互动');domain_panel.get_by_role('button',name='保存并选用',exact=True).click()
    expect(a.get_by_role('button',name='领域／可选：测试沟通互动',exact=True)).to_be_visible()
    a.get_by_role('button',name='目标日期／可选：改用数字输入',exact=True).click();a.get_by_role('textbox',name='目标日期／可选',exact=True).fill('20270120')
    save(a);short=next(g for g in state()['goals'] if g['input']['title']=='测试用筷子')
    assert short['input']['support']=='测试：先不提示，需要时再示范'
    assert short['input']['due']=='2027-01-20'
    assert short['input']['domain']['name']=='测试沟通互动' and short['input']['domain']['kind']=='domain'
    assert len([c for c in state()['catalog'] if c['input']['kind']=='domain'])==1
    a.get_by_role('button',name='新建目标',exact=True).click();a.get_by_role('textbox',name='本领名称',exact=True).fill('测试长期方向')
    a.get_by_role('combobox',name='目标类型',exact=True).select_option('long');save(a)
    long=next(g for g in state()['goals'] if g['input']['level']=='long');assert long['input']['criteria']==''
    nav(a,'方法');a.get_by_role('button',name='新建方法',exact=True).click();a.get_by_role('textbox',name='方法名称',exact=True).fill('测试等待示范')
    for title in ['测试用筷子','测试长期方向']:a.get_by_role('checkbox',name=title,exact=True).click()
    save(a);method=state()['methods'][0];assert set(method['input']['goal_ids'])=={short['id'],long['id']}
    assert method['input']['description']==''
    nav(a,'目标');expect(a.get_by_role('button',name='关联方法：测试等待示范',exact=True)).to_have_count(2)
    nav(a,'方法');a.get_by_role('button',name='编辑方法：测试等待示范',exact=True).click()
    a.get_by_role('textbox',name='具体做法（可后补）',exact=True).fill('测试：先示范，再等待')
    save(a);assert state()['methods'][0]['version']==2
    a.get_by_role('button',name='查看方法：测试等待示范',exact=True).click();expect(a.get_by_role('dialog').get_by_text('测试：先示范，再等待',exact=True)).to_have_count(2)
    a.get_by_role('button',name='关闭查看详情与修改记录',exact=True).click()
    print('PASS goal templates preserve text; name-only methods and two-way goal associations persist; method versions visible',flush=True)
    nav(a,'课程和老师');a.get_by_role('button',name='新增老师',exact=True).click();a.get_by_role('textbox',name='老师名称',exact=True).fill('测试老师旧名');save(a)
    teacher=next(c for c in state()['catalog'] if c['input']['kind']=='teacher')
    a.get_by_role('button',name='维护选项：测试老师旧名',exact=True).click();a.get_by_role('textbox',name='老师名称',exact=True).fill('测试老师新名');save(a)
    for _ in range(2):
        a.get_by_role('button',name='维护选项：测试老师新名',exact=True).click();a.get_by_role('checkbox',name='可在后续记录中选择',exact=True).click();save(a)
    current_teacher=next(c for c in state()['catalog'] if c['id']==teacher['id'])
    assert current_teacher['version']==4
    a.get_by_role('button',name='查看改名历史：测试老师新名',exact=True).click();expect(a.get_by_role('dialog').get_by_text('测试老师旧名',exact=True)).to_be_visible()
    a.get_by_role('button',name='关闭查看详情与修改记录',exact=True).click()
    print('PASS shared catalog rename, stop/restore, stable ID and old name history',flush=True)
    login(b,'family','s');nav(a,'目标');a.get_by_role('button',name='编辑目标：测试用筷子',exact=True).click()
    a.get_by_role('textbox',name='生活意义',exact=True).fill('测试：我的未提交内容')
    b.get_by_role('button',name='编辑目标：测试用筷子',exact=True).click();b.get_by_role('textbox',name='生活意义',exact=True).fill('测试：第二位家人更新');save(b)
    a.evaluate("window.dispatchEvent(new Event('focus'))")
    expect(a.get_by_text('另一位家人已修改这条记录，你的填写仍保留。',exact=True)).to_be_visible()
    expect(a.get_by_role('textbox',name='生活意义',exact=True)).to_have_value('测试：我的未提交内容')
    expect(a.get_by_text('生活意义：测试：第二位家人更新',exact=True)).to_be_visible()
    expect(a.get_by_role('button',name='保存',exact=True)).to_be_disabled()
    a.get_by_role('button',name='备份我的填写并采用最新版本',exact=True).click();expect(a.get_by_role('textbox',name='生活意义',exact=True)).to_have_value('测试：第二位家人更新')
    expect(a.get_by_text('生活意义：测试：我的未提交内容',exact=True)).to_be_visible()
    a.get_by_role('textbox',name='生活意义',exact=True).fill('测试：核对后合并');save(a)
    assert next(g for g in state()['goals'] if g['id']==short['id'])['version']==3
    print('PASS two members: latest version never overwrites active input; conflict blocks saving until explicit reconciliation',flush=True)
    # Simulate a lost response after the backend committed, then retry exactly once.
    a.get_by_role('button',name='新建目标',exact=True).click();a.get_by_role('textbox',name='本领名称',exact=True).fill('测试响应丢失')
    def lose_response(route):
        response=route.fetch();assert response.ok;route.abort('failed')
    a.route('**/api/prd/records/*',lose_response,times=1)
    a.get_by_role('button',name='保存',exact=True).click();expect(a.get_by_role('button',name='重试同一次保存',exact=True)).to_be_visible()
    committed=next(g for g in state()['goals'] if g['input']['title']=='测试响应丢失');assert committed['version']==1
    a.reload()
    a.get_by_role('button',name='继续填写：测试响应丢失',exact=True).click()
    expect(a.get_by_role('button',name='重试同一次保存',exact=True)).to_be_enabled()
    a.get_by_role('button',name='重试同一次保存',exact=True).click();expect(a.get_by_role('dialog')).to_have_count(0)
    history=first.request.get(BASE+'/api/prd/records/'+committed['id']+'/history').json();assert len(history)==1
    print('PASS lost response after commit: persisted operation retries without duplicate record/history',flush=True)
    nav(a,'目标')
    for title in ['测试草稿甲','测试草稿乙']:
        a.get_by_role('button',name='新建目标',exact=True).click();a.get_by_role('textbox',name='本领名称',exact=True).fill(title);a.get_by_role('button',name='关闭填写目标',exact=True).click()
    a.reload()
    for title in ['测试草稿甲','测试草稿乙']:expect(a.get_by_role('button',name='继续填写：'+title,exact=True)).to_be_visible()
    a.get_by_role('button',name='继续填写：测试草稿甲',exact=True).click();expect(a.get_by_role('textbox',name='本领名称',exact=True)).to_have_value('测试草稿甲')
    a.screenshot(path=str(OUT/'goal-draft-mobile.png'))
    a.get_by_role('button',name='关闭填写目标',exact=True).click();a.get_by_role('button',name='我的账号',exact=True).click()
    expect(a.get_by_role('button',name='导出 Excel',exact=True)).to_be_visible()
    a.get_by_role('button',name='退出登录',exact=True).click();login(a,'family','s')
    assert a.get_by_role('button',name='继续填写：测试草稿甲',exact=True).count()==0
    a.set_viewport_size({'width':320,'height':740});assert not a.evaluate('document.documentElement.scrollWidth > innerWidth')
    assert not errors,errors
    print('PASS multiple drafts survive refresh; account switch isolates local content; narrow viewport has no horizontal overflow',flush=True)
    browser.close()
