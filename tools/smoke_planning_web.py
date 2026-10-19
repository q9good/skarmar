"""Planning balance dashboard against the disposable authenticated host."""
import os
from datetime import datetime, timedelta
from pathlib import Path
from uuid import uuid4
from zoneinfo import ZoneInfo

from playwright.sync_api import expect, sync_playwright

BASE = os.environ['SKARMA_TEST_URL']
OUTPUT = Path('.local/screenshots')
OUTPUT.mkdir(parents=True, exist_ok=True)
today = datetime.now(ZoneInfo('Asia/Shanghai')).date()

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
    page.get_by_role('button', name='＋ 新建干预计划', exact=True).click()
    page.get_by_role('button', name='查看均衡规划提示', exact=True).click()
    expect(page.get_by_role('img', name='训练分布：暂无数据', exact=True)).to_be_visible()

    def post(path, body):
        response = context.request.post(BASE + '/api' + path, data=body, headers={'X-Skarma-Request': '1'})
        assert response.ok, response.text()
        return response.json()

    def put(path, body):
        response = context.request.put(BASE + '/api' + path, data=body, headers={'X-Skarma-Request': '1'})
        assert response.ok, response.text()
        return response.json()

    def create_goal(title, level, parent=None):
        return post('/goals', {'id': str(uuid4()), 'title': '【规划验收】' + title, 'level': level,
                              'parent_id': parent['id'] if parent else None, 'area': '生活与沟通', 'criteria': '依据具体观察与帮助情况复盘'})

    long_goal = create_goal('生活与沟通', 'long')
    medium = create_goal('表达需要', 'medium', long_goal)
    another_medium = create_goal('参与生活', 'medium', long_goal)
    frequent = create_goal('请求熟悉物品', 'short', medium)
    secondary = create_goal('共同注意', 'short', medium)
    neglected = create_goal('尝试新生活任务', 'short', another_medium)
    achieved = create_goal('巩固已达成任务', 'short', another_medium)
    achieved = put('/goals/' + achieved['id'], {**achieved, 'expected_version': achieved['version'], 'progress_status': 'achieved', 'change_note': '验收：已确认达成'})

    def record(days_ago, targets, action='complete'):
        return put('/sessions/' + str(uuid4()), {'operation_id': str(uuid4()), 'expected_version': None, 'action': action, 'session': {
            'business_date': str(today - timedelta(days=days_ago)), 'time_slot': '上午', 'activities': ['游戏'], 'organization': '居家',
            'plan': '【规划验收】安排不同方向', 'observation': '【规划验收】实际观察',
            'targets': [{'goal_id': goal['id'], 'role': 'primary' if i == 0 else 'secondary', 'plan': '观察并提供适当支持',
                         'progress': '【规划验收】已有逐目标观察', 'outcome': 'continue', 'next_step': ''} for i, goal in enumerate(targets)],
            'has_difficulty': False, 'difficulty': '', 'has_experience': False, 'experience': '', 'experience_kind': 'task',
        }})

    record(0, [frequent, medium, secondary])
    record(1, [frequent, secondary])
    record(40, [neglected, secondary])
    record(-2, [neglected, secondary], 'save_draft')
    pathway = post('/pathways', {'id': str(uuid4()), 'title': '【规划验收】视觉选择', 'description': '提供不同场景中的选择机会',
                                 'steps': '提供选择并等待回应', 'goal_ids': [frequent['id'], neglected['id']]})
    post('/experiences', {'id': str(uuid4()), 'content': '【规划验收】一次展示少量选择', 'goal_ids': [], 'pathway_ids': [pathway['id']]})
    page.get_by_role('button', name='刷新', exact=True).click()
    rare_card = page.get_by_test_id('planning-goal-' + neglected['id'])
    frequent_card = page.get_by_test_id('planning-goal-' + frequent['id'])
    expect(rare_card.get_by_text('所选时间 0 次 · 最近训练 ' + str(today - timedelta(days=40)), exact=True)).to_be_visible()
    expect(frequent_card.get_by_text('所选时间 2 次 · 最近训练 ' + str(today), exact=True)).to_be_visible()
    assert page.locator('svg circle').count() > 2, 'charts must render real SVG segments'

    page.get_by_role('button', name='查看近期未覆盖目标', exact=True).click()
    expect(rare_card).to_be_visible()
    expect(page.get_by_test_id('planning-goal-' + frequent['id'])).to_have_count(0)
    expect(page.get_by_test_id('planning-goal-' + achieved['id'])).to_have_count(0)
    page.get_by_role('button', name='显示全部目标', exact=True).click()
    page.get_by_role('button', name='近 90 天', exact=True).click()
    expect(rare_card.get_by_text('所选时间 1 次 · 最近训练 ' + str(today - timedelta(days=40)), exact=True)).to_be_visible()
    page.get_by_role('button', name='近 30 天', exact=True).click()
    page.get_by_role('button', name='查看分布：' + frequent['title'], exact=True).click()
    expect(frequent_card).to_be_visible()
    expect(rare_card).to_have_count(0)
    page.get_by_role('button', name='显示全部目标', exact=True).click()

    page.get_by_role('button', name='中期目标', exact=True).click()
    parent_card = page.get_by_test_id('planning-goal-' + medium['id'])
    expect(parent_card.get_by_text('所选时间 2 次 · 最近训练 ' + str(today), exact=True)).to_be_visible()
    expect(parent_card.get_by_text('直接训练 1 次 · 仅通过下属短期覆盖 1 次', exact=True)).to_be_visible()
    expect(page.get_by_test_id('planning-goal-' + another_medium['id']).get_by_text('下属短期已达成／完成 1 / 2 个', exact=True)).to_be_visible()
    page.get_by_role('button', name='短期目标', exact=True).click()
    assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
    page.get_by_role('button', name='刷新', exact=True).scroll_into_view_if_needed()
    page.screenshot(path=str(OUTPUT / 'planning.png'), full_page=True)
    page.get_by_text('训练分布', exact=True).scroll_into_view_if_needed()
    page.screenshot(path=str(OUTPUT / 'planning-charts.png'), full_page=True)

    page.get_by_role('button', name='方法覆盖', exact=True).click()
    path_card = page.get_by_test_id('planning-pathway-' + pathway['id'])
    expect(path_card.get_by_text('使用中 · 当前范围适用 2 个目标 · 1 条经验', exact=True)).to_be_visible()
    expect(page.get_by_text('目前尚未记录每次实际使用的方法，因此不展示方法训练次数。', exact=True)).to_be_visible()
    path_card.get_by_role('button', name='查看方法与细节', exact=True).click()
    expect(page.get_by_role('textbox', name='方法名称', exact=True)).to_have_value(pathway['title'])

    # Adding a neglected direction preserves a dirty draft and cannot duplicate a target.
    page.get_by_role('button', name='今天', exact=True).click()
    page.get_by_role('button', name='＋ 新建干预计划', exact=True).click()
    page.get_by_role('textbox', name='整次训练安排', exact=True).fill('【规划验收】已有计划内容需要保留')
    page.get_by_role('button', name='查看均衡规划提示', exact=True).click()
    page.get_by_test_id('planning-goal-' + neglected['id']).get_by_role('button', name='加入当前计划', exact=True).click()
    expect(page.get_by_role('textbox', name='整次训练安排', exact=True)).to_have_value('【规划验收】已有计划内容需要保留')
    expect(page.get_by_text('主目标 · ' + neglected['title'], exact=True).first).to_be_visible()
    page.get_by_role('button', name='查看均衡规划提示', exact=True).click()
    page.get_by_test_id('planning-goal-' + frequent['id']).get_by_role('button', name='加入当前计划', exact=True).click()
    expect(page.get_by_text('副目标 · ' + frequent['title'], exact=True).first).to_be_visible()
    page.get_by_role('button', name='查看均衡规划提示', exact=True).click()
    page.get_by_test_id('planning-goal-' + frequent['id']).get_by_role('button', name='加入当前计划', exact=True).click()
    expect(page.get_by_role('button', name='移除' + frequent['title'], exact=True)).to_have_count(1)
    page.get_by_role('button', name='查看均衡规划提示', exact=True).click()
    page.get_by_test_id('planning-goal-' + achieved['id']).get_by_role('button', name='加入当前计划', exact=True).click()
    expect(page.get_by_text('复习 · ' + achieved['title'], exact=True).first).to_be_visible()
    expect(page.get_by_role('textbox', name='整次训练安排', exact=True)).to_have_value('【规划验收】已有计划内容需要保留')
    page.get_by_role('button', name='保存计划并退出', exact=True).click()
    expect(page.get_by_text('草稿已保存到服务器。干预后从今天的安排填写总结。', exact=True)).to_be_visible()
    page.reload()
    page.get_by_role('button', name='填写干预总结', exact=True).click()
    page.get_by_role('button', name='修改本次计划', exact=True).click()
    expect(page.get_by_role('textbox', name='整次训练安排', exact=True)).to_have_value('【规划验收】已有计划内容需要保留')
    page.get_by_role('button', name='查看均衡规划提示', exact=True).click()
    page.get_by_test_id('planning-goal-' + neglected['id']).get_by_role('button', name='查看目标与经验', exact=True).click()
    expect(page.get_by_text(neglected['title'], exact=True)).to_be_visible()

    # Empty data and an aggregated >6-goal pie remain usable on narrow screens.
    real_state = context.request.get(BASE + '/api/state').json()
    chart_state = {**real_state, 'goals': [], 'sessions': []}
    for i in range(8):
        goal = {**frequent, 'id': str(uuid4()), 'title': '【分布验收】方向' + str(i)}
        chart_state['goals'].append(goal)
        chart_state['sessions'].append({**real_state['sessions'][0], 'id': str(uuid4()), 'status': 'completed', 'business_date': str(today),
                                      'targets': [{'goal_id': goal['id'], 'progress': '', 'role': 'primary'}], 'target_snapshots': [{'goal_id': goal['id'], 'goal_snapshot': goal}]})
    page.route('**/api/state', lambda route: route.fulfill(json=chart_state))
    page.reload()
    page.get_by_role('button', name='＋ 新建干预计划', exact=True).click()
    page.get_by_role('button', name='查看均衡规划提示', exact=True).click()
    page.get_by_role('button', name='查看分布：其余 2 个目标', exact=True).click()
    expect(page.locator('[data-testid^="planning-goal-"]')).to_have_count(2)
    page.get_by_role('button', name='显示全部目标', exact=True).click()
    for width in [320, 390, 960]:
        page.set_viewport_size({'width': width, 'height': 844})
        assert not page.evaluate('document.documentElement.scrollWidth > innerWidth'), f'overflow at {width}px'
    page.unroute_all(behavior='wait')
    page.route('**/api/state', lambda route: route.fulfill(json={'goals': [], 'sessions': [], 'pathways': [], 'experiences': [], 'difficulties': []}))
    page.reload()
    page.get_by_role('button', name='＋ 新建干预计划', exact=True).click()
    page.get_by_role('button', name='查看均衡规划提示', exact=True).click()
    expect(page.get_by_text('暂无这个范围的目标，可到“目标”页创建，或切换层级与归档范围。', exact=True)).to_be_visible()
    assert not errors, errors
    browser.close()
print('PASS mobile planning: real charts, zero coverage, time filters, medium rollup, pathway scope, pie selection, safe draft planning, role assignment and narrow/desktop layouts')
