"""Explicit starter setup and editable references in a disposable local space."""
import os
from pathlib import Path
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import expect, sync_playwright

BASE = os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost', '127.0.0.1', '::1'}
OUT = Path('.local/prd-starter-ui')
OUT.mkdir(parents=True, exist_ok=True)
with sync_playwright() as p:
    browser = p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'), headless=True, args=['--no-sandbox'])
    admin_ctx = browser.new_context(viewport={'width': 390, 'height': 844})
    admin = admin_ctx.new_page()
    errors = []

    def button(scope, title):
        return scope.get_by_role('button', name=title, exact=True)

    def click(page, title):
        button(page, title).click()

    def field(page, title):
        return page.get_by_role('textbox', name=title, exact=True)

    def login(page, username):
        page.on('pageerror', lambda e: errors.append(str(e)))
        page.goto(BASE)
        field(page, '账号').fill(username)
        page.get_by_label('密码', exact=True).fill('synthetic-prd-ui-password')
        click(page, '登录')
        expect(button(page, '记录上课')).to_be_enabled()

    def call(method, path, body=None):
        r = admin_ctx.request.fetch(BASE + '/api' + path, method=method, data=body, headers={'X-Skarma-Request': '1', 'X-Skarma-Contract': 'skarmar.prd.v1'})
        assert r.ok, (r.status, r.text())
        return r.json()

    def state():
        return call('GET', '/prd/state')

    login(admin, 'owner')
    assert not state()['goals']
    call('POST', '/auth/users', {'username': 'family', 'display_name': '合成参考家人', 'password': 'synthetic-prd-ui-password', 'role': 'member'})
    click(admin, '我的账号')
    click(admin, '初始内容与参考')
    click(admin, '生成初始内容预览')
    expect(admin.get_by_text('初始内容写入预览', exact=True)).to_be_visible()
    assert not state()['goals'] and not state()['catalog']
    # Optional methods are not silently selected with the goals/catalogs.
    click(admin, '参考方法（10项，按需选择）')
    expect(admin.get_by_role('checkbox', name='先建立注意，再发出指令', exact=True)).not_to_be_checked()
    click(admin, '选取全部参考方法及关联方向')
    click(admin, '生成初始内容预览')
    expect(admin.get_by_text('初始内容写入预览', exact=True)).to_be_visible()

    def lose_commit(route):
        if route.request.post_data_json.get('kind') == 'commit':
            response = route.fetch()
            assert response.ok
            route.abort('failed')
        else:
            route.continue_()

    admin.route('**/api/data-transfer', lose_commit)
    click(admin, '确认按预览创建')
    expect(admin.get_by_role('alert')).to_be_visible()
    s = state()
    assert (len(s['goals']), len(s['methods']), len(s['catalog'])) == (6, 10, 10)
    admin.unroute('**/api/data-transfer', lose_commit)
    click(admin, '确认按预览创建')
    expect(admin.get_by_text('已确认：新增 26 项，使用已有 0 项，跳过 0 项。', exact=True)).to_be_visible()
    assert state() == s
    assert not any(s[k] for k in ['lessons', 'sessions', 'knowledge', 'actions'])
    assert all(g['input']['level'] == 'long' for g in s['goals'])
    g = next(g for g in s['goals'] if g['input']['title'] == '内在动机与主动探索')
    assert '阶段观察' in g['input']['criteria'] and '探索本身' in g['input']['meaning']
    m = next(m for m in s['methods'] if m['input']['title'] == '先建立注意，再发出指令')
    assert m['input']['goal_ids'] == [next(g['id'] for g in s['goals'] if g['input']['title'] == '注意力与学习效能')]
    print('PASS explicit administrator preview, optional methods, six long goals/ten methods/ten independent options, lost-response replay and no invented results', flush=True)

    ctx = browser.new_context(viewport={'width': 360, 'height': 740})
    page = ctx.new_page()
    login(page, 'family')
    click(page, '我的账号')
    expect(button(page, '初始内容与参考')).to_have_count(0)
    click(page, '返回')
    click(page, '目标')
    click(page, '新建目标')
    click(page, '参考长期方向')
    click(page, '查看参考：内在动机与主动探索')
    expect(page.get_by_role('dialog', name='参考长期方向', exact=True)).to_be_visible()
    page.screenshot(path=str(OUT / 'goal-reference.png'))
    click(page, '关闭参考长期方向')
    expect(field(page, '本领名称')).to_have_value('')
    click(page, '参考长期方向')
    click(page, '查看参考：内在动机与主动探索')
    click(page, '使用参考填写')
    expect(field(page, '本领名称')).to_have_value('内在动机与主动探索')
    expect(page.get_by_role('combobox', name='目标类型', exact=True)).to_have_value('long')
    # Further reference use forks an independent draft; previous typing survives.
    field(page, '本领名称').fill('合成保留的原目标')
    click(page, '参考长期方向')
    click(page, '查看参考：日常生活逐渐自主')
    click(page, '按参考新建，保留当前草稿')
    expect(field(page, '本领名称')).to_have_value('日常生活逐渐自主')
    field(page, '本领名称').fill('合成个体化生活目标')
    click(page, '保存')
    expect(page.get_by_role('dialog')).to_have_count(0)
    page.reload()
    expect(button(page, '记录上课')).to_be_enabled()
    page.wait_for_function("Object.keys(localStorage).some(k=>k.endsWith('prd-drafts:v1') && Object.values(JSON.parse(localStorage.getItem(k))).some(d=>d.mutation.value.title==='合成保留的原目标'))")
    assert len(state()['goals']) == 7
    own = next(g for g in state()['goals'] if g['input']['title'] == '合成个体化生活目标')
    click(page, '方法')
    click(page, '新建方法')
    click(page, '选择方法适用目标（已选0）')
    page.get_by_role('checkbox', name='合成个体化生活目标', exact=True).click()
    click(page, '关闭选择方法适用目标')
    click(page, '参考做法')
    click(page, '查看参考：拆步骤示范，再逐渐少帮')
    click(page, '使用参考填写')
    expect(field(page, '具体做法／必填')).not_to_have_value('')
    field(page, '方法名称').fill('合成个体化少帮助')
    click(page, '保存')
    expect(page.get_by_role('dialog')).to_have_count(0)
    own_method = next(m for m in state()['methods'] if m['input']['title'] == '合成个体化少帮助')
    assert own_method['input']['goal_ids'] == [own['id']]
    print('PASS member references: viewing never fills/saves, explicit editable goal draft, existing text retained across reload, method preserves only human-selected associations', flush=True)

    # Re-preview must preserve later edits, current names and inactive state.
    call('PUT', '/prd/records/' + g['id'], {'id': g['id'], 'operation_id': str(uuid4()), 'expected_version': 1, 'change_note': '合成个体化', 'mutation': {'kind': 'goal', 'value': {**g['input'], 'title': '合成已调整的长期方向'}}})
    c = s['catalog'][0]
    call('PUT', '/prd/records/' + c['id'], {'id': c['id'], 'operation_id': str(uuid4()), 'expected_version': 1, 'change_note': '合成停用', 'mutation': {'kind': 'catalog', 'value': {**c['input'], 'active': False}}})
    before = state()
    click(admin, '重新核对初始内容预览')
    expect(button(admin, '确认按预览创建')).to_be_disabled()
    assert state() == before
    assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
    assert not errors, errors
    print('PASS later seed preview leaves edits and inactive catalog unchanged, ordinary-member setup hidden, 360px layout and no browser errors', flush=True)
    browser.close()
