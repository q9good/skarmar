"""Mobile family record regression. Operates only on a temporary localhost database."""
import os
from pathlib import Path
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import expect, sync_playwright

BASE = os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost', '127.0.0.1', '::1'}
OUT = Path('.local/family-record-1118')
OUT.mkdir(parents=True, exist_ok=True)
HEADERS = {'X-Skarma-Request': '1', 'X-Skarma-Contract': 'skarmar.prd.v1', 'X-Skarma-Data-Epoch': 'formal-20261101'}

with sync_playwright() as p:
    browser = p.chromium.launch(executable_path=os.getenv('CHROMIUM_EXECUTABLE'), headless=True, args=['--no-sandbox'])
    ctx = browser.new_context(viewport={'width': 390, 'height': 844})
    page = ctx.new_page()
    page_errors = []
    page.on('pageerror', lambda error: page_errors.append(str(error)))
    session_puts = []
    page.on('request', lambda req: session_puts.append(req.post_data_json) if req.method == 'PUT' and '/prd/sessions/' in req.url else None)

    def call(method, path, body=None, context=ctx):
        response = context.request.fetch(BASE + '/api' + path, method=method, data=body, headers=HEADERS)
        assert response.ok, (response.status, response.text())
        return response.json()

    def button(name):
        return page.get_by_role('button', name=name, exact=True)

    def checkbox(name, scope=page):
        return scope.get_by_role('checkbox', name=name, exact=True)

    def field(name, scope=page):
        return scope.get_by_role('textbox', name=name, exact=True)

    def login(target, username, password):
        target.goto(BASE)
        field('账号', target).fill(username)
        target.get_by_label('密码', exact=True).fill(password)
        target.get_by_role('button', name='登录', exact=True).click()
        expect(target.get_by_role('button', name='记录课堂', exact=True)).to_be_enabled()

    login(page, 'owner', 'synthetic-prd-ui-password')
    goal_id = str(uuid4())
    call('PUT', '/prd/records/' + goal_id, {'id': goal_id, 'operation_id': str(uuid4()), 'expected_version': None,
        'change_note': '', 'mutation': {'kind': 'goal', 'value': {'title': '合成轮流滚球', 'level': 'long'}}})
    for index in range(17):
        method_id = str(uuid4())
        call('PUT', '/prd/records/' + method_id, {'id': method_id, 'operation_id': str(uuid4()), 'expected_version': None,
            'change_note': '', 'method_create_mode': 'full', 'mutation': {'kind': 'method',
            'value': {'title': f'合成做法{index:02}', 'goal_ids': [goal_id], 'steps': '合成：等孩子发出邀请。'}}})
    page.reload()
    button('记录家庭').click()
    field('准备做什么').fill('合成：留待以后继续的独立计划')
    button('关闭填写干预计划').click()
    draft_link = page.get_by_role('button', name='继续干预填写：', exact=False)
    expect(draft_link).to_have_count(1)
    button('记录家庭').click()
    button('更多干预操作').click()
    button('已经做完：直接记总结').click()
    modal = page.get_by_role('dialog', name='填写干预总结', exact=True)
    button('选择本次目标').click()
    expect(page.get_by_role('dialog')).to_have_count(1)
    checkbox('选择目标：合成轮流滚球', modal).click()
    button('选择本次目标').click()
    field('实际表现：合成轮流滚球').fill('合成：等家人推球后主动轮流')
    button('实际方法').click()
    expect(page.get_by_role('dialog')).to_have_count(1)
    methods = modal.get_by_test_id('family-method-options')
    field('搜索实际方法', methods).fill('做法16')
    checkbox('实际方法：合成做法16', methods).click()
    field('搜索实际方法', methods).fill('')
    expect(checkbox('实际方法：合成做法16', methods)).to_be_checked()
    button('展开全部实际方法（17项）').click()
    expect(methods.get_by_role('checkbox', name='实际方法：', exact=False)).to_have_count(17)
    button('收起实际方法').click()
    checkbox('补充经验和困难（可选）').click()
    button('新增本次经验').click()
    field('经验内容 1').fill('合成经验：等邀请有帮助')
    page.get_by_role('combobox', name='经验阶段 1', exact=True).select_option('idea')
    button('调整知识关联 1').click()
    expect(page.get_by_role('dialog')).to_have_count(1)
    field('搜索关联方法').fill('做法15')
    checkbox('关联方法：合成做法15').click()
    expect(checkbox('关联方法：合成做法15')).to_be_checked()
    page.screenshot(path=str(OUT / '01-inline-association.png'))
    # Block storage cleanup and shared reads only AFTER the server accepts this write.
    # The completed card and its close behavior must not depend on either.
    def hold_cleanup(route):
        response = route.fetch()
        assert response.ok, response.text()
        page.evaluate("""() => {
            const original=navigator.locks.request.bind(navigator.locks);
            navigator.locks.request=(name,fn)=>original(name,async lock=>{
                if(name.includes('session-drafts'))await new Promise(r=>window.releaseCleanup=r);
                return fn(lock);
            });
            window.restoreLocks=()=>delete navigator.locks.request;
        }""")
        route.fulfill(response=response)

    def fail_refresh(route):
        route.fulfill(status=503, json={'error': '合成共享资料暂不可读'})

    page.route('**/api/prd/state', fail_refresh)
    page.route('**/api/prd/sessions/*', hold_cleanup, times=1)
    button('正式提交干预总结').click()
    page.wait_for_function("typeof window.releaseCleanup==='function'")
    expect(page.get_by_role('dialog')).to_have_count(0)
    title = '查看家庭干预：合成：等家人推球后主动轮流'
    expect(button(title)).to_have_count(1)
    expect(draft_link).to_have_count(1)
    assert len(session_puts) == 1
    saved_id = session_puts[0]['id']
    draft_receipt = session_puts[0]['operation_id']
    assert call('GET', '/prd/process?view=session_save_receipt&id=' + draft_receipt)['id'] == saved_id
    assert call('GET', '/prd/process?view=session_save_receipt&id=' + str(uuid4())) is None
    page.evaluate('window.restoreLocks();window.releaseCleanup()')
    page.unroute('**/api/prd/state', fail_refresh)
    page.reload()
    expect(button(title)).to_have_count(1)
    expect(draft_link).to_have_count(1)
    button(title).scroll_into_view_if_needed()
    page.screenshot(path=str(OUT / '02-completed-card.png'))

    call('POST', '/auth/users', {'username': 'other', 'display_name': '合成其他家人', 'password': 'synthetic-other-password', 'role': 'member'})
    other = browser.new_context()
    login(other.new_page(), 'other', 'synthetic-other-password')
    assert call('GET', '/prd/process?view=session_save_receipt&id=' + draft_receipt, context=other) is None
    other.close()

    button(title).click()
    button('修改已完成总结').click()
    field('实际表现：合成轮流滚球').fill('合成修订：轮流并主动请求再玩一次')
    field('本次修订说明').fill('合成：补记主动邀请')

    def lose_reply(route):
        response = route.fetch()
        assert response.ok, response.text()
        route.abort('failed')

    # Both the original response and first receipt read are unavailable.
    # A reload must recover the same operation without sending an extra write.
    receipt_glob = '**/api/prd/process?view=session_save_receipt&id=*'
    page.route(receipt_glob, lambda route: route.abort('failed'))
    page.route('**/api/prd/sessions/*', lose_reply, times=1)
    button('保存总结修订').click()
    expect(button('重试同一次干预保存')).to_be_visible()
    assert len(session_puts) == 2
    page.unroute(receipt_glob)
    page.reload()
    revised_title = '查看家庭干预：合成修订：轮流并主动请求再玩一次'
    expect(button(revised_title)).to_have_count(1)
    expect(page.get_by_text('已保存家庭记录，可打开这条记录继续编辑。', exact=False)).to_be_visible()
    expect(draft_link).to_have_count(1)
    assert len(session_puts) == 2

    # A request that has not committed remains available on the original item.
    button(revised_title).click()
    button('修改已完成总结').click()
    field('实际表现：合成轮流滚球').fill('合成再次修订：孩子请求了第二轮')
    field('本次修订说明').fill('合成再补记')
    page.route('**/api/prd/sessions/*', lambda route: route.abort('failed'), times=1)
    button('保存总结修订').click()
    expect(button('重试同一次干预保存')).to_be_visible()
    page.reload()
    button(revised_title).click()
    button('修改已完成总结').click()
    expect(field('实际表现：合成轮流滚球')).to_have_value('合成再次修订：孩子请求了第二轮')
    button('重试同一次干预保存').click()
    expect(page.get_by_role('dialog')).to_have_count(0)
    assert session_puts[2] == session_puts[3]
    shared = call('GET', '/prd/state')
    assert len(shared['sessions']) == 1 and shared['sessions'][0]['version'] == 3
    assert len(shared['knowledge']) == 1 and len(shared['knowledge'][0]['input']['method_ids']) == 2
    assert len(call('GET', '/prd/sessions/' + saved_id + '/history')) == 3
    expect(draft_link).to_have_count(1)
    draft_link.click()
    expect(field('准备做什么')).to_have_value('合成：留待以后继续的独立计划')
    assert not page_errors, page_errors
    browser.close()
    print('PASS F01–F03: delayed draft cleanup and failed refresh; receipt access bound to actor; no write on receipt recovery; exact retry; one editable family record and independent draft; inline goals/methods/knowledge with 17 options')
