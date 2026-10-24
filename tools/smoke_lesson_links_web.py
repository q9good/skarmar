"""Phone-size lesson relations/catalog regression against disposable synthetic spaces."""
import json
import os
from pathlib import Path
from playwright.sync_api import expect, sync_playwright

BASE = os.environ['SKARMA_TEST_URL']
OUT = Path(os.environ['SKARMA_RELATION_OUTPUT'])
OUT.mkdir(parents=True, exist_ok=True)
FIRST, SECOND = '【课程自测】轮流参与', '【课程自测】主动表达'
METHOD, COURSE, TEACHER = '【课程自测】视觉提示', '【课程自测】音乐互动', '【课程自测】甲老师'
with sync_playwright() as p:
    browser = p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE', '/usr/bin/chromium'), headless=True, args=['--no-sandbox'])
    context = browser.new_context(viewport={'width': 390, 'height': 844})
    page = context.new_page()
    errors = []
    page.on('pageerror', lambda e: errors.append(str(e)))
    page.goto(BASE)
    page.get_by_role('textbox', name='账号', exact=True).fill('owner')
    page.get_by_label('密码', exact=True).fill('synthetic-owner-contract-password')
    page.get_by_role('button', name='登录', exact=True).click()
    expect(page.get_by_role('button', name='记录上课', exact=True)).to_be_visible()

    def state():
        response = context.request.get(BASE + '/api/state')
        assert response.ok
        return response.json()

    def focus():
        page.evaluate("window.dispatchEvent(new Event('focus'))")

    assert not state()['lessons']
    for name in [FIRST, SECOND]:
        page.get_by_role('button', name='目标', exact=True).click()
        page.get_by_role('button', name='＋ 新建目标', exact=True).click()
        page.get_by_role('textbox', name='目标名称', exact=True).fill(name)
        page.get_by_role('textbox', name='达成标准', exact=True).fill('保留当时具体观察')
        page.get_by_role('button', name='保存目标', exact=True).click()
        expect(page.get_by_text('新目标已保存。', exact=True)).to_be_visible()
    goals = {g['title']: g for g in state()['goals']}
    page.get_by_role('button', name='方法', exact=True).click()
    page.get_by_role('button', name='＋ 新建方法', exact=True).click()
    page.get_by_role('textbox', name='方法名称', exact=True).fill(METHOD)
    page.get_by_role('textbox', name='具体步骤', exact=True).fill('展示选项，再等待回应')
    page.get_by_role('checkbox', name=SECOND + ' · 短期', exact=True).click()
    page.get_by_role('button', name='保存方法', exact=True).click()
    expect(page.get_by_role('button', name='查看方法：' + METHOD, exact=True)).to_be_visible()
    method = state()['pathways'][0]
    page.get_by_role('button', name='今天', exact=True).click()
    page.get_by_role('button', name='记录上课', exact=True).click()
    for label, field, value in [('课程名称（必填）', '新增课程名称', COURSE), ('授课老师（必填）', '新增老师名称', TEACHER)]:
        page.get_by_role('combobox', name=label, exact=True).select_option('__new__')
        page.get_by_role('textbox', name=field, exact=True).fill(value)
        page.get_by_role('button', name='保存并使用新选项', exact=True).click()
    page.get_by_role('textbox', name='上课内容', exact=True).fill('【课程自测】一起唱歌并轮流选择')
    for name in [FIRST, SECOND]:
        page.get_by_role('combobox', name='添加课程目标', exact=True).select_option(goals[name]['id'])
    page.get_by_role('combobox', name='课程目标角色：' + SECOND, exact=True).select_option('primary')
    page.get_by_role('textbox', name='课程主目标内容', exact=True).fill('提供选择机会')
    page.get_by_role('textbox', name='课程主目标观察', exact=True).fill('【课程自测】主动表达两次')
    page.get_by_role('checkbox', name='课程实际方法：' + METHOD, exact=True).click()
    page.get_by_role('textbox', name='存在问题', exact=True).fill('【课程自测】需要等待五秒')
    page.get_by_role('button', name='需要跟进', exact=True).click()
    expect(page.get_by_role('checkbox', name=SECOND + ' · 短期', exact=True)).to_be_checked()
    expect(page.get_by_text('上课草稿已保存在本机', exact=True)).to_be_visible()
    page.reload()
    page.get_by_role('button', name='继续填写', exact=True).click()
    expect(page.get_by_role('textbox', name='课程主目标观察', exact=True)).to_have_value('【课程自测】主动表达两次')
    expect(page.get_by_role('checkbox', name='课程实际方法：' + METHOD, exact=True)).to_be_checked()
    page.get_by_role('button', name='保存上课记录', exact=True).click()
    expect(page.get_by_text('上课记录已保存。', exact=True)).to_be_in_viewport()
    saved = state()['lessons'][0]
    assert next(t for t in saved['links']['targets'] if t['role'] == 'primary')['goal_id'] == goals[SECOND]['id']
    assert saved['links']['pathway_ids'] == [method['id']]
    difficulty = next(d for d in state()['difficulties'] if d.get('source_lesson_id') == saved['id'])
    assert difficulty['goal_ids'] == [goals[SECOND]['id']]
    assert difficulty['pathway_ids'] == [method['id']]
    assert not state()['sessions']
    page.get_by_role('button', name='目标', exact=True).click()
    page.get_by_text(SECOND, exact=True).click()
    expect(page.get_by_text('【课程自测】需要等待五秒', exact=True)).to_be_visible()
    expect(page.get_by_role('button', name=f"查看上课记录：{COURSE} · {saved['business_date']}", exact=True)).to_be_visible()
    page.get_by_role('button', name='查看方法：' + METHOD, exact=True).click()
    expect(page.get_by_role('button', name=f"查看上课记录：{COURSE} · {saved['business_date']}", exact=True)).to_be_visible()
    page.get_by_role('button', name='跟进', exact=True).click()
    card = page.get_by_test_id('followup-' + difficulty['id'])
    card.get_by_role('button', name='查看来源上课', exact=True).click()
    page.get_by_role('button', name='查看课程均衡规划提示', exact=True).click()
    summary = page.get_by_test_id('planning-summary')
    expect(summary.get_by_text('已覆盖 2 / 2 个目标 · 完成 1 节机构课程', exact=True)).to_be_visible()
    page.get_by_role('button', name='家庭干预覆盖', exact=True).click()
    expect(summary.get_by_text('已覆盖 0 / 2 个目标 · 完成 0 场训练', exact=True)).to_be_visible()
    page.get_by_role('button', name='合计覆盖', exact=True).click()
    expect(summary.get_by_text('已覆盖 2 / 2 个目标 · 完成 1 次活动', exact=True)).to_be_visible()
    page.get_by_role('button', name='方法覆盖', exact=True).click()
    expect(page.get_by_test_id('planning-pathway-' + method['id']).get_by_text('所选时间机构课程实际使用 1 节 · 家庭实际使用次数待接入', exact=True)).to_be_visible()
    page.get_by_text('有哪些方式可以尝试？', exact=True).scroll_into_view_if_needed()
    page.screenshot(path=str(OUT / 'lesson-planning.png'))
    page.get_by_role('button', name='收起课程规划提示', exact=True).click()
    page.get_by_text('2 · 本次课程目标', exact=True).scroll_into_view_if_needed()
    page.screenshot(path=str(OUT / 'lesson-targets.png'), full_page=True)

    # A second writer updates the linked difficulty while this lesson draft stays intact.
    page.get_by_role('textbox', name='上课内容', exact=True).fill('【课程自测】尚未保存的本机内容')
    response = context.request.put(BASE + '/api/difficulties/' + difficulty['id'], data={**difficulty, 'expected_version': difficulty['version'], 'content': '【课程自测】独立维护的困难', 'status': 'active', 'conclusion': '另行观察'}, headers={'X-Skarma-Request': '1'})
    assert response.ok, response.text()
    focus()
    expect(page.get_by_role('button', name='保存上课记录', exact=True)).to_be_disabled()
    expect(page.get_by_role('textbox', name='上课内容', exact=True)).to_have_value('【课程自测】尚未保存的本机内容')
    page.get_by_role('button', name='备份我的填写并载入最新上课记录', exact=True).click()
    page.get_by_role('textbox', name='存在问题', exact=True).fill('【课程自测】修正课程问题原文')
    page.get_by_role('button', name='保存上课记录', exact=True).click()
    expect(page.get_by_text('上课记录已保存。', exact=True)).to_be_visible()
    assert next(d for d in state()['difficulties'] if d['id'] == difficulty['id'])['content'] == '【课程自测】独立维护的困难'

    page.get_by_role('button', name='记录', exact=True).click()
    page.get_by_role('button', name='维护课程和老师选项', exact=True).click()
    page.get_by_role('button', name='改名老师：' + TEACHER, exact=True).click()
    page.get_by_role('textbox', name='选项新名称', exact=True).fill(TEACHER + '新名')
    page.get_by_role('button', name='保存新名称', exact=True).click()
    expect(page.get_by_text('已更名为：' + TEACHER + '新名', exact=True)).to_be_visible()
    page.get_by_role('button', name='停用老师：' + TEACHER + '新名', exact=True).click()
    page.get_by_role('button', name='恢复老师：' + TEACHER + '新名', exact=True).click()
    assert state()['lessons'][0]['teacher'] == TEACHER
    page.get_by_role('button', name='＋ 新建上课记录', exact=True).click()
    names = page.get_by_role('combobox', name='授课老师（必填）', exact=True).locator('option').evaluate_all('(items) => items.map(x => x.value)')
    assert TEACHER not in names and TEACHER + '新名' in names
    # Empty unsaved form is not a persisted record.
    page.get_by_role('button', name='返回', exact=True).click()
    page.reload()
    page.get_by_role('button', name='记录', exact=True).click()
    page.get_by_text(COURSE, exact=True).click()
    expect(page.get_by_role('combobox', name='授课老师（必填）', exact=True)).to_have_value(TEACHER)
    expect(page.get_by_role('checkbox', name='课程实际方法：' + METHOD, exact=True)).to_be_checked()
    assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
    assert not errors, errors
    (OUT / 'result.json').write_text(json.dumps({'synthetic': True, 'snapshot': state(), 'checks': ['goal roles and details', 'actual methods', 'problem source', 'goal/method drilldown', 'draft reload', 'separate and combined planning', 'followup concurrent conflict preserves draft', 'curated followup preserved', 'catalog add rename disable restore', 'historical teacher preserved', 'refresh']}, ensure_ascii=False, indent=2))
    browser.close()
print('PASS lesson UI: dropdown catalogs, structured goal/method/problem links, planning sources, concurrent followup protection and refresh')
