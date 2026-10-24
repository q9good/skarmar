"""Purpose/pathway/experience editing on the disposable authenticated host."""
import os
import re
from pathlib import Path
from playwright.sync_api import expect, sync_playwright
from check_excel import read_workbook

BASE = os.environ['SKARMA_TEST_URL']
OUTPUT = Path('.local/screenshots')
OUTPUT.mkdir(parents=True, exist_ok=True)

with sync_playwright() as p:
    browser = p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE', '/usr/bin/chromium'), headless=True, args=['--no-sandbox'])
    context = browser.new_context(viewport={'width': 390, 'height': 844})
    page = context.new_page()
    errors = []
    page.on('pageerror', lambda error: errors.append(str(error)))
    page.goto(BASE)
    page.get_by_role('textbox', name='账号', exact=True).fill('owner')
    page.get_by_label('密码', exact=True).fill('synthetic-owner-contract-password')
    page.get_by_role('button', name='登录', exact=True).click()
    expect(page.get_by_role('button', name='记录上课', exact=True)).to_be_visible()

    def state():
        response = context.request.get(BASE + '/api/state')
        assert response.ok
        return response.json()

    def put(path, body):
        response = context.request.put(BASE + '/api' + path, data=body, headers={'X-Skarma-Request': '1'})
        assert response.ok, response.text()
        return response.json()

    initial = state()
    purpose = next(goal for goal in initial['goals'] if goal['level'] == 'long')
    other = next(goal for goal in initial['goals'] if goal['level'] == 'short' and goal['status'] == 'active')
    title = '【验收】视觉提示'
    second_title = '【验收】共同阅读'
    paths = []
    for name in [title, second_title]:
        page.get_by_role('button', name='方法', exact=True).click()
        page.get_by_role('button', name='＋ 新建方法', exact=True).click()
        page.get_by_role('textbox', name='方法名称', exact=True).fill(name)
        page.get_by_role('textbox', name='作用说明', exact=True).fill('帮助理解并表达需要')
        page.get_by_role('textbox', name='具体步骤', exact=True).fill('展示选项\n留出等待时间')
        page.get_by_role('checkbox', name=re.compile(re.escape(purpose['title']) + ' · 长期')).click()
        page.get_by_role('checkbox', name=re.compile(re.escape(other['title']) + ' · 短期')).click()
        page.get_by_role('button', name='保存方法', exact=True).click()
        expect(page.get_by_text('方法已保存。', exact=True)).to_be_visible()
        paths.append(next(path for path in state()['pathways'] if path['title'] == name))
    assert all(set(path['goal_ids']) == {purpose['id'], other['id']} for path in paths)

    principle = '【验收】先理解沟通需要，再选择支持方式'
    detail = '【验收】每次只展示两张图片'
    both = '【验收】同时关联原则与操作细节'
    for content, with_purpose, with_pathway in [(principle, True, False), (detail, False, True), (both, True, True)]:
        page.get_by_role('button', name='跟进', exact=True).click()
        page.get_by_role('button', name='＋ 新建经验', exact=True).click()
        page.get_by_role('textbox', name='经验内容', exact=True).fill(content)
        if with_purpose:
            page.get_by_role('checkbox', name=re.compile(re.escape(purpose['title']) + ' · 长期')).click()
        if with_pathway:
            page.get_by_role('checkbox', name=title, exact=True).click()
        page.get_by_role('button', name='保存独立经验', exact=True).click()
        expect(page.get_by_text('独立经验已保存，待验证。', exact=True)).to_be_visible()
    recorded = state()
    assert len(recorded['sessions']) == len(initial['sessions']), 'independent knowledge must not fabricate a training session'
    assert all(item['source_session_id'] is None for item in recorded['experiences'])
    assert next(item for item in recorded['experiences'] if item['content'] == detail)['goal_ids'] == []

    page.get_by_role('button', name='目标', exact=True).click()
    page.get_by_text(purpose['title'], exact=True).click()
    expect(page.get_by_text('实现这个目标的方法', exact=True)).to_be_visible()
    for content in [principle, detail, both]:
        expect(page.get_by_text(content, exact=True)).to_be_visible()

    # A stale pathway form keeps the user's text until they explicitly resolve it.
    page.get_by_role('button', name='方法', exact=True).click()
    page.get_by_role('button', name='查看方法：' + title, exact=True).click()
    page.get_by_role('textbox', name='作用说明', exact=True).fill('【验收】我的未提交说明')
    current = paths[0]
    put('/pathways/' + current['id'], {**current, 'expected_version': current['version'], 'description': '【验收】另一位记录者的说明'})
    page.get_by_role('button', name='保存方法', exact=True).click()
    expect(page.get_by_text('方法已被更新，请保留填写内容并核对最新版本', exact=True)).to_be_visible()
    expect(page.get_by_role('textbox', name='作用说明', exact=True)).to_have_value('【验收】我的未提交说明')
    page.get_by_role('button', name='保留填写，按最新版本继续', exact=True).click()
    page.get_by_role('button', name='保存方法', exact=True).click()
    expect(page.get_by_text('方法已保存。', exact=True)).to_be_visible()
    assert next(path for path in state()['pathways'] if path['id'] == current['id'])['version'] == 3

    page.get_by_role('button', name='查看方法：' + second_title, exact=True).click()
    page.get_by_role('radio', name='已归档', exact=True).click()
    page.get_by_role('button', name='保存方法', exact=True).click()
    expect(page.get_by_text('方法已保存。', exact=True)).to_be_visible()
    assert set(next(path for path in state()['pathways'] if path['id'] == paths[1]['id'])['goal_ids']) == {purpose['id'], other['id']}

    page.get_by_role('button', name='跟进', exact=True).click()
    item = next(item for item in state()['experiences'] if item['content'] == both)
    card = page.get_by_test_id('followup-' + item['id'])
    card.get_by_role('button', name='更新跟进', exact=True).click()
    card.get_by_role('textbox', name='经验内容', exact=True).fill('【验收】更新后的微观细节')
    card.get_by_role('checkbox', name=re.compile(re.escape(purpose['title']) + ' · 长期')).click()
    card.get_by_role('checkbox', name=second_title + ' · 已归档', exact=True).click()
    card.get_by_role('button', name='已验证', exact=True).click()
    card.get_by_role('textbox', name='验证依据', exact=True).fill('【验收】在两种活动中分别核对')
    card.get_by_role('button', name='保存跟进', exact=True).click()
    expect(page.get_by_text('验证记录已保存。', exact=True)).to_be_visible()
    updated = next(value for value in state()['experiences'] if value['id'] == item['id'])
    assert updated['goal_ids'] == []
    assert set(updated['pathway_ids']) == {path['id'] for path in paths}
    assert updated['source_session_id'] is None and updated['status'] == 'verified'

    # Refresh never adopts a newer base version while an experience form is dirty.
    card.get_by_role('textbox', name='经验内容', exact=True).fill('【验收】仍应保留的未提交原则')
    put('/experiences/' + item['id'], {'expected_version': updated['version'], 'status': 'verified', 'conclusion': updated['conclusion'], 'next_review_date': None, 'content': '【验收】另一位记录者的原则'})
    card.get_by_role('button', name='保存跟进', exact=True).click()
    expect(page.get_by_text('跟进记录已更新，请重新核对', exact=True)).to_be_visible()
    expect(card.get_by_role('textbox', name='经验内容', exact=True)).to_have_value('【验收】仍应保留的未提交原则')
    page.get_by_role('button', name='刷新', exact=True).click()
    expect(card.get_by_text('记录已被更新，当前填写仍保留。请核对最新记录后再保存。', exact=True)).to_be_visible()
    expect(card.get_by_role('textbox', name='经验内容', exact=True)).to_have_value('【验收】仍应保留的未提交原则')
    card.get_by_role('button', name='载入最新跟进内容', exact=True).click()
    expect(card.get_by_role('textbox', name='经验内容', exact=True)).to_have_value('【验收】另一位记录者的原则')

    page.get_by_role('button', name='我的账号', exact=True).click()
    with page.expect_download() as download:
        page.get_by_role('button', name='导出 Excel', exact=True).click()
    workbook = read_workbook(Path(download.value.path()).read_bytes())
    assert len(workbook['途径']) == 2
    assert len(workbook['目的途径关系']) == 4
    assert len(workbook['经验关联']) == 4
    page.get_by_role('button', name='方法', exact=True).click()
    page.get_by_role('button', name='刷新', exact=True).scroll_into_view_if_needed()
    assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
    page.screenshot(path=str(OUTPUT / 'pathways.png'), full_page=True)
    assert not errors, errors
    browser.close()
print('PASS mobile knowledge: many-to-many pathways, principles/details, independent sources, conflict preservation, archive retention and Excel relations')
