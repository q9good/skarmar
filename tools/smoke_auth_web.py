"""Only against the fresh disposable database started by check-host.mjs."""
import os
from pathlib import Path
from playwright.sync_api import expect, sync_playwright
from check_excel import read_workbook

BASE = os.environ['SKARMA_TEST_URL']
OWNER_PASSWORD = 'synthetic-owner-contract-password'
MEMBER_PASSWORD = 'synthetic-teacher-browser-password'
OUTPUT = Path('.local/screenshots')
OUTPUT.mkdir(parents=True, exist_ok=True)

with sync_playwright() as p:
    browser = p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE', '/usr/bin/chromium'), headless=True, args=['--no-sandbox'])
    context = browser.new_context(viewport={'width': 390, 'height': 844})
    page = context.new_page()
    errors = []
    page.on('pageerror', lambda error: errors.append(str(error)))
    page.goto(BASE)
    expect(page.get_by_text('欢迎回来', exact=True)).to_be_visible()
    assert context.request.get(BASE + '/api/state').status == 401
    assert context.request.get(BASE + '/api/export.xlsx').status == 401
    assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
    page.screenshot(path=str(OUTPUT / 'login.png'), full_page=True)

    def login(username, password):
        page.get_by_role('textbox', name='账号', exact=True).fill(username)
        page.get_by_label('密码', exact=True).fill(password)
        page.get_by_role('button', name='登录', exact=True).click()
        expect(page.get_by_text('把今天的小进步，', exact=False)).to_be_visible()

    applicant_password = 'synthetic-applicant-browser-password'
    page.get_by_role('button', name='申请注册', exact=True).click()
    expect(page.get_by_text('填写申请后由管理员审批，审批通过前无法登录。')).to_be_visible()
    page.get_by_role('textbox', name='账号', exact=True).fill('applicant')
    page.get_by_role('textbox', name='称呼', exact=True).fill('测试申请人')
    page.get_by_role('textbox', name='邮箱', exact=True).fill('applicant@example.test')
    page.get_by_label('密码', exact=True).fill(applicant_password)
    page.get_by_label('再次输入密码', exact=True).fill(applicant_password)
    page.get_by_role('button', name='提交注册申请', exact=True).click()
    expect(page.get_by_text('注册申请已提交。管理员审批通过后，就可以使用刚设置的密码登录。')).to_be_visible()
    page.get_by_label('密码', exact=True).fill(applicant_password)
    page.get_by_role('button', name='登录', exact=True).click()
    expect(page.get_by_text('账号或密码不正确', exact=True)).to_be_visible()

    page.get_by_role('textbox', name='账号', exact=True).fill('owner')
    page.get_by_label('密码', exact=True).fill('wrong')
    page.get_by_role('button', name='登录', exact=True).click()
    expect(page.get_by_text('账号或密码不正确', exact=True)).to_be_visible()
    login('owner', OWNER_PASSWORD)
    assert all(cookie['httpOnly'] for cookie in context.cookies() if 'skarma_session' in cookie['name'])
    page.get_by_role('button', name='我的账号', exact=True).click()
    expect(page.get_by_text('待审批申请 · 1', exact=True)).to_be_visible()
    expect(page.get_by_text('申请人填写的邮箱（未验证）：applicant@example.test', exact=False)).to_be_visible()
    page.get_by_role('button', name='批准', exact=True).click()
    expect(page.get_by_text('测试申请人 的申请已批准，可以登录。', exact=True)).to_be_visible()
    with page.expect_download() as export:
        page.get_by_role('button', name='导出 Excel', exact=True).click()
    assert export.value.suggested_filename.endswith('.xlsx')
    workbook = read_workbook(Path(export.value.path()).read_bytes())
    assert len(workbook['目标']) == 5
    assert workbook['训练记录'] == []
    expect(page.get_by_text('Excel 已交给浏览器下载，请在下载列表中查看。', exact=True)).to_be_visible()
    page.get_by_role('textbox', name='新成员账号', exact=True).fill('teacher')
    page.get_by_role('textbox', name='成员称呼', exact=True).fill('测试老师')
    page.get_by_label('初始密码', exact=True).fill(MEMBER_PASSWORD)
    page.get_by_role('button', name='创建账号', exact=True).click()
    expect(page.get_by_text('账号已创建，请私下告知成员登录信息。', exact=True)).to_be_visible()
    assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
    page.screenshot(path=str(OUTPUT / 'accounts.png'), full_page=True)
    delayed = []
    page.route('**/api/goals/*/history', lambda route: delayed.append(route))
    page.get_by_role('button', name='目标', exact=True).click()
    page.get_by_text('主动请求帮助', exact=True).click()
    page.wait_for_timeout(100)
    assert delayed, 'goal history should issue a protected request'
    page.get_by_role('button', name='我的账号', exact=True).click()
    page.get_by_role('button', name='退出登录', exact=True).click()
    expect(page.get_by_text('欢迎回来', exact=True)).to_be_visible()

    login('teacher', MEMBER_PASSWORD)
    for route in delayed:
        route.fulfill(status=401, content_type='application/json', body='{"error":"测试：上一账号请求已过期"}')
    page.unroute_all(behavior='wait')
    page.wait_for_timeout(100)
    expect(page.get_by_text('把今天的小进步，', exact=False)).to_be_visible()
    identity = context.request.get(BASE + '/api/auth/me').json()
    scope = f"{identity['space']}:{identity['user']['id']}"
    page.get_by_role('button', name='我的账号', exact=True).click()
    assert page.get_by_role('button', name='创建账号', exact=True).count() == 0
    assert context.request.get(BASE + '/api/auth/users').status == 403
    page.get_by_role('button', name='返回', exact=True).click()
    page.get_by_role('button', name='＋ 新建训练计划', exact=True).click()
    page.get_by_role('textbox', name='整次训练安排', exact=True).fill('这个本机草稿属于老师账号')
    expect(page.get_by_text('草稿已保存在本机', exact=False)).to_be_visible()
    page.get_by_role('button', name='我的账号', exact=True).click()

    # Members may export saved records, without including their unsaved browser draft.
    with page.expect_download() as export:
        page.get_by_role('button', name='导出 Excel', exact=True).click()
    workbook = read_workbook(Path(export.value.path()).read_bytes())
    assert '这个本机草稿属于老师账号' not in str(workbook)
    downloads = []
    page.on('download', lambda download: downloads.append(download))
    page.route('**/api/export.xlsx', lambda route: route.fulfill(status=200, content_type='text/html', body='<html>gateway</html>'))
    page.get_by_role('button', name='导出 Excel', exact=True).click()
    expect(page.get_by_text('下载未完成，请稍后重试', exact=True)).to_be_visible()
    assert not downloads, 'a gateway page must not be saved as an Excel file'
    page.unroute_all(behavior='wait')

    # If revocation cannot be confirmed, keep the session visible and let the user retry.
    page.route('**/api/auth/logout', lambda route: route.fulfill(status=503, content_type='application/json', body='{"error":"测试：暂时无法退出"}'))
    page.get_by_role('button', name='退出登录', exact=True).click()
    expect(page.get_by_text('测试：暂时无法退出', exact=True)).to_be_visible()
    page.unroute_all(behavior='wait')
    page.get_by_role('button', name='退出登录', exact=True).click()
    expect(page.get_by_text('欢迎回来', exact=True)).to_be_visible()
    login('owner', OWNER_PASSWORD)
    assert page.get_by_text('继续你的草稿', exact=True).count() == 0
    page.get_by_role('button', name='我的账号', exact=True).click()
    page.get_by_role('button', name='退出登录', exact=True).click()
    expect(page.get_by_text('欢迎回来', exact=True)).to_be_visible()
    login('teacher', MEMBER_PASSWORD)
    page.get_by_role('button', name='继续填写', exact=True).click()
    expect(page.get_by_role('textbox', name='整次训练安排', exact=True)).to_have_value('这个本机草稿属于老师账号')
    page.get_by_role('button', name='返回', exact=True).click()
    result = context.request.post(BASE + '/api/auth/logout', headers={'X-Skarma-Request': '1'})
    assert result.ok
    page.get_by_role('button', name='我的账号', exact=True).click()
    page.get_by_role('button', name='导出 Excel', exact=True).click()
    expect(page.get_by_text('欢迎回来', exact=True)).to_be_visible()
    assert page.get_by_text('把今天的小进步，', exact=False).count() == 0
    assert page.evaluate('(key) => localStorage.getItem(key)', f'skarma:{scope}:training-draft:v2')
    assert not page.evaluate('(password) => JSON.stringify(localStorage).includes(password)', MEMBER_PASSWORD)
    assert not errors, errors
    browser.close()
print('PASS mobile login/export: roles, Excel download, unsaved-draft exclusion, gateway failure, draft recovery and expired-session gating')
