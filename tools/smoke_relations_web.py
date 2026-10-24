"""Create related entities through the real UI on an empty synthetic space."""
from browser_navigation import start_plan
import json
import os
from pathlib import Path
from playwright.sync_api import expect, sync_playwright

BASE = os.environ['SKARMA_TEST_URL']
OUT = Path(os.environ['SKARMA_RELATION_OUTPUT'])
OUT.mkdir(parents=True, exist_ok=True)
GOAL = '【关联自测】主动表达需要'
METHOD = '【关联自测】视觉选项提示'
PRINCIPLE = '【关联自测】先等待，再提供提示'
OBSERVATION = '【关联自测】两张选项有助于主动表达'
PLAN = '【关联自测】家人一起练习选择物品'

with sync_playwright() as p:
    browser = p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE', '/usr/bin/chromium'), headless=True, args=['--no-sandbox'])
    context = browser.new_context(viewport={'width': 390, 'height': 844})
    page = context.new_page()
    errors = []
    page.on('pageerror', lambda error: errors.append(str(error)))

    def login(page):
        page.goto(BASE)
        page.get_by_role('textbox', name='账号', exact=True).fill('owner')
        page.get_by_label('密码', exact=True).fill('synthetic-owner-contract-password')
        page.get_by_role('button', name='登录', exact=True).click()
        expect(page.get_by_role('button', name='记录上课', exact=True)).to_be_visible()

    def state():
        response = context.request.get(BASE + '/api/state')
        assert response.ok
        return response.json()

    login(page)
    initial = state()
    assert all(not initial[key] for key in ['goals', 'pathways', 'experiences', 'sessions']), 'use an empty temporary space'

    page.get_by_role('button', name='目标', exact=True).click()
    page.get_by_role('button', name='＋ 新建目标', exact=True).click()
    page.get_by_role('textbox', name='目标名称', exact=True).fill(GOAL)
    page.get_by_role('textbox', name='达成标准', exact=True).fill('在日常场景中主动表达需要，并保留具体观察')
    page.get_by_role('button', name='保存目标', exact=True).click()
    expect(page.get_by_text('新目标已保存。', exact=True)).to_be_visible()
    goal = state()['goals'][0]
    assert goal['title'] == GOAL and goal['level'] == 'short'

    page.get_by_role('button', name='方法', exact=True).click()
    page.get_by_role('button', name='＋ 新建方法', exact=True).click()
    page.get_by_role('checkbox', name=GOAL + ' · 短期', exact=True).click()
    expect(page.get_by_role('checkbox', name=GOAL + ' · 短期', exact=True)).to_contain_text('✓')
    page.get_by_role('textbox', name='方法名称', exact=True).fill(METHOD)
    page.get_by_role('textbox', name='作用说明', exact=True).fill('帮助理解和表达当前需要')
    page.get_by_role('textbox', name='具体步骤', exact=True).fill('展示两张选项，留出等待时间，再按需要提供提示')
    page.get_by_role('button', name='保存方法', exact=True).click()
    expect(page.get_by_text('方法已保存。', exact=True)).to_be_visible()
    method = state()['pathways'][0]
    assert method['goal_ids'] == [goal['id']]

    page.get_by_role('button', name='跟进', exact=True).click()
    page.get_by_role('button', name='＋ 新建经验', exact=True).click()
    page.get_by_role('textbox', name='经验内容', exact=True).fill(PRINCIPLE)
    page.get_by_role('checkbox', name=GOAL + ' · 短期', exact=True).click()
    expect(page.get_by_role('checkbox', name=GOAL + ' · 短期', exact=True)).to_contain_text('✓')
    page.get_by_role('checkbox', name=METHOD, exact=True).click()
    expect(page.get_by_role('checkbox', name=METHOD, exact=True)).to_contain_text('✓')
    page.get_by_role('button', name='保存独立经验', exact=True).click()
    expect(page.get_by_text('独立经验已保存，待验证。', exact=True)).to_be_visible()
    principle = state()['experiences'][0]
    assert principle['goal_ids'] == [goal['id']] and principle['pathway_ids'] == [method['id']]
    assert principle['source_session_id'] is None
    print('PASS UI created one goal, one method and one independent experience with exact bindings', flush=True)

    page.get_by_role('button', name='今天', exact=True).click()
    start_plan(page)
    page.get_by_role('button', name='游戏', exact=True).click()
    page.get_by_role('textbox', name='整次训练安排', exact=True).fill(PLAN)
    page.get_by_role('button', name='＋ 主目标', exact=True).click()
    page.get_by_role('button', name=GOAL, exact=True).click()
    page.get_by_role('textbox', name='主目标计划', exact=True).fill('提供选择机会，记录表达和提示程度')
    page.get_by_role('button', name='保存计划并退出', exact=True).click()
    expect(page.get_by_role('button', name='填写今天的总结', exact=True)).to_be_visible()
    draft = state()['sessions'][0]
    assert draft['status'] == 'draft' and draft['targets'][0]['goal_id'] == goal['id']
    page.get_by_role('button', name='填写今天的总结', exact=True).click()
    page.get_by_role('textbox', name='主目标实际进度', exact=True).fill('提供选项后主动表达一次，其余情况仍需要等待')
    page.get_by_role('button', name='继续', exact=True).click()
    page.get_by_role('textbox', name='整次训练观察', exact=True).fill('家人干预的合成观察，用于核对关联')
    page.get_by_role('button', name='是', exact=True).nth(1).click()
    page.get_by_role('textbox', name='成功经验详情', exact=True).fill(OBSERVATION)
    expect(page.get_by_role('combobox', name='经验关联目标', exact=True)).to_have_value(goal['id'])
    page.get_by_role('combobox', name='经验关联方法', exact=True).select_option(method['id'])
    page.get_by_role('button', name='提交干预总结', exact=True).click()
    expect(page.get_by_text('这次训练已完成，困难与经验已按填写内容保存。', exact=True)).to_be_visible()
    expect(page.get_by_text('主目标 · ' + GOAL, exact=True)).to_be_visible()
    expect(page.get_by_text(OBSERVATION, exact=True)).to_be_visible()
    page.get_by_text('成功经验', exact=True).scroll_into_view_if_needed()
    page.screenshot(path=str(OUT / 'training.png'))
    after = state()
    assert len(after['goals']) == len(after['pathways']) == len(after['sessions']) == 1
    assert len(after['experiences']) == 2
    session = after['sessions'][0]
    assert session['id'] == draft['id'] and session['status'] == 'completed'
    experience = next(item for item in after['experiences'] if item['source_session_id'] == session['id'])
    assert experience['goal_ids'] == [goal['id']] and experience['pathway_ids'] == [method['id']]

    page.get_by_role('button', name='目标', exact=True).click()
    page.get_by_text(GOAL, exact=True).click()
    expect(page.get_by_text(METHOD, exact=True)).to_be_visible()
    expect(page.get_by_text(PRINCIPLE, exact=True)).to_be_visible()
    expect(page.get_by_text(OBSERVATION, exact=True)).to_be_visible()
    expect(page.get_by_text(PLAN, exact=True)).to_be_visible()
    page.get_by_text('实现这个目标的方法', exact=True).scroll_into_view_if_needed()
    page.screenshot(path=str(OUT / 'goal.png'))

    page.get_by_role('button', name='查看方法：' + METHOD, exact=True).click()
    expect(page.get_by_role('button', name='查看目标：' + GOAL, exact=True)).to_be_visible()
    expect(page.get_by_text(PRINCIPLE, exact=True)).to_be_visible()
    expect(page.get_by_text(OBSERVATION, exact=True)).to_be_visible()
    page.get_by_text('关联经验', exact=True).scroll_into_view_if_needed()
    page.screenshot(path=str(OUT / 'method.png'))

    page.get_by_role('button', name='跟进', exact=True).click()
    for item in [principle, experience]:
        card = page.get_by_test_id('followup-' + item['id'])
        expect(card.get_by_text('关联目标：' + GOAL, exact=True)).to_be_visible()
        expect(card.get_by_text('关联方法：' + METHOD, exact=True)).to_be_visible()
    source_card = page.get_by_test_id('followup-' + experience['id'])
    expect(source_card.get_by_role('button', name='查看来源训练', exact=True)).to_be_visible()
    source_card.scroll_into_view_if_needed()
    page.screenshot(path=str(OUT / 'experience.png'))
    source_card.get_by_role('button', name='查看来源训练', exact=True).click()
    expect(page.get_by_text(PLAN, exact=True)).to_be_visible()

    page.get_by_role('button', name='记录', exact=True).click()
    page.get_by_role('button', name='家庭干预', exact=True).click()
    expect(page.get_by_text(PLAN, exact=True)).to_be_visible()
    page.reload()
    expect(page.get_by_role('button', name='记录上课', exact=True)).to_be_visible()
    page.get_by_role('button', name='目标', exact=True).click()
    page.get_by_text(GOAL, exact=True).click()
    for text in [METHOD, PRINCIPLE, OBSERVATION, PLAN]:
        expect(page.get_by_text(text, exact=True)).to_be_visible()

    other = browser.new_context(viewport={'width': 390, 'height': 844})
    other_page = other.new_page()
    other_page.on('pageerror', lambda error: errors.append(str(error)))
    login(other_page)
    other_page.get_by_role('button', name='目标', exact=True).click()
    other_page.get_by_text(GOAL, exact=True).click()
    for text in [METHOD, PRINCIPLE, OBSERVATION, PLAN]:
        expect(other_page.get_by_text(text, exact=True)).to_be_visible()
    assert not errors, errors
    assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
    result = {'synthetic': True, 'created': {'goals': 1, 'methods': 1, 'sessions': 1, 'independent_experiences': 1, 'training_experiences': 1},
              'ids': {'goal': goal['id'], 'method': method['id'], 'session': session['id'], 'principle': principle['id'], 'experience': experience['id']},
              'checks': ['UI creation', 'exact persisted IDs', 'goal shows method and both experiences and training', 'method shows goal and both experiences',
                         'experience shows goal/method and training source', 'family record list', 'page reload', 'second browser'],
              'snapshot': after}
    if os.environ.get('SKARMA_TEST_MUTATIONS') == '1':
        from smoke_mutations_web import check_mutations
        result['mutations'] = check_mutations(page, context, browser, BASE, OUT, result, login, state)
        result['snapshot'] = state()
        assert not errors, errors
    (OUT / 'result.json').write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')
    browser.close()
print('PASS UI relationships, source jump, reload and independent browser; all writes through forms')
