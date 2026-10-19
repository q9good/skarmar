"""Administrator-only deletion and completed training revision through forms."""
import uuid
from playwright.sync_api import expect


def check_record_changes(page, context, browser, base, out, ids, second, draft, state):
    headers = {'X-Skarma-Request': '1', 'Origin': base}
    response = context.request.post(base + '/api/auth/users', headers=headers, data={
        'username': 'family-member', 'display_name': '合成家人成员', 'password': 'synthetic-family-member-password', 'role': 'member'})
    assert response.ok
    member_context = browser.new_context(viewport={'width': 390, 'height': 844})
    member = member_context.new_page()
    errors = []
    member.on('pageerror', lambda error: errors.append(str(error)))
    member.goto(base)
    member.get_by_role('textbox', name='账号', exact=True).fill('family-member')
    member.get_by_label('密码', exact=True).fill('synthetic-family-member-password')
    member.get_by_role('button', name='登录', exact=True).click()
    expect(member.get_by_text('把今天的小进步，', exact=False)).to_be_visible()

    def item(kind, record_id):
        return next(row for row in state()[kind] if row['id'] == record_id)

    original = item('sessions', ids['session'])
    original_experience = item('experiences', ids['experience'])

    def open_training(p):
        p.get_by_role('button', name='记录', exact=True).click()
        p.get_by_role('button', name='家庭干预', exact=True).click()
        p.get_by_text(original['plan'], exact=True).click()
        expect(p.get_by_role('button', name='修改训练', exact=True)).to_be_visible()

    open_training(member)
    expect(member.get_by_role('button', name='删除训练', exact=True)).to_have_count(0)
    member.get_by_role('button', name='修改训练', exact=True).click()
    member.get_by_role('textbox', name='本次修订说明', exact=True).fill('【修订自测】家人成员更正训练观察')
    member.get_by_role('textbox', name='整次训练观察', exact=True).fill('【修订自测】普通成员修改后的观察')
    expect(member.get_by_role('button', name='删除训练', exact=True)).to_have_count(0)
    member.get_by_role('textbox', name='本次修订说明', exact=True).scroll_into_view_if_needed()
    member.screenshot(path=str(out / 'member-training-revision.png'))
    member.get_by_role('button', name='保存训练修改', exact=True).click()
    expect(member.get_by_text('训练修改已保存，历史版本已保留。', exact=True)).to_be_visible()
    revised = item('sessions', ids['session'])
    assert revised['version'] == original['version'] + 1 and revised['status'] == 'completed'
    assert revised['target_snapshots'] == original['target_snapshots']
    assert item('experiences', ids['experience']) == original_experience
    history = member_context.request.get(base + '/api/sessions/' + ids['session'] + '/history').json()
    assert len(history) == 2 and history[1]['session'] == original

    member.get_by_role('button', name='修改训练', exact=True).click()
    member.get_by_role('textbox', name='整次训练观察', exact=True).fill('【修订自测】放弃提交的修改')
    member.get_by_role('button', name='备份填写并取消修改', exact=True).click()
    expect(member.get_by_text('本次修改未提交，填写已备份到本机。', exact=True)).to_be_visible()
    assert item('sessions', ids['session']) == revised
    cancelled = member.evaluate("Object.entries(localStorage).filter(([key]) => key.includes(':cancelled-revision-backup:')).map(([, value]) => JSON.parse(value))")
    assert any(value['input']['observation'] == '【修订自测】放弃提交的修改' for value in cancelled)

    member.get_by_role('button', name='修改训练', exact=True).click()
    member.get_by_role('textbox', name='本次修订说明', exact=True).fill('【修订自测】修正总结经验，保留单独维护的绑定')
    member.get_by_role('textbox', name='成功经验详情', exact=True).fill('【修订自测】家人确认后的经验')
    member.get_by_role('button', name='保存训练修改', exact=True).click()
    expect(member.get_by_text('训练修改已保存，历史版本已保留。', exact=True)).to_be_visible()
    linked = item('experiences', ids['experience'])
    assert item('sessions', ids['session'])['experience'] == '【修订自测】家人确认后的经验'
    assert linked == original_experience, 'verified experience is maintained separately from the original training summary'
    assert linked['goal_ids'] == original_experience['goal_ids'] and linked['pathway_ids'] == original_experience['pathway_ids']
    assert linked['conclusion'] == original_experience['conclusion'] and linked['status'] == original_experience['status']
    assert len(state()['experiences']) == 2

    # Both browsers edit the same version; the later writer keeps local input.
    member.get_by_role('button', name='修改训练', exact=True).click()
    page.get_by_role('button', name='今天', exact=True).click()
    page.get_by_role('button', name='刷新', exact=True).click()
    open_training(page)
    page.get_by_role('button', name='修改训练', exact=True).click()
    member.get_by_role('textbox', name='本次修订说明', exact=True).fill('【修订自测】先保存的版本')
    member.get_by_role('textbox', name='整次训练观察', exact=True).fill('【修订自测】先保存，其他人应看到冲突')
    member.get_by_role('button', name='保存训练修改', exact=True).click()
    expect(member.get_by_text('训练修改已保存，历史版本已保留。', exact=True)).to_be_visible()
    winning = item('sessions', ids['session'])
    page.get_by_role('textbox', name='本次修订说明', exact=True).fill('【修订自测】晚保存的版本')
    page.get_by_role('textbox', name='整次训练观察', exact=True).fill('【修订自测】冲突时保留我的填写')
    page.get_by_role('button', name='保存训练修改', exact=True).click()
    expect(page.get_by_text('发现另一个版本', exact=True)).to_be_visible()
    expect(page.get_by_role('textbox', name='整次训练观察', exact=True)).to_have_value('【修订自测】冲突时保留我的填写')
    assert item('sessions', ids['session']) == winning
    page.get_by_role('button', name='采用最新记录', exact=True).click()
    expect(page.get_by_text(winning['observation'], exact=True)).to_be_visible()

    # Direct API attempts also enforce member permissions for every business type.
    for kind, record_id in [('goals', ids['goal']), ('pathways', ids['method']), ('experiences', ids['experience']), ('sessions', ids['session']), ('difficulties', str(uuid.uuid4())), ('lessons', str(uuid.uuid4()))]:
        before = state()
        denied = member_context.request.delete(base + '/api/' + kind + '/' + record_id, headers=headers, data={'expected_version': 1})
        assert denied.status == 403 and '只有管理员' in denied.json()['error']
        assert state() == before

    # A referenced goal cannot be removed by accident.
    goal = item('goals', ids['goal'])
    page.get_by_role('button', name='目标', exact=True).click()
    page.get_by_text(goal['title'], exact=True).click()
    page.get_by_role('button', name='删除目标', exact=True).click()
    page.get_by_role('button', name='确认删除', exact=True).click()
    expect(page.get_by_text('目标仍被训练引用，请先修改或删除关联训练；停止使用可选择归档', exact=True)).to_be_visible()
    assert item('goals', ids['goal']) == goal
    page.get_by_text('目标仍被训练引用，请先修改或删除关联训练；停止使用可选择归档', exact=True).scroll_into_view_if_needed()
    page.screenshot(path=str(out / 'admin-referenced-goal-delete.png'))
    page.get_by_role('button', name='取消删除', exact=True).click()

    page.get_by_role('button', name='方法', exact=True).click()
    method = item('pathways', ids['method'])
    page.get_by_role('button', name='查看方法：' + method['title'], exact=True).click()
    page.get_by_role('button', name='删除方法', exact=True).click()
    page.get_by_role('button', name='取消删除', exact=True).click()
    assert item('pathways', ids['method']) == method
    page.get_by_role('button', name='删除方法', exact=True).click()
    page.get_by_role('button', name='确认删除', exact=True).click()
    expect(page.get_by_role('button', name='查看方法：' + method['title'], exact=True)).to_have_count(0)
    assert not state()['pathways']
    assert item('experiences', ids['experience'])['pathway_ids'] == []
    member.get_by_role('button', name='方法', exact=True).click()
    member.get_by_role('button', name='刷新', exact=True).click()
    expect(member.get_by_role('button', name='查看方法：' + method['title'], exact=True)).to_have_count(0)
    member.reload()
    expect(member.get_by_text('把今天的小进步，', exact=False)).to_be_visible()
    member.get_by_role('button', name='方法', exact=True).click()
    expect(member.get_by_role('button', name='查看方法：' + method['title'], exact=True)).to_have_count(0)

    page.get_by_role('button', name='跟进', exact=True).click()
    card = page.get_by_test_id('followup-' + ids['principle'])
    card.get_by_role('button', name='删除经验', exact=True).click()
    card.get_by_role('button', name='确认删除', exact=True).click()
    expect(card).to_have_count(0)
    assert len(state()['experiences']) == 1
    open_training(page)
    page.get_by_role('button', name='删除训练', exact=True).click()
    page.get_by_role('button', name='确认删除', exact=True).click()
    expect(page.get_by_text('记录已删除，其他记录保留。', exact=True)).to_be_visible()
    assert all(row['id'] != ids['session'] for row in state()['sessions'])
    retained = item('experiences', ids['experience'])
    assert retained['source_session_id'] is None and retained['source_deleted']
    assert retained['content'] == linked['content']
    page.get_by_role('button', name='跟进', exact=True).click()
    card = page.get_by_test_id('followup-' + ids['experience'])
    expect(card.get_by_text('原来源训练已删除 · 跟进保留', exact=True)).to_be_visible()
    expect(card.get_by_role('button', name='查看来源训练', exact=True)).to_have_count(0)
    card.scroll_into_view_if_needed()
    page.screenshot(path=str(out / 'deleted-training-retains-experience.png'))

    page.get_by_role('button', name='目标', exact=True).click()
    page.get_by_text(goal['title'], exact=True).click()
    page.get_by_role('button', name='删除目标', exact=True).click()
    page.get_by_role('button', name='确认删除', exact=True).click()
    expect(page.get_by_text(goal['title'], exact=True)).to_have_count(0)
    assert all(row['id'] != ids['goal'] for row in state()['goals'])
    page.get_by_role('button', name='记录', exact=True).click()
    page.get_by_role('button', name='家庭干预', exact=True).click()
    draft_plan = item('sessions', draft['id'])['plan']
    member.get_by_role('button', name='记录', exact=True).click()
    member.get_by_role('button', name='家庭干预', exact=True).click()
    member.get_by_text(draft_plan, exact=True).click()
    member.get_by_role('textbox', name='整次训练观察', exact=True).fill('【删除自测】另一位家人尚未提交的填写')
    page.get_by_text(draft_plan, exact=True).click()
    page.get_by_role('button', name='删除训练', exact=True).click()
    page.get_by_role('button', name='确认删除', exact=True).click()
    expect(page.get_by_text('记录已删除，其他记录保留。', exact=True)).to_be_visible()
    assert not state()['sessions']
    member.get_by_role('button', name='刷新', exact=True).click()
    expect(member.get_by_text('训练已被管理员删除，当前本机填写仍保留，无法保存到原记录。', exact=True)).to_be_visible()
    expect(member.get_by_role('button', name='保存总结草稿并退出', exact=True)).to_be_disabled()
    expect(member.get_by_role('textbox', name='整次训练观察', exact=True)).to_have_value('【删除自测】另一位家人尚未提交的填写')
    member.get_by_role('button', name='备份填写并关闭已删除草稿', exact=True).click()
    expect(member.get_by_text('已备份本机填写，可以新建其他记录。', exact=True)).to_be_visible()
    backups = member.evaluate("Object.entries(localStorage).filter(([key]) => key.includes(':deleted-training-backup:')).map(([, value]) => JSON.parse(value))")
    assert any(value['input']['observation'] == '【删除自测】另一位家人尚未提交的填写' for value in backups)
    page.get_by_role('button', name='跟进', exact=True).click()
    card = page.get_by_test_id('followup-' + ids['experience'])
    card.get_by_role('button', name='删除经验', exact=True).click()
    card.get_by_role('button', name='确认删除', exact=True).click()
    expect(card).to_have_count(0)
    page.get_by_role('button', name='目标', exact=True).click()
    page.get_by_text(second['title'], exact=True).click()
    page.get_by_role('button', name='删除目标', exact=True).click()
    page.get_by_role('button', name='确认删除', exact=True).click()
    expect(page.get_by_text(second['title'], exact=True)).to_have_count(0)
    page.reload()
    expect(page.get_by_text('把今天的小进步，', exact=False)).to_be_visible()
    final = state()
    assert all(not final[kind] for kind in ['goals', 'pathways', 'sessions', 'experiences'])
    assert len(final['deleted']) == 7
    assert not errors, errors
    member_context.close()
    print('PASS member training revisions, two-writer conflict, admin UI deletion/cancel, retained related records and cross-browser deletion refresh', flush=True)
    return {'member_revision': 'passed', 'admin_deletion': 'passed', 'member_delete_denied': 6, 'deleted_records': 7, 'concurrent_revision': 'passed'}
