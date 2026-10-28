"""Bidirectional course/teacher suggestions through local synthetic records only."""
import os
from datetime import datetime, timedelta
from urllib.parse import urlparse
from uuid import uuid4
from zoneinfo import ZoneInfo
from playwright.sync_api import expect, sync_playwright

BASE = os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost', '127.0.0.1', '::1'}
today = datetime.now(ZoneInfo('Asia/Shanghai')).date()
with sync_playwright() as p:
    browser = p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'), headless=True, args=['--no-sandbox'])
    setup = browser.new_context()
    owner = setup.new_page()
    errors = []

    def button(scope, name):
        return scope.get_by_role('button', name=name, exact=True)

    def login(page, user):
        page.goto(BASE)
        page.get_by_role('textbox', name='账号', exact=True).fill(user)
        page.get_by_label('密码', exact=True).fill('synthetic-prd-ui-password')
        button(page, '登录').click()
        expect(button(page, '记录上课')).to_be_enabled()

    def call(method, path, body=None):
        response = setup.request.fetch(BASE + '/api' + path, method=method, data=body, headers={'X-Skarma-Request': '1', 'X-Skarma-Contract': 'skarmar.prd.v1'})
        assert response.ok, (response.status, response.text())
        return response.json()

    def catalog(kind, name):
        id = str(uuid4())
        return call('PUT', '/prd/records/' + id, {'id': id, 'operation_id': str(uuid4()), 'expected_version': None, 'change_note': '', 'mutation': {'kind': 'catalog', 'value': {'kind': kind, 'name': name, 'active': True}}})['value']

    def lesson(course, teacher, days):
        id = str(uuid4())
        return call('PUT', '/prd/lessons/' + id, {'id': id, 'operation_id': str(uuid4()), 'expected_version': None, 'action': {'kind': 'observation', 'value': {'business_date': (today - timedelta(days=days)).isoformat(), 'course_id': course['id'], 'teacher_id': teacher['id'], 'notes': ''}}})

    login(owner, 'owner')
    assert not call('GET', '/prd/state')['lessons'], 'Use a fresh synthetic database'
    call('POST', '/auth/users', {'username': 'family', 'display_name': '合成排序家人', 'password': 'synthetic-prd-ui-password', 'role': 'member'})
    courses = [catalog('course', name) for name in ['合成关联课甲', '合成关联课乙', '合成总体热门课']]
    paired, alternate, popular, unused = [catalog('teacher', name) for name in ['合成关联老师', '合成另一老师', '合成总体热门老师', '合成从未关联老师']]
    for _ in range(3):
        lesson(courses[0], paired, 90)
    for _ in range(4):
        lesson(courses[1], paired, 2)
    for _ in range(9):
        lesson(courses[2], popular, 1)
    lesson(courses[0], alternate, 1)
    # Old snapshots still use the former name, while ranking follows the stable ID.
    paired = call('PUT', '/prd/records/' + paired['id'], {'id': paired['id'], 'operation_id': str(uuid4()), 'expected_version': 1, 'change_note': '合成改名', 'mutation': {'kind': 'catalog', 'value': {**paired['input'], 'name': '合成关联老师新称呼'}}})['value']
    teacher_names = [c['input']['name'] for c in [paired, alternate, popular, unused]]
    course_names = [c['input']['name'] for c in courses]
    ctx = browser.new_context(viewport={'width': 360, 'height': 740})
    page = ctx.new_page()
    page.on('pageerror', lambda e: errors.append(str(e)))
    login(page, 'family')

    def picker(kind, selected=''):
        label = '课程名称（必填）' if kind == '课程' else '授课老师（必填）'
        button(page, label + '：' + (selected or '请选择' + kind)).click()
        panel = page.get_by_role('dialog', name='选择' + kind, exact=True)
        expect(panel).to_be_visible()
        expect(panel.get_by_role('textbox', name='搜索' + kind, exact=True)).to_have_value('')
        return panel

    def order(panel, names):
        return [name.strip() for name in panel.get_by_role('button').all_text_contents() if name.strip() in names]

    def choose(kind, name, selected=''):
        button(picker(kind, selected), name).click()

    button(page, '记录上课').click()
    choose('课程', course_names[0])
    expect(button(page, '授课老师（必填）：请选择老师')).to_be_visible()
    assert order(page.get_by_role('dialog', name='记录上课', exact=True), teacher_names) == teacher_names[:3]
    panel = picker('老师')
    assert order(panel, teacher_names) == teacher_names, order(panel, teacher_names)
    expect(panel.get_by_text('按这门课程的历史授课次数排序', exact=True)).to_be_visible()
    # Opening/searching does not change either field; an unseen pairing is allowed.
    panel.get_by_role('textbox', name='搜索老师', exact=True).fill('从未关联')
    assert order(panel, teacher_names) == [teacher_names[3]]
    button(panel, teacher_names[3]).click()
    expect(button(page, '课程名称（必填）：' + course_names[0])).to_be_visible()
    choose('课程', course_names[1], course_names[0])
    expect(button(page, '授课老师（必填）：' + teacher_names[3])).to_be_visible()
    choose('老师', teacher_names[0], teacher_names[3])
    panel = picker('课程', course_names[1])
    assert order(panel, course_names) == [course_names[1], course_names[0], course_names[2]]
    expect(panel.get_by_text('按这位老师的历史上课次数排序', exact=True)).to_be_visible()
    button(panel, course_names[0]).click()
    panel = picker('老师', teacher_names[0])
    assert order(panel, teacher_names) == teacher_names, order(panel, teacher_names)
    # Background synchronization must not move options under the user's finger.
    for _ in range(3):
        lesson(courses[0], alternate, 0)
    with page.expect_response(lambda response: response.url.endswith('/api/prd/state') and response.ok):
        page.evaluate("window.dispatchEvent(new Event('focus'))")
    assert order(panel, teacher_names) == teacher_names
    button(page, '关闭选择老师').click()
    panel = picker('老师', teacher_names[0])
    assert order(panel, teacher_names) == [teacher_names[1], teacher_names[0], teacher_names[2], teacher_names[3]]
    # Selecting a different course never clears the teacher or fixes a pairing.
    button(panel, teacher_names[0]).click()
    panel = picker('课程', course_names[0])
    panel.get_by_role('textbox', name='搜索课程', exact=True).fill('总体热门')
    button(panel, course_names[2]).click()
    expect(button(page, '授课老师（必填）：' + teacher_names[0])).to_be_visible()
    button(page, '提交记录').click()
    expect(page.get_by_role('dialog')).to_have_count(0)
    expect(button(page, '检查连接并继续上传')).to_have_count(0, timeout=20000)
    saved = [row for row in call('GET', '/prd/state')['lessons'] if row['creator_name'] == '合成排序家人']
    assert len(saved) == 1 and saved[0]['course']['id'] == courses[2]['id'] and saved[0]['teacher']['id'] == paired['id']
    button(page, '记录上课').click()
    choose('老师', teacher_names[0])
    expect(button(page, '课程名称（必填）：请选择课程')).to_be_visible()
    assert order(page.get_by_role('dialog', name='记录上课', exact=True), course_names) == [course_names[1], course_names[0], course_names[2]]
    panel = picker('课程')
    assert order(panel, course_names) == [course_names[1], course_names[0], course_names[2]]
    assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
    assert not errors, errors
    print('PASS bidirectional pair frequency, full history and renamed IDs, all options/search, unchanged selections, frozen open order, fresh reopen and explicit saved pairing', flush=True)
    browser.close()
