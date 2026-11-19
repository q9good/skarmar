"""Exercise the visible mobile navigation and a plan-to-library journey on localhost."""
import os
import re
from pathlib import Path
from urllib.parse import urlparse
from uuid import uuid4

from playwright.sync_api import expect, sync_playwright

base = os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(base).hostname in ('127.0.0.1', 'localhost', '::1')
headers = {'X-Skarma-Request': '1', 'X-Skarma-Contract': 'skarmar.prd.v1',
           'X-Skarma-Data-Epoch': 'formal-20261101'}

with sync_playwright() as playwright:
    browser = playwright.chromium.launch(executable_path=os.getenv('CHROMIUM_EXECUTABLE'), args=['--no-sandbox'])
    context = browser.new_context(viewport={'width': 390, 'height': 844}, is_mobile=True, has_touch=True)
    page = context.new_page()
    page.set_default_timeout(8000)
    failures, writes = [], []
    page.on('pageerror', lambda error: failures.append(str(error)))
    page.on('request', lambda r: writes.append(r.post_data_json)
            if r.method == 'PUT' and '/prd/sessions/' in r.url else None)
    page.on('response', lambda r: failures.append(f'{r.status} {urlparse(r.url).path}')
            if r.status >= 400 and '/api/prd/' in r.url else None)

    def api(method, path, data=None):
        response = context.request.fetch(base + '/api' + path, method=method, headers=headers, data=data)
        assert response.ok, (path, response.status, response.text())
        return response.json()

    def seed(kind, value):
        key = str(uuid4())
        api('PUT', '/prd/records/' + key, {'id': key, 'operation_id': str(uuid4()),
            'expected_version': None, 'change_note': '', 'mutation': {'kind': kind, 'value': value},
            **({'method_create_mode': 'full'} if kind == 'method' else {})})
        return key

    def control(role, name):
        return page.get_by_role(role, name=name, exact=True)

    def tap(name):
        control('button', name).tap()

    def fill(name, value):
        control('textbox', name).fill(value)

    def screenshot(name, anchor):
        folder = Path('.local/family-followup-1119')
        folder.mkdir(parents=True, exist_ok=True)
        anchor.scroll_into_view_if_needed()
        page.screenshot(path=str(folder / (name + '.png')))

    def assert_drafts(count):
        expect(control('button', '继续课记')).to_have_count(count)
        expect(page.get_by_role('button', name=re.compile('^继续干预填写：'))).to_have_count(count)

    page.goto(base)
    fill('账号', 'owner')
    page.get_by_label('密码', exact=True).fill('synthetic-prd-ui-password')
    tap('登录')
    expect(control('button', '记录课堂')).to_be_enabled()
    goal = seed('goal', {'title': '合成轮流', 'level': 'long'})
    method = seed('method', {'title': '合成等待', 'steps': '留出反应时间', 'goal_ids': [goal]})
    course = seed('catalog', {'kind': 'course', 'name': '合成课程', 'active': True})
    teacher = seed('catalog', {'kind': 'teacher', 'name': '合成老师', 'active': True})
    today = page.evaluate("new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai'}).format(new Date())")
    for i in range(2):
        key = str(uuid4())
        api('PUT', '/prd/lessons/' + key, {'id': key, 'operation_id': str(uuid4()), 'expected_version': None,
            'action': {'kind': 'observation', 'value': {'business_date': today, 'teacher_id': teacher,
            'course_id': course, 'teacher_notes': f'合成观察{i}'}}})
    page.reload()
    tap('记录课堂')
    tap('关闭记录课堂')
    tap('记录家庭')
    tap('更多干预操作')
    tap('已经做完：直接记总结')
    tap('关闭填写干预总结')
    assert_drafts(1)
    for name in ('跟进', '经验库与困难', '困难', '目标与方法', '目标', '方法', '记录', '家庭干预', '我的账号'):
        tap(name)
        assert_drafts(0)
    page.reload()
    tap('今天')
    assert_drafts(1)
    print('PASS Today-only resume entries across main tabs, their subpages, and account', flush=True)

    plan_title = '合成一起滚球'
    tap('记录家庭')
    fill('准备做什么', plan_title)
    unrelated = str(uuid4())
    api('PUT', '/prd/sessions/' + unrelated, {'id': unrelated, 'operation_id': str(uuid4()),
        'expected_version': None, 'action': 'save_draft', 'change_note': '',
        'input': {'business_date': today, 'time_slot': '上午', 'plan': '合成另一份待总结计划'}})
    tap('保存计划并退出')
    expect(page.get_by_role('dialog')).to_have_count(0)
    saved_id = writes[-1]['id']
    screenshot('01-saved-plan-action', control('button', '填写这份计划的总结'))
    tap('填写这份计划的总结')
    expect(control('dialog', '填写干预总结')).to_be_visible()
    difficulty, experience = '合成：换球时需要帮助', '合成：等待后主动轮流'
    control('checkbox', '补充经验和困难（可选）').tap()
    tap('新增本次困难')
    fill('实际情况／必填 1', difficulty)
    tap('新增本次经验')
    fill('经验内容 2', experience)
    control('combobox', '经验阶段 2').select_option('try')
    tap('正式提交干预总结')
    expect(page.get_by_role('alert')).to_contain_text('实际目标')
    expect(page.get_by_role('alert')).to_be_in_viewport()
    tap('选择本次目标')
    control('checkbox', '选择目标：合成轮流').tap()
    fill('实际表现：合成轮流', '合成：完成一轮')
    tap('正式提交干预总结')
    expect(page.get_by_role('alert')).to_contain_text('实际方法')
    tap('实际方法')
    control('checkbox', '实际方法：合成等待').tap()
    expect(page.get_by_text('关联：合成轮流、合成等待', exact=True)).to_have_count(2)
    assert len(writes) == 1
    tap('正式提交干预总结')
    expect(page.get_by_role('dialog')).to_have_count(0)
    expect(control('button', '查看家庭干预：合成：完成一轮')).to_have_count(1)
    assert len(writes) == 2 and writes[-1]['id'] == saved_id and writes[-1]['action'] == 'complete'
    submitted = writes[-1]['input']['summary']['new_knowledge']
    state = api('GET', '/prd/state')
    rows = [k for k in state['knowledge'] if k['source_session_id'] == saved_id]
    assert len(rows) == len(submitted) == 2
    assert {r['input']['id'] for r in rows} == {k['id'] for k in submitted}
    assert {r['input']['content'] for r in rows} == {experience, difficulty}
    assert all(r['input']['goal_ids'] == [goal] and r['input']['method_ids'] == [method] for r in rows)
    assert next(s for s in state['sessions'] if s['id'] == saved_id)['status'] == 'completed'
    expect(control('button', '填写这份计划的总结')).to_have_count(0)
    tap('跟进')
    tap('经验库与困难')
    expect(control('button', '查看知识：' + experience)).to_be_visible()
    tap('困难')
    expect(control('button', '查看知识：' + difficulty)).to_be_visible()
    screenshot('02-followup-difficulty', control('button', '查看知识：' + difficulty))
    assert_drafts(0)
    page.reload()
    tap('目标与方法')
    for section, title in [('目标', '合成轮流'), ('方法', '合成等待')]:
        tap(section)
        tap('查看' + section + '：' + title)
        for prefix, content in [('关联经验：', experience), ('关联困难：', difficulty)]:
            expect(page.get_by_text(prefix + content, exact=True)).to_be_visible()
        tap('关闭查看详情与修改记录')
    tap('今天')
    tap('查看家庭干预：合成：完成一轮')
    tap('修改已完成总结')
    expect(control('textbox', '实际情况／必填 1')).to_have_value(difficulty)
    tap('关闭填写干预总结')
    print('PASS single plan ID, recoverable incomplete submit, both knowledge rows and cross-page links after completion and reload', flush=True)

    # A save receipt points to the exact plan even if it is not in today's list.
    tap('另建一次干预')
    tap('准备去做：新建计划')
    fill('准备做什么', '合成补录前一天')
    page.get_by_role('button', name=re.compile('修改日期时段$')).tap()
    page.get_by_label('干预日期：选择日期', exact=True).fill('2020-01-02')
    tap('保存计划并退出')
    expect(page.get_by_role('dialog')).to_have_count(0)
    previous_id = writes[-1]['id']
    tap('填写这份计划的总结')
    tap('调整总结目标与原计划')
    expect(page.get_by_text('原计划：合成补录前一天', exact=True)).to_be_visible()
    tap('暂存总结并退出')
    assert writes[-1]['id'] == previous_id and previous_id != saved_id
    expect(page.get_by_role('dialog')).to_have_count(0)
    # Restore the pre-upgrade shape of this account's local draft. A blank old
    # association can be repaired without retyping its content or resending a plan.
    tap('填写这份计划的总结')
    control('checkbox', '补充经验和困难（可选）').tap()
    tap('新增本次困难')
    fill('实际情况／必填 1', '合成旧草稿：需要提示')
    tap('选择本次目标')
    control('checkbox', '选择目标：合成轮流').tap()
    fill('实际表现：合成轮流', '合成：试了一次')
    tap('实际方法')
    control('checkbox', '实际方法：合成等待').tap()
    expect(page.get_by_text('草稿已存本机', exact=True)).to_be_visible()
    tap('关闭填写干预总结')
    page.evaluate("""id => {
        const key = Object.keys(localStorage).find(k => k.endsWith(':session-drafts:v1'));
        const rows = JSON.parse(localStorage.getItem(key));
        const draft = rows[id];
        delete draft.defaultKnowledgeLinks;
        const entry = draft.request.input.summary.new_knowledge[0];
        entry.goal_ids = []; entry.method_ids = [];
        localStorage.setItem(key, JSON.stringify(rows));
    }""", previous_id)
    page.reload()
    page.get_by_role('button', name=re.compile('^继续干预填写：2020-01-02')).tap()
    expect(control('textbox', '实际情况／必填 1')).to_have_value('合成旧草稿：需要提示')
    tap('正式提交干预总结')
    expect(page.get_by_role('alert')).to_contain_text('关联目标或方法')
    tap('关联本次目标和方法 1')
    tap('记录已有经验或困难的反馈')
    tap('无变化：' + difficulty)
    tap('暂存总结并退出')
    expect(page.get_by_role('dialog')).to_have_count(0)
    assert not [k for k in api('GET', '/prd/state')['knowledge'] if k['source_session_id'] == previous_id]
    tap('填写这份计划的总结')
    tap('记录已有经验或困难的反馈')
    expect(control('button', '取消本次反馈：' + difficulty)).to_be_visible()
    tap('正式提交干预总结')
    expect(page.get_by_role('dialog')).to_have_count(0)
    assert writes[-1]['id'] == previous_id and writes[-1]['action'] == 'complete'
    old_row, = [k for k in api('GET', '/prd/state')['knowledge'] if k['source_session_id'] == previous_id]
    assert old_row['input']['content'] == '合成旧草稿：需要提示'
    assert old_row['input']['goal_ids'] == [goal] and old_row['input']['method_ids'] == [method]
    print('PASS existing unlinked local draft repaired in place, original record ID and content retained', flush=True)
    events = api('GET', '/prd/process?view=session_feedback&id=' + previous_id)
    assert len(events) == 1 and events[0]['input']['result'] == 'same'
    assert next(s for s in api('GET', '/prd/state')['sessions'] if s['id'] == unrelated)['status'] == 'planned'
    assert not failures, failures
    print('PASS dated save receipt opens its own record; no uncaught error or failed business request', flush=True)
    browser.close()
