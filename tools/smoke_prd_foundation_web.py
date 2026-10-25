"""PRD foundation UI regression against a fresh disposable localhost database only."""
import os
from pathlib import Path
from urllib.parse import urlparse
from uuid import uuid4
from playwright.sync_api import expect, sync_playwright

BASE = os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost', '127.0.0.1', '::1'}, 'Synthetic writes require localhost'
OUT = Path('.local/prd-foundation-validation/screenshots')
OUT.mkdir(parents=True, exist_ok=True)

with sync_playwright() as p:
    browser = p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'), headless=True, args=['--no-sandbox'])
    context = browser.new_context(viewport={'width': 360, 'height': 640})
    page = context.new_page()
    errors = []
    page.on('pageerror', lambda error: errors.append(str(error)))
    page.goto(BASE)
    expect(page.get_by_text('skarmar', exact=True)).to_be_visible()
    assert page.title().startswith('skarmar')
    page.get_by_role('textbox', name='账号', exact=True).fill('owner')
    page.get_by_label('密码', exact=True).fill('synthetic-owner-contract-password')
    page.get_by_role('button', name='登录', exact=True).click()
    expect(page.get_by_role('button', name='记录上课', exact=True)).to_be_visible()

    def write(method, path, body):
        response = context.request.fetch(BASE + '/api' + path, method=method, data=body,
                                        headers={'X-Skarma-Request': '1'})
        assert response.ok, (response.status, response.text())
        return response.json()

    def refresh():
        page.evaluate("window.dispatchEvent(new Event('focus'))")

    def first_screen():
        for name in ['记录上课', '记录一次家庭干预']:
            button = page.get_by_role('button', name=name, exact=True)
            expect(button).to_be_in_viewport(ratio=1)
            box = button.bounding_box()
            assert box['width'] >= 44 and box['height'] >= 44
        assert not page.get_by_role('dialog').count(), 'No automatic reminder overlay'
        assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')

    initial = context.request.get(BASE + '/api/state').json()
    assert not any(initial[k] for k in ['goals', 'sessions', 'lessons']), 'Use a fresh isolated synthetic database'
    first_screen()
    page.get_by_role('button', name='查看长期关注', exact=True).click()
    expect(page.get_by_role('dialog', name='长期关注', exact=True)).to_be_visible()
    page.get_by_role('button', name='关闭长期关注', exact=True).click()
    expect(page.get_by_role('button', name='查看长期关注', exact=True)).to_be_focused()

    goals = []
    for count in [3, 6]:
        while len(goals) < count:
            n = len(goals) + 1
            goals.append(write('POST', '/goals', {'id': str(uuid4()), 'title': f'测试长期方向{n}', 'level': 'long',
                'parent_id': None, 'area': '', 'criteria': f'测试方向{n}的关注说明'}))
        refresh()
        page.get_by_role('button', name='查看长期关注', exact=True).click()
        dialog = page.get_by_role('dialog', name='长期关注', exact=True)
        expect(dialog.get_by_text(f'测试长期方向{count}', exact=True)).to_have_count(1)
        expect(dialog.get_by_text('测试方向1的关注说明', exact=True)).to_be_visible()
        assert page.evaluate("document.querySelector('[role=dialog]').contains(document.activeElement)")
        page.go_back()
        expect(dialog).to_have_count(0)
        first_screen()
        page.screenshot(path=str(OUT / f'today-{count}-directions.png'))
    expect(page.get_by_role('button', name='查看长期关注', exact=True)).to_contain_text('另有 3 项')
    print('PASS 360x640 first screen at 0/3/6 directions; explicit overlay, focus restoration and browser Back', flush=True)

    # A due review remains visible inline and never obscures the main actions.
    goal = goals[0]
    write('PUT', '/goals/' + goal['id'], {**goal, 'expected_version': goal['version'],
        'next_review_date': '2020-01-01', 'change_note': '合成到期提醒'})
    refresh()
    expect(page.get_by_text('给我的待办', exact=True)).to_be_visible()
    first_screen()
    page.get_by_role('button', name='查看长期关注', exact=True).click()
    page.keyboard.press('Escape')
    expect(page.get_by_role('dialog')).to_have_count(0)

    # Plan choice is explicit; no accidental selection among multiple pending plans.
    page.get_by_role('button', name='记录一次家庭干预', exact=True).click()
    expect(page.get_by_role('dialog', name='家庭干预', exact=True)).to_be_visible()
    page.get_by_role('button', name='准备去做：新建计划', exact=True).click()
    expect(page.get_by_role('textbox', name='整次训练安排', exact=True)).to_be_visible()
    page.get_by_role('textbox', name='整次训练安排', exact=True).fill('测试未提交安排')
    page.get_by_role('button', name='今天', exact=True).click()
    page.get_by_role('button', name='继续填写', exact=True).click()
    expect(page.get_by_role('textbox', name='整次训练安排', exact=True)).to_have_value('测试未提交安排')
    page.get_by_role('button', name='今天', exact=True).click()
    print('PASS primary intervention entry and form retention across tab changes', flush=True)

    page.get_by_role('button', name='复盘：测试长期方向1', exact=True).click()
    page.get_by_role('button', name='修改／复盘目标', exact=True).click()
    label = '本次复盘日期（可选）'
    calendar = page.get_by_label(label + '：选择日期', exact=True)
    original = calendar.input_value()
    page.get_by_role('button', name=label + '：改用数字输入', exact=True).click()
    expect(calendar).to_have_count(0)
    digits = page.get_by_role('textbox', name=label, exact=True)
    expect(digits).to_have_value(original)
    digits.fill('20261107')
    expect(digits).to_have_value('2026-11-07')
    page.get_by_role('button', name=label + '：改用日历选择', exact=True).click()
    expect(calendar).to_have_value('2026-11-07')
    expect(digits).to_have_count(0)
    calendar.fill('2026-11-14')
    page.get_by_role('button', name=label + '：改用数字输入', exact=True).click()
    expect(digits).to_have_value('2026-11-14')
    digits.fill('2026-11-')
    page.get_by_role('button', name=label + '：改用日历选择', exact=True).click()
    page.get_by_role('button', name=label + '：改用数字输入', exact=True).click()
    expect(digits).to_have_value('2026-11-')
    assert float(digits.evaluate('e => getComputedStyle(e).fontSize').replace('px', '')) >= 16
    print('PASS one date input at a time; valid and unfinished values survive mode changes', flush=True)

    page.get_by_role('button', name='我的账号', exact=True).click()
    toggle = page.get_by_role('switch', name='大字模式', exact=True)
    toggle.click()
    expect(toggle).to_be_checked()
    page.reload()
    page.get_by_role('button', name='我的账号', exact=True).click()
    expect(page.get_by_role('switch', name='大字模式', exact=True)).to_be_checked()
    page.get_by_role('button', name='今天', exact=True).click()
    button = page.get_by_role('button', name='记录上课', exact=True)
    assert button.bounding_box()['height'] >= 56
    page.get_by_role('button', name='记录上课', exact=True).click()
    for field in page.locator('input,textarea,select').all():
        assert float(field.evaluate('e => getComputedStyle(e).fontSize').replace('px', '')) >= 18
    # Narrow screen reflow and a page-zoom simulation are separate from a real iPhone audit.
    for width, height, zoom in [(430, 932, '1'), (320, 740, '1'), (640, 900, '2')]:
        page.set_viewport_size({'width': width, 'height': height})
        page.evaluate('z => document.body.style.zoom = z', zoom)
        assert not page.evaluate('document.documentElement.scrollWidth > innerWidth'), (width, zoom)
    page.evaluate("document.body.style.zoom = '1'")
    page.set_viewport_size({'width': 430, 'height': 932})
    page.screenshot(path=str(OUT / 'lesson-large-text.png'))
    assert not errors, errors
    print('PASS persistent large text, 56px actions, 18px inputs, narrow reflow and page zoom simulation; no JS errors', flush=True)
    # Two shared drafts must require an explicit choice, without changing either version.
    from datetime import datetime
    from zoneinfo import ZoneInfo
    day = str(datetime.now(ZoneInfo('Asia/Shanghai')).date())
    pending = []
    for slot, plan in [('上午', '测试计划一'), ('晚上', '测试计划二')]:
        identifier = str(uuid4())
        result = write('PUT', '/sessions/' + identifier, {'operation_id': str(uuid4()), 'expected_version': None,
            'action': 'save_draft', 'session': {'business_date': day, 'time_slot': slot, 'activities': [], 'organization': '',
            'plan': plan, 'observation': '', 'targets': [], 'has_difficulty': False, 'difficulty': '',
            'has_experience': False, 'experience': '', 'experience_kind': 'task'}})
        pending.append(result['session'])
    page.get_by_role('button', name='今天', exact=True).click()
    refresh()
    page.get_by_role('button', name='填写今天的总结', exact=True).click()
    chooser = page.get_by_role('dialog', name='家庭干预', exact=True)
    expect(chooser).to_be_visible()
    for record in pending:
        expect(chooser.get_by_role('button', name=record['time_slot'] + ' · ' + record['plan'], exact=True)).to_be_visible()
    page.get_by_role('button', name='关闭家庭干预', exact=True).click()
    latest = context.request.get(BASE + '/api/state').json()['sessions']
    assert all(next(item for item in latest if item['id'] == old['id'])['version'] == old['version'] for old in pending)
    print('PASS two pending shared plans require explicit choice and keep both versions unchanged', flush=True)
    browser.close()
