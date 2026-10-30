"""Current PRD Web account lifecycle; fresh localhost database and synthetic users."""
import json, os
from pathlib import Path
from urllib.parse import urlparse
from playwright.sync_api import expect, sync_playwright

BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}
OUT=Path('.local/prd-account-review');OUT.mkdir(parents=True,exist_ok=True)
shots=[];errors=[]
with sync_playwright() as p:
    browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
    admin_ctx=browser.new_context(viewport={'width':390,'height':844},locale='zh-CN');admin=admin_ctx.new_page()
    member_ctx=browser.new_context(viewport={'width':390,'height':844},locale='zh-CN');member=member_ctx.new_page()
    for page in [admin,member]:page.on('pageerror',lambda e:errors.append(str(e)))
    def btn(page,name):return page.get_by_role('button',name=name,exact=True)
    def click(page,name):btn(page,name).click()
    def field(page,name):return page.get_by_role('textbox',name=name,exact=True)
    def login(page,user,password):
        page.goto(BASE);field(page,'账号').fill(user);page.get_by_label('密码',exact=True).fill(password);click(page,'登录');expect(btn(page,'记录课堂')).to_be_enabled()
    def shot(page,key,title,description,focus):
        focus.scroll_into_view_if_needed();page.screenshot(path=str(OUT/(key+'.png')))
        shots.append({'file':key+'.png','title':title,'section':'账号与初始设置','description':description,'width':390,'height':844})
    member.goto(BASE);click(member,'申请注册');field(member,'账号').fill('review-member');field(member,'称呼').fill('审核演示家人');field(member,'邮箱').fill('review-member@example.test');member.get_by_label('密码',exact=True).fill('s');member.get_by_label('再次输入密码',exact=True).fill('s');click(member,'提交注册申请')
    expect(member.get_by_text('注册申请已提交。管理员审批通过后，就可以使用刚设置的密码登录。',exact=True)).to_be_visible()
    shot(member,'registration-receipt','注册实际提交与待审批','实际用单字符合成密码提交成功，没有12位限制；审批前不能登录，邮箱仅作申请人提供的信息。',btn(member,'登录'))
    member.get_by_label('密码',exact=True).fill('s');click(member,'登录');expect(member.get_by_role('alert').last).to_be_visible();expect(btn(member,'记录课堂')).to_have_count(0)
    login(admin,'owner','synthetic-prd-ui-password');click(admin,'我的账号');expect(btn(admin,'批准')).to_be_visible()
    shot(admin,'account-approval','管理员审批注册申请','管理员实际批准申请后，原密码可登录；拒绝和账号停用／恢复均有独立验证。',btn(admin,'批准'))
    click(admin,'批准');expect(admin.get_by_text('审核演示家人 的申请已批准，可以登录。',exact=True)).to_be_visible()
    login(member,'review-member','s');click(member,'我的账号');expect(btn(member,'创建账号')).to_have_count(0);assert member_ctx.request.get(BASE+'/api/auth/users').status==403
    switch=member.get_by_role('switch',name='大字模式',exact=True);switch.click();expect(switch).to_be_checked()
    shot(member,'account-display','普通成员账号与本机大字模式','普通成员没有账号管理入口；本机大字设置重载后保留，其他设备不受影响。',switch)
    member.reload();expect(member.get_by_role('switch',name='大字模式',exact=True)).to_have_count(0);click(member,'我的账号');expect(member.get_by_role('switch',name='大字模式',exact=True)).to_be_checked();member.get_by_role('switch',name='大字模式',exact=True).click()
    row=admin.get_by_text('review-member · 成员 · 正常使用',exact=True).locator('..');row.get_by_role('button',name='停用账号',exact=True).click();expect(admin.get_by_text('review-member · 成员 · 已停用',exact=True)).to_be_visible()
    member.reload();expect(field(member,'账号')).to_be_visible();assert member_ctx.request.get(BASE+'/api/auth/me').status==401
    row=admin.get_by_text('review-member · 成员 · 已停用',exact=True).locator('..');row.get_by_role('button',name='启用账号',exact=True).click();expect(admin.get_by_text('review-member · 成员 · 正常使用',exact=True)).to_be_visible()
    row=admin.get_by_text('review-member · 成员 · 正常使用',exact=True).locator('..');row.get_by_role('button',name='重置密码',exact=True).click();admin.get_by_label('审核演示家人的新密码',exact=True).fill('t');row.get_by_role('button',name='确认重置密码',exact=True).click();expect(admin.get_by_label('审核演示家人的新密码',exact=True)).to_have_count(0)
    login(member,'review-member','t');click(member,'我的账号');member.get_by_label('当前密码',exact=True).fill('t');member.get_by_label('新密码',exact=True).fill('u');member.get_by_label('再次输入新密码',exact=True).fill('u');click(member,'更新密码');expect(field(member,'账号')).to_be_visible();login(member,'review-member','u');click(member,'我的账号');click(member,'退出登录');expect(field(member,'账号')).to_be_visible();login(member,'review-member','u')
    assert not any(x in member.evaluate('JSON.stringify(localStorage)') for x in ['synthetic-prd-ui-password'])
    # A rejected application cannot obtain an active account.
    r=member_ctx.request.post(BASE+'/api/auth/register',headers={'X-Skarma-Request':'1'},data={'username':'rejected-review','display_name':'审核演示拒绝申请','email':'rejected@example.test','password':'s'});assert r.ok,r.text()
    click(admin,'刷新');expect(btn(admin,'拒绝')).to_be_visible();click(admin,'拒绝');click(admin,'确认拒绝');expect(btn(admin,'拒绝')).to_have_count(0)
    # Existing administrator can still create a member directly.
    field(admin,'新成员账号').fill('created-review');field(admin,'成员称呼').fill('审核演示新增家人');admin.get_by_label('初始密码',exact=True).fill('s');click(admin,'创建账号');expect(admin.get_by_text('created-review · 成员 · 正常使用',exact=True)).to_be_visible()
    assert not errors,errors
    (OUT/'manifest.json').write_text(json.dumps({'synthetic':True,'shots':shots},ensure_ascii=False,indent=2)+'\n')
    print('PASS current Web registration, pending gate, approval/rejection, direct creation, member restrictions, deactivate/reactivate, reset/own-password change, logout/relogin and persisted large text',flush=True)
    browser.close()
