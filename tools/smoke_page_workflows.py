"""Six-page workflow regression. Only synthetic data on a disposable local host."""
import os
from datetime import datetime, timedelta
from pathlib import Path
from uuid import uuid4
from zoneinfo import ZoneInfo
from playwright.sync_api import expect, sync_playwright

BASE = os.environ['SKARMA_TEST_URL']
OUT = Path('.local/screenshots')
OUT.mkdir(parents=True, exist_ok=True)
today = datetime.now(ZoneInfo('Asia/Shanghai')).date()
prefix = '【页面验收】'

with sync_playwright() as p:
    browser = p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE', '/usr/bin/chromium'), headless=True, args=['--no-sandbox'])
    first = browser.new_context(viewport={'width': 390, 'height': 844})
    second = browser.new_context(viewport={'width': 390, 'height': 844})
    a, b = first.new_page(), second.new_page()
    errors = []
    for page in [a, b]:
        page.on('pageerror', lambda error: errors.append(str(error)))
        page.goto(BASE)
        page.get_by_role('textbox', name='账号', exact=True).fill('owner')
        page.get_by_label('密码', exact=True).fill('synthetic-owner-contract-password')
        page.get_by_role('button', name='登录', exact=True).click()
        expect(page.get_by_text('把今天的小进步，', exact=False)).to_be_visible()
        page.get_by_role('button', name='目标', exact=True).click()

    def state():
        response = first.request.get(BASE + '/api/state')
        assert response.ok
        return response.json()

    def write(method, path, body):
        response = first.request.fetch(BASE + '/api' + path, method=method, data=body, headers={'X-Skarma-Request': '1'})
        assert response.ok, response.text()
        return response.json()

    def focus(page):
        page.evaluate("window.dispatchEvent(new Event('focus'))")

    # Goal creation has local validation, optional hierarchy, confirmed state and cross-browser refresh.
    a.get_by_role('button', name='＋ 新建目标', exact=True).click()
    a.get_by_role('button', name='保存目标', exact=True).click()
    expect(a.get_by_text('请填写目标名称。', exact=True)).to_be_visible()
    a.get_by_role('textbox', name='目标名称', exact=True).fill(prefix + '独立沟通目标')
    a.get_by_role('combobox', name='选择已有领域分类', exact=True).select_option('沟通')
    a.get_by_role('button', name='保存目标', exact=True).click()
    expect(a.get_by_text('新目标已保存。', exact=True)).to_be_visible()
    goal = next(item for item in state()['goals'] if item['title'] == prefix + '独立沟通目标')
    assert goal['parent_id'] is None
    focus(b)
    expect(b.get_by_text(goal['title'], exact=True)).to_be_visible()
    a.get_by_text(goal['title'], exact=True).click()
    a.get_by_role('button', name='修改／复盘目标', exact=True).click()
    expect(a.get_by_role('textbox', name='本次复盘日期（可选）', exact=True)).to_have_value(str(today))
    expect(a.get_by_role('textbox', name='下次复盘日期（可选）', exact=True)).to_have_value(str(today + timedelta(days=14)))
    assert a.locator('input[type=date]').count() == 5
    a.get_by_role('textbox', name='进度与复盘记录', exact=True).fill(prefix + '我的未提交观察')
    remote = write('PUT', '/goals/' + goal['id'], {**goal, 'expected_version': goal['version'], 'criteria': prefix + '另一位家人的标准', 'change_note': '并发测试'})
    focus(a)
    expect(a.get_by_role('button', name='备份我的填写并采用最新目标', exact=True)).to_be_visible()
    expect(a.get_by_role('textbox', name='进度与复盘记录', exact=True)).to_have_value(prefix + '我的未提交观察')
    expect(a.get_by_role('button', name='保存目标复盘', exact=True)).to_be_disabled()
    a.get_by_role('button', name='备份我的填写并采用最新目标', exact=True).click()
    a.get_by_role('textbox', name='本次复盘日期（可选）', exact=True).fill(str(today))
    a.get_by_role('textbox', name='下次复盘日期（可选）', exact=True).fill(str(today))
    a.get_by_role('button', name='保存目标复盘', exact=True).click()
    expect(a.get_by_text('目标复盘已保存，修改历史已保留。', exact=True)).to_be_visible()
    a.get_by_role('button', name='今天', exact=True).click()
    expect(a.get_by_text('目标复盘提醒', exact=True)).to_be_visible()
    a.get_by_role('button', name='今天不再弹出，稍后从列表查看', exact=True).click()
    print('PASS goal create/review/date controls, two-browser refresh, dirty conflict preservation and due reminder', flush=True)

    # Plan/summary separation, primary selection, one-goal completion and linked follow-ups.
    method = write('POST', '/pathways', {'id': str(uuid4()), 'title': prefix + '视觉提示', 'description': '支持沟通', 'steps': '展示选项', 'goal_ids': [goal['id']]})
    focus(a)
    a.get_by_role('button', name='＋ 另建干预计划', exact=True).click()
    a.get_by_role('button', name='游戏', exact=True).click()
    a.get_by_role('textbox', name='整次训练安排', exact=True).fill(prefix + '家人安排')
    a.get_by_role('button', name='＋ 主目标', exact=True).click()
    a.get_by_role('button', name=goal['title'], exact=True).click()
    a.get_by_role('combobox', name='更换主目标', exact=True).select_option(goal['id'])
    a.get_by_role('button', name='保存计划并退出', exact=True).click()
    expect(a.get_by_text('草稿已保存到服务器。干预后从今天的安排填写总结。', exact=True)).to_be_visible()
    planned = next(item for item in state()['sessions'] if item['plan'] == prefix + '家人安排')
    assert planned['status'] == 'draft' and len(planned['targets']) == 1
    # Choose this exact plan card, even if earlier regression tests saved another plan.
    a.get_by_text(prefix + '家人安排', exact=True).click()
    expect(a.get_by_role('button', name='提交干预总结', exact=True)).to_be_visible()
    a.get_by_role('textbox', name='主目标实际进度', exact=True).fill(prefix + '实际观察')
    a.get_by_role('button', name='继续', exact=True).click()
    a.get_by_role('button', name='是', exact=True).nth(0).click()
    a.get_by_role('textbox', name='困难详情', exact=True).fill(prefix + '需要等待')
    expect(a.get_by_role('combobox', name='困难关联目标', exact=True)).to_have_value(goal['id'])
    options = a.get_by_role('combobox', name='困难关联目标', exact=True).locator('option').all_text_contents()
    assert options[1].startswith('本次计划 · ')
    a.get_by_role('combobox', name='困难关联方法', exact=True).select_option(method['id'])
    a.get_by_role('button', name='是', exact=True).nth(1).click()
    a.get_by_role('textbox', name='成功经验详情', exact=True).fill(prefix + '留出等待时间')
    a.get_by_role('combobox', name='经验关联方法', exact=True).select_option(method['id'])
    a.get_by_role('button', name='提交干预总结', exact=True).click()
    expect(a.get_by_text('这次训练已完成，困难与经验已按填写内容保存。', exact=True)).to_be_visible()
    current = state()
    difficulty = next(item for item in current['difficulties'] if item['source_session_id'] == planned['id'])
    experience = next(item for item in current['experiences'] if item['source_session_id'] == planned['id'])
    assert difficulty['goal_ids'] == experience['goal_ids'] == [goal['id']]
    assert difficulty['pathway_ids'] == experience['pathway_ids'] == [method['id']]
    a.get_by_role('button', name='目标', exact=True).click()
    a.get_by_text(goal['title'], exact=True).click()
    expect(a.get_by_text(prefix + '需要等待', exact=True)).to_be_visible()
    expect(a.get_by_text(prefix + '留出等待时间', exact=True)).to_be_visible()
    a.get_by_role('button', name='查看方法：' + method['title'], exact=True).click()
    expect(a.get_by_role('button', name='查看目标：' + goal['title'], exact=True)).to_be_visible()
    expect(a.get_by_text(prefix + '留出等待时间', exact=True)).to_be_visible()
    a.get_by_role('button', name='跟进', exact=True).click()
    card = a.get_by_test_id('followup-' + experience['id'])
    card.get_by_role('button', name='更新跟进', exact=True).click()
    card.get_by_role('button', name='已验证', exact=True).click()
    card.get_by_role('textbox', name='验证依据', exact=True).fill(prefix + '连续观察三次')
    card.get_by_role('button', name='保存跟进', exact=True).click()
    expect(card.get_by_text('验证依据：' + prefix + '连续观察三次', exact=True)).to_be_visible()
    print('PASS plan/summary separation, optional secondary and goal/method/difficulty/experience visibility', flush=True)

    # Separate institution/family lists and last calendar month teacher statistics.
    last_end = today.replace(day=1) - timedelta(days=1)
    last_start = last_end.replace(day=1)
    course, teacher = prefix + '机构音乐课', prefix + '甲老师'
    for day in [last_start, last_end, today]:
        write('PUT', '/lessons/' + str(uuid4()), {'expected_version': None, 'lesson': {'business_date': str(day), 'course': course, 'teacher': teacher, 'content': '机构课程', 'goals': '', 'problems': '', 'home_cooperation': ''}})
    a.get_by_role('button', name='记录', exact=True).click()
    focus(a)
    expect(a.get_by_text(teacher + '：2 节课', exact=True)).to_be_visible()
    a.get_by_text('机构课时统计', exact=True).scroll_into_view_if_needed()
    a.screenshot(path=str(OUT / 'page-workflows-records.png'))
    expect(a.get_by_text(prefix + '家人安排', exact=True)).to_have_count(0)
    a.get_by_role('button', name='家庭干预', exact=True).click()
    expect(a.get_by_text(prefix + '家人安排', exact=True)).to_be_visible()
    expect(a.get_by_text(course, exact=True)).to_have_count(0)
    a.get_by_role('button', name='机构课程', exact=True).click()
    a.get_by_role('button', name='维护课程和老师选项', exact=True).click()
    a.get_by_role('button', name='停用老师：' + teacher, exact=True).click()
    expect(a.get_by_role('button', name='恢复老师：' + teacher, exact=True)).to_be_visible()
    expect(a.get_by_text(teacher + '：2 节课', exact=True)).to_be_visible()
    a.get_by_role('button', name='＋ 新建上课记录', exact=True).click()
    expect(a.get_by_role('combobox', name='授课老师（必填）', exact=True).locator('option', has_text=teacher)).to_have_count(0)
    a.get_by_role('combobox', name='课程名称（必填）', exact=True).select_option(course)
    a.get_by_role('combobox', name='授课老师（必填）', exact=True).select_option('__new__')
    a.get_by_role('textbox', name='新增老师名称', exact=True).fill(prefix + '乙老师')
    a.get_by_role('button', name='保存并使用新选项', exact=True).click()
    a.get_by_role('textbox', name='上课内容', exact=True).fill(prefix + '新的机构课')
    a.get_by_role('button', name='保存上课记录', exact=True).click()
    expect(a.get_by_text('上课记录已保存。', exact=True)).to_be_in_viewport()
    a.get_by_role('button', name='今天', exact=True).click()
    assert not a.get_by_text(str(last_start), exact=True).count(), 'today must not contain old institution or family records'
    assert a.get_by_role('button', name='规划', exact=True).count() == 0
    a.screenshot(path=str(OUT / 'page-workflows-today.png'), full_page=True)
    assert not a.evaluate('document.documentElement.scrollWidth > innerWidth')
    assert not errors, errors
    browser.close()
print('PASS institutions/family record switch, calendar-month statistics, shared options and archive preservation')
