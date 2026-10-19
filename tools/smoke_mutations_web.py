"""Extend the synthetic UI creation test with editing and capability probes.

DELETE and completed-session PUT are negative probes for currently missing
features. Their rejection is not evidence of working deletion or revision.
"""
import uuid
from playwright.sync_api import expect


def check_mutations(page, context, browser, base, out, created, login, state):
    ids = created['ids']
    original = created['snapshot']
    old_goal = original['goals'][0]
    old_method = original['pathways'][0]
    old_session = original['sessions'][0]
    goal_name = old_goal['title'] + '（已修订）'
    method_name = old_method['title'] + '（已修订）'
    second_name = '【编辑自测】轮流参与活动'
    principle_text = '【编辑自测】先等待五秒，再提供提示'
    observation_text = '【编辑自测】减少选项后更容易主动表达'
    checks = []

    def entry(key, item_id):
        return next(item for item in state()[key] if item['id'] == item_id)

    def goal(title):
        page.get_by_role('button', name='目标', exact=True).click()
        page.get_by_text(title, exact=True).click()

    def method():
        page.get_by_role('button', name='方法', exact=True).click()
        page.get_by_role('button', name='查看方法：' + method_name, exact=True).click()

    def binding(scope, name, selected):
        control = scope.get_by_role('checkbox', name=name, exact=True)
        control.click()
        if selected:
            expect(control).to_contain_text('✓')
        else:
            expect(control).not_to_contain_text('✓')

    def edit_experience(item_id, content, bind_method):
        page.get_by_role('button', name='跟进', exact=True).click()
        card = page.get_by_test_id('followup-' + item_id)
        card.get_by_role('button', name='更新跟进', exact=True).click()
        card.get_by_role('textbox', name='经验内容', exact=True).fill(content)
        binding(card, goal_name + ' · 短期', False)
        binding(card, second_name + ' · 短期', True)
        if not bind_method:
            binding(card, method_name, False)
        card.get_by_role('button', name='已验证', exact=True).click()
        card.get_by_role('textbox', name='验证依据', exact=True).fill('【编辑自测】连续三次观察到相同表现')
        with page.expect_response(lambda r: r.request.method == 'PUT' and r.url.endswith('/experiences/' + item_id)) as saved:
            card.get_by_role('button', name='保存跟进', exact=True).click()
        assert saved.value.ok
        card.get_by_role('button', name='收起', exact=True).click()
        expect(card.get_by_text(content, exact=True)).to_be_visible()
        return entry('experiences', item_id)

    # Current labels change; the historical training definition must not change.
    goal(old_goal['title'])
    page.get_by_role('button', name='修改／复盘目标', exact=True).click()
    page.get_by_role('textbox', name='目标名称', exact=True).fill(goal_name)
    page.get_by_role('textbox', name='达成标准', exact=True).fill('【编辑自测】在两个不同场景主动表达需要')
    page.get_by_role('textbox', name='进度与复盘记录', exact=True).fill('【编辑自测】保留旧版标准与训练快照')
    page.get_by_role('button', name='保存目标复盘', exact=True).click()
    expect(page.get_by_text('目标复盘已保存，修改历史已保留。', exact=True)).to_be_visible()
    updated_goal = entry('goals', ids['goal'])
    assert updated_goal['version'] == old_goal['version'] + 1
    assert updated_goal['title'] == goal_name
    assert entry('sessions', ids['session']) == old_session
    history = context.request.get(base + '/api/goals/' + ids['goal'] + '/history').json()
    assert any(item['goal'] == old_goal for item in history)
    assert any(item['goal'] == updated_goal for item in history)
    page.get_by_text('目标修改历史 · ' + str(len(history)), exact=True).scroll_into_view_if_needed()
    page.screenshot(path=str(out / 'edited-goal-history.png'))
    checks.append('goal name/criteria/review edited; history and completed training snapshot preserved')

    page.get_by_role('button', name='目标', exact=True).click()
    page.get_by_role('button', name='＋ 新建目标', exact=True).click()
    page.get_by_role('textbox', name='目标名称', exact=True).fill(second_name)
    page.get_by_role('textbox', name='达成标准', exact=True).fill('与家人轮流参与两次活动')
    page.get_by_role('button', name='保存目标', exact=True).click()
    expect(page.get_by_text('新目标已保存。', exact=True)).to_be_visible()
    second = next(item for item in state()['goals'] if item['title'] == second_name)

    # Move the method to another goal, while explicit experience bindings stay put.
    page.get_by_role('button', name='方法', exact=True).click()
    page.get_by_role('button', name='查看方法：' + old_method['title'], exact=True).click()
    page.get_by_role('textbox', name='方法名称', exact=True).fill(method_name)
    page.get_by_role('textbox', name='具体步骤', exact=True).fill('【编辑自测】先观察五秒，再提供视觉提示')
    binding(page, goal_name + ' · 短期', False)
    binding(page, second_name + ' · 短期', True)
    page.get_by_role('button', name='保存方法', exact=True).click()
    expect(page.get_by_text('方法已保存。', exact=True)).to_be_visible()
    current_method = entry('pathways', ids['method'])
    assert current_method['title'] == method_name and current_method['goal_ids'] == [second['id']]
    assert current_method['version'] == old_method['version'] + 1
    assert all(item['goal_ids'] == [ids['goal']] for item in state()['experiences'])
    goal(goal_name)
    expect(page.get_by_role('button', name='查看方法：' + method_name, exact=True)).to_have_count(0)
    for item in original['experiences']:
        expect(page.get_by_text(item['content'], exact=True)).to_be_visible()
    goal(second_name)
    expect(page.get_by_text(method_name, exact=True)).to_be_visible()
    for item in original['experiences']:
        expect(page.get_by_text(item['content'], exact=True)).to_be_visible()
    checks.append('method renamed and rebound; old/new goal displays correct; no inferred experience binding')

    independent = edit_experience(ids['principle'], principle_text, False)
    assert independent['goal_ids'] == [second['id']] and independent['pathway_ids'] == []
    assert independent['source_session_id'] is None and independent['status'] == 'verified'
    generated = edit_experience(ids['experience'], observation_text, True)
    assert generated['goal_ids'] == [second['id']] and generated['pathway_ids'] == [ids['method']]
    assert generated['source_session_id'] == ids['session'] and generated['status'] == 'verified'
    assert entry('sessions', ids['session']) == old_session
    goal(goal_name)
    for text in [principle_text, observation_text]:
        expect(page.get_by_text(text, exact=True)).to_have_count(0)
    expect(page.get_by_text(old_session['plan'], exact=True)).to_be_visible()
    goal(second_name)
    for text in [principle_text, observation_text]:
        expect(page.get_by_text(text, exact=True)).to_be_visible()
    method()
    expect(page.get_by_text(principle_text, exact=True)).to_have_count(0)
    expect(page.get_by_text(observation_text, exact=True)).to_be_visible()
    page.get_by_role('button', name='跟进', exact=True).click()
    card = page.get_by_test_id('followup-' + ids['experience'])
    expect(card.get_by_text('关联目标：' + second_name, exact=True)).to_be_visible()
    expect(card.get_by_text('关联方法：' + method_name, exact=True)).to_be_visible()
    card.scroll_into_view_if_needed()
    page.screenshot(path=str(out / 'edited-experience-bindings.png'))
    card.get_by_role('button', name='查看来源训练', exact=True).click()
    expect(page.get_by_text(old_session['plan'], exact=True)).to_be_visible()
    expect(page.get_by_text('主目标 · ' + old_goal['title'], exact=True)).to_be_visible()
    checks.append('both experiences edited/rebound; removed links absent; source and historical summary retained')

    # Remove all method-goal links without deleting the method or its experience.
    method()
    binding(page, second_name + ' · 短期', False)
    page.get_by_role('button', name='保存方法', exact=True).click()
    expect(page.get_by_text('方法已保存。', exact=True)).to_be_visible()
    assert entry('pathways', ids['method'])['goal_ids'] == []
    assert entry('experiences', ids['experience']) == generated
    goal(second_name)
    expect(page.get_by_role('button', name='查看方法：' + method_name, exact=True)).to_have_count(0)
    expect(page.get_by_text(observation_text, exact=True)).to_be_visible()
    method()
    binding(page, second_name + ' · 短期', True)
    page.get_by_role('button', name='保存方法', exact=True).click()
    expect(page.get_by_text('方法已保存。', exact=True)).to_be_visible()
    checks.append('method-goal links removed and restored without deleting method or experience')

    # Edit the same saved draft, replace and remove targets, and save a summary draft.
    page.get_by_role('button', name='今天', exact=True).click()
    page.get_by_role('button', name='＋ 新建干预计划', exact=True).click()
    page.get_by_role('button', name='游戏', exact=True).click()
    page.get_by_role('textbox', name='整次训练安排', exact=True).fill('【编辑自测】待修改计划')
    page.get_by_role('button', name='＋ 主目标', exact=True).click()
    page.get_by_role('button', name=goal_name, exact=True).click()
    page.get_by_role('button', name='保存计划并退出', exact=True).click()
    expect(page.get_by_role('button', name='填写干预总结', exact=True)).to_be_visible()
    draft = next(item for item in state()['sessions'] if item['status'] == 'draft')
    page.get_by_role('button', name='填写干预总结', exact=True).click()
    page.get_by_role('button', name='修改本次计划', exact=True).click()
    page.get_by_role('textbox', name='整次训练安排', exact=True).fill('【编辑自测】已经修改的计划')
    page.get_by_role('combobox', name='更换主目标', exact=True).select_option(second['id'])
    page.get_by_role('button', name='保存计划并退出', exact=True).click()
    expect(page.get_by_role('button', name='填写干预总结', exact=True)).to_be_visible()
    revised = entry('sessions', draft['id'])
    assert revised['targets'][0]['goal_id'] == second['id']
    assert revised['version'] == draft['version'] + 1
    goal(goal_name)
    expect(page.get_by_text(revised['plan'], exact=True)).to_have_count(0)
    goal(second_name)
    expect(page.get_by_text(revised['plan'], exact=True)).to_be_visible()
    page.get_by_role('button', name='今天', exact=True).click()
    page.get_by_role('button', name='填写干预总结', exact=True).click()
    page.get_by_role('button', name='修改本次计划', exact=True).click()
    page.get_by_role('button', name='移除' + second_name, exact=True).click()
    page.get_by_role('button', name='保存计划并退出', exact=True).click()
    expect(page.get_by_role('button', name='填写干预总结', exact=True)).to_be_visible()
    assert entry('sessions', draft['id'])['targets'] == []
    goal(second_name)
    expect(page.get_by_text(revised['plan'], exact=True)).to_have_count(0)
    page.get_by_role('button', name='今天', exact=True).click()
    page.get_by_role('button', name='填写干预总结', exact=True).click()
    page.get_by_role('button', name='修改本次计划', exact=True).click()
    page.get_by_role('button', name='＋ 主目标', exact=True).click()
    page.get_by_role('button', name=second_name, exact=True).click()
    page.get_by_role('button', name='保存计划并退出', exact=True).click()
    expect(page.get_by_role('button', name='填写干预总结', exact=True)).to_be_visible()
    page.get_by_role('button', name='填写干预总结', exact=True).click()
    page.get_by_role('textbox', name='主目标实际进度', exact=True).fill('【编辑自测】总结草稿中的观察')
    page.get_by_role('textbox', name='整次训练观察', exact=True).fill('【编辑自测】保存后还可以继续补充')
    page.get_by_role('button', name='保存总结草稿并退出', exact=True).click()
    expect(page.get_by_role('button', name='填写干预总结', exact=True)).to_be_visible()
    saved_draft = entry('sessions', draft['id'])
    assert saved_draft['targets'][0]['goal_id'] == second['id']
    assert saved_draft['targets'][0]['progress'] == '【编辑自测】总结草稿中的观察'
    assert saved_draft['observation'] == '【编辑自测】保存后还可以继续补充'
    assert saved_draft['status'] == 'draft'
    assert len(state()['sessions']) == 2 and len(state()['experiences']) == 2
    page.get_by_role('button', name='填写干预总结', exact=True).click()
    expect(page.get_by_role('textbox', name='主目标实际进度', exact=True)).to_have_value(saved_draft['targets'][0]['progress'])
    expect(page.get_by_role('textbox', name='整次训练观察', exact=True)).to_have_value(saved_draft['observation'])
    page.get_by_role('button', name='返回', exact=True).click()
    checks.append('same draft plan and summary edited; primary replaced/removed/restored; no duplicate training/experience')

    # Archive/stop are reversible status edits, not deletion.
    goal(goal_name)
    page.get_by_role('button', name='修改／复盘目标', exact=True).click()
    page.get_by_role('button', name='已归档', exact=True).click()
    page.get_by_role('button', name='保存目标复盘', exact=True).click()
    expect(page.get_by_text('目标复盘已保存，修改历史已保留。', exact=True)).to_be_visible()
    assert entry('goals', ids['goal'])['status'] == 'archived'
    assert entry('sessions', ids['session']) == old_session
    page.get_by_role('button', name='修改／复盘目标', exact=True).click()
    page.get_by_role('button', name='活动', exact=True).click()
    page.get_by_role('button', name='保存目标复盘', exact=True).click()
    expect(page.get_by_text('目标复盘已保存，修改历史已保留。', exact=True)).to_be_visible()
    method()
    page.get_by_role('radio', name='已归档', exact=True).click()
    page.get_by_role('button', name='保存方法', exact=True).click()
    expect(page.get_by_text('方法已保存。', exact=True)).to_be_visible()
    assert entry('pathways', ids['method'])['status'] == 'archived'
    assert entry('pathways', ids['method'])['goal_ids'] == [second['id']]
    assert entry('experiences', ids['experience']) == generated
    method()
    page.get_by_role('radio', name='使用中', exact=True).click()
    page.get_by_role('button', name='保存方法', exact=True).click()
    expect(page.get_by_text('方法已保存。', exact=True)).to_be_visible()
    page.get_by_role('button', name='跟进', exact=True).click()
    card = page.get_by_test_id('followup-' + ids['principle'])
    card.get_by_role('button', name='更新跟进', exact=True).click()
    card.get_by_role('button', name='停用', exact=True).click()
    with page.expect_response(lambda r: r.request.method == 'PUT' and r.url.endswith('/experiences/' + ids['principle'])):
        card.get_by_role('button', name='保存跟进', exact=True).click()
    assert entry('experiences', ids['principle'])['status'] == 'stopped'
    assert entry('experiences', ids['principle'])['goal_ids'] == [second['id']]
    card.get_by_role('button', name='已验证', exact=True).click()
    with page.expect_response(lambda r: r.request.method == 'PUT' and r.url.endswith('/experiences/' + ids['principle'])):
        card.get_by_role('button', name='保存跟进', exact=True).click()
    assert entry('experiences', ids['principle'])['status'] == 'verified'
    checks.append('goal/method archived and restored; experience stopped and restored; related data retained')

    # Explicit negative probes: neither missing capability is called a pass.
    headers = {'X-Skarma-Request': '1', 'Origin': base}
    before_probes = state()
    unavailable = []
    for kind, item_id in [('goals', ids['goal']), ('pathways', ids['method']), ('experiences', ids['experience']), ('sessions', ids['session']), ('sessions', draft['id'])]:
        response = context.request.delete(base + '/api/' + kind + '/' + item_id, headers=headers)
        assert response.status == 405, (kind, response.status, response.text())
        assert state() == before_probes
        unavailable.append({'operation': 'DELETE', 'kind': kind, 'status': response.status})
    session_fields = {key: value for key, value in old_session.items() if key not in ['id', 'version', 'status', 'target_snapshots']}
    response = context.request.put(base + '/api/sessions/' + ids['session'], headers=headers,
                                   data={'operation_id': str(uuid.uuid4()), 'expected_version': old_session['version'], 'action': 'save_draft', 'session': session_fields})
    assert response.status == 409 and '已完成记录暂只支持回看' in response.json()['error']
    assert state() == before_probes
    unavailable.append({'operation': 'PUT completed session', 'status': response.status, 'error': response.json()['error']})
    print('CONFIRMED MISSING: deletion (5 probes HTTP 405), completed training revision (HTTP 409); state unchanged', flush=True)

    page.reload()
    expect(page.get_by_text('把今天的小进步，', exact=False)).to_be_visible()
    goal(second_name)
    for text in [method_name, principle_text, observation_text, saved_draft['plan']]:
        expect(page.get_by_text(text, exact=True)).to_be_visible()
    independent_browser = browser.new_context(viewport={'width': 390, 'height': 844})
    another = independent_browser.new_page()
    errors = []
    another.on('pageerror', lambda error: errors.append(str(error)))
    login(another)
    another.get_by_role('button', name='目标', exact=True).click()
    another.get_by_text(second_name, exact=True).click()
    for text in [method_name, principle_text, observation_text, saved_draft['plan']]:
        expect(another.get_by_text(text, exact=True)).to_be_visible()
    assert not errors, errors
    independent_browser.close()
    assert not page.evaluate('document.documentElement.scrollWidth > innerWidth')
    checks.append('edited graph retained on reload and a new independent authenticated browser')
    print('PASS UI edits, target/link removal and reversible status changes; exact saved IDs verified', flush=True)
    return {'checks': checks, 'unavailable': unavailable, 'second_goal_id': second['id'], 'draft_id': draft['id']}
