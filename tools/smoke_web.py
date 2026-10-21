"""Run against a disposable demo DB served on port 3001. Requires Playwright + Chromium."""

import json
import os
from pathlib import Path
from urllib.request import Request, urlopen, build_opener, ProxyHandler
from urllib.parse import urlparse

from playwright.sync_api import expect, sync_playwright
from browser_submit import submit_while_pending

BASE = os.environ.get("SKARMA_TEST_URL", "http://127.0.0.1:3001")
if urlparse(BASE).hostname in ('127.0.0.1', 'localhost', '::1'):
    # The local test server is in this executor, outside the Internet proxy.
    urlopen = build_opener(ProxyHandler({})).open
OUTPUT = Path(".local/screenshots")
OUTPUT.mkdir(parents=True, exist_ok=True)
with urlopen(f"{BASE}/api/auth/me") as response:
    identity = json.load(response)
SCOPE = f"{identity['space']}:{identity['user']['id']}"


def state():
    with urlopen(f"{BASE}/api/state") as response:
        return json.load(response)


assert not state()["sessions"], "Use an empty disposable validation DB for this test."
with sync_playwright() as playwright:
    browser = playwright.chromium.launch(
        executable_path=os.environ.get("CHROMIUM_EXECUTABLE", "/usr/bin/chromium"),
        headless=True,
        args=["--no-sandbox"],
    )
    context = browser.new_context(viewport={"width": 390, "height": 844})
    page = context.new_page()
    errors = []
    page.on("pageerror", lambda error: errors.append(str(error)))
    page.goto(BASE)
    expect(page.get_by_text("把今天的小进步，", exact=False)).to_be_visible()
    assert not page.evaluate("document.documentElement.scrollWidth > innerWidth")
    page.screenshot(path=str(OUTPUT / "today.png"), full_page=True)

    # Daily classes can be recorded without creating a training plan.
    page.get_by_role("button", name="＋ 记录今天上课", exact=True).click()
    expect(page.get_by_role("button", name="保存上课记录", exact=True)).to_be_disabled()
    page.get_by_role("combobox", name="课程名称（必填）", exact=True).select_option("__new__")
    page.get_by_role("textbox", name="新增课程名称", exact=True).fill("【浏览器验收】音乐课")
    page.get_by_role("button", name="保存并使用新选项", exact=True).click()
    page.get_by_role("combobox", name="授课老师（必填）", exact=True).select_option("__new__")
    page.get_by_role("textbox", name="新增老师名称", exact=True).fill("王老师")
    page.get_by_role("button", name="保存并使用新选项", exact=True).click()
    page.get_by_role("textbox", name="上课内容", exact=True).fill("一起唱歌")
    page.get_by_role("textbox", name="需要家里配合的", exact=True).fill("在家练习")
    expect(page.get_by_text("上课草稿已保存在本机", exact=True)).to_be_visible()
    page.reload()
    page.get_by_role("button", name="继续填写", exact=True).click()
    expect(page.get_by_role("combobox", name="课程名称（必填）", exact=True)).to_have_value("【浏览器验收】音乐课")
    expect(page.get_by_role("textbox", name="需要家里配合的", exact=True)).to_have_value("在家练习")
    page.get_by_role("button", name="保存上课记录", exact=True).click()
    expect(page.get_by_text("上课记录已保存。", exact=True)).to_be_visible()
    saved_lesson = state()["lessons"][0]
    assert saved_lesson["course"] == "【浏览器验收】音乐课"
    assert saved_lesson["teacher"] == "王老师"
    assert not state()["sessions"]
    page.get_by_role("button", name="记录", exact=True).click()
    page.get_by_text("【浏览器验收】音乐课", exact=True).click()
    page.get_by_role("textbox", name="存在问题", exact=True).fill("暂时跟不上节奏")
    page.get_by_role("button", name="保存上课记录", exact=True).click()
    assert state()["lessons"][0]["version"] == 2
    assert state()["lessons"][0]["problems"] == "暂时跟不上节奏"
    page.get_by_role("button", name="今天", exact=True).click()

    page.get_by_role("button", name="＋ 新建干预计划", exact=True).click()
    page.get_by_role("button", name="游戏", exact=True).click()
    page.get_by_role("textbox", name="整次训练安排", exact=True).fill("【浏览器验收】一起完成轮流游戏")
    page.get_by_role("button", name="＋ 主目标", exact=True).click()
    page.get_by_role("button", name="主动请求帮助", exact=True).click()
    page.get_by_role("button", name="＋ 副目标", exact=True).click()
    page.get_by_role("button", name="轮流参与活动", exact=True).click()
    page.get_by_role("textbox", name="主目标计划", exact=True).fill("需要时表达帮助请求")
    page.get_by_role("textbox", name="副目标计划", exact=True).fill("观察轮流和帮助情况")
    submit_while_pending(page, page.get_by_role("button", name="保存计划并退出", exact=True), screenshot=OUTPUT / 'training-saving.png')
    expect(page.get_by_text("草稿已保存到服务器。干预后从今天的安排填写总结。", exact=True)).to_be_visible()
    planned = state()["sessions"][0]
    assert planned["status"] == "draft" and planned["version"] == 1
    assert len(state()["sessions"]) == 1

    page.reload()
    page.get_by_role("button", name="填写干预总结", exact=True).click()
    page.get_by_role("button", name="修改本次计划", exact=True).click()
    expect(page.get_by_role("textbox", name="整次训练安排", exact=True)).to_have_value(planned["plan"])
    page.get_by_role("button", name="转到干预总结", exact=True).click()
    context.set_offline(True)
    page.get_by_role("textbox", name="主目标实际进度", exact=True).fill("【浏览器验收】现场观察草稿")
    expect(page.get_by_text("草稿已保存在本机", exact=False)).to_be_visible()
    context.set_offline(False)
    page.reload()
    page.get_by_role("button", name="继续填写", exact=True).click()
    expect(page.get_by_role("textbox", name="主目标实际进度", exact=True)).to_have_value("【浏览器验收】现场观察草稿")
    page.get_by_role("button", name="达成归档", exact=True).nth(0).click()
    page.get_by_role("button", name="继续", exact=True).nth(1).click()
    page.get_by_role("button", name="是", exact=True).nth(0).click()
    page.get_by_role("textbox", name="困难详情", exact=True).fill("【浏览器验收】需要继续观察的困难")
    page.get_by_role("button", name="是", exact=True).nth(1).click()
    page.get_by_role("textbox", name="成功经验详情", exact=True).fill("【浏览器验收】在熟悉场景中提供选择")
    assert not page.evaluate("document.documentElement.scrollWidth > innerWidth")

    # Commit server-side, then replace the response with a gateway failure.
    # A reload must preserve and replay the same operation.
    pending = submit_while_pending(page, page.get_by_role("button", name="提交干预总结", exact=True), lose_response=True)
    expect(page.get_by_role("button", name="重试同一次提交", exact=True)).to_be_visible()
    expect(page.get_by_text('上次提交结果尚未确认，请重试同一次提交。', exact=True)).to_be_visible()
    page.reload()
    page.get_by_role("button", name="继续填写", exact=True).click()
    expect(page.get_by_text('上次提交结果尚未确认，请重试同一次提交。', exact=True)).to_be_visible()
    replay = submit_while_pending(page, page.get_by_role("button", name="重试同一次提交", exact=True))
    assert replay == pending, 'reload and retry must retain the exact operation ID and request'
    expect(page.get_by_text("这次训练已完成，困难与经验已按填写内容保存。", exact=True)).to_be_visible()
    completed = state()
    assert len(completed["sessions"]) == 1
    assert completed["sessions"][0]["id"] == planned["id"]
    assert completed["sessions"][0]["version"] == 2
    assert completed["sessions"][0]["status"] == "completed"
    assert len(completed["difficulties"]) == len(completed["experiences"]) == 1
    goal_id = completed["sessions"][0]["targets"][0]["goal_id"]
    assert next(g for g in completed["goals"] if g["id"] == goal_id)["status"] == "archived"
    assert completed["sessions"][0]["target_snapshots"][0]["goal_snapshot"]["status"] == "active"
    page.screenshot(path=str(OUTPUT / "completed.png"), full_page=True)

    page.get_by_role("button", name="查看困难与经验跟进", exact=True).click()
    page.get_by_role("button", name="更新跟进", exact=True).nth(0).click()
    expect(page.get_by_text('可以只选目标、只选方法，也可以同时选择。方法服务哪些目标，不会自动替当前记录增加绑定。', exact=True)).to_be_visible()
    expect(page.get_by_text('不会自动替这条经验增加绑定。', exact=False)).to_have_count(0)
    page.get_by_role('textbox', name='困难内容', exact=True).scroll_into_view_if_needed()
    page.screenshot(path=str(OUTPUT / 'difficulty-binding-copy.png'))
    page.get_by_role("button", name="跟进中", exact=True).click()
    page.get_by_role("textbox", name="跟进结论", exact=True).fill("【浏览器验收】继续核对现场条件")
    page.get_by_role("textbox", name="下次回看日期（可选）", exact=True).fill(planned["business_date"])
    page.get_by_role("button", name="保存跟进", exact=True).click()
    expect(page.get_by_text("跟进已保存。", exact=True)).to_be_visible()
    page.get_by_role("button", name="更新跟进", exact=True).click()
    page.get_by_role("button", name="已验证", exact=True).click()
    page.get_by_role("textbox", name="验证依据", exact=True).fill("【浏览器验收】示例核对依据")
    page.get_by_role("button", name="保存跟进", exact=True).nth(1).click()
    expect(page.get_by_text("验证记录已保存。", exact=True)).to_be_visible()
    followed = state()
    assert followed["difficulties"][0]["status"] == "active"
    assert followed["experiences"][0]["status"] == "verified"
    page.get_by_role("button", name="查看来源训练", exact=True).nth(0).click()
    expect(page.get_by_text("训练回看", exact=True)).to_be_visible()

    # Progress and archive are independent; historical snapshots stay unchanged.
    secondary_id = next(t["goal_id"] for t in followed["sessions"][0]["targets"] if t["role"] == "secondary")
    secondary = next(g for g in followed["goals"] if g["id"] == secondary_id)
    page.get_by_role("button", name="目标", exact=True).click()
    page.get_by_text(secondary["title"], exact=True).click()
    page.get_by_role("button", name="修改／复盘目标", exact=True).click()
    page.get_by_role("textbox", name="达成标准", exact=True).fill("【浏览器验收】修改后的观察标准")
    page.get_by_role("button", name="已完成", exact=True).click()
    page.get_by_role("textbox", name="下次复盘日期（可选）", exact=True).fill(planned["business_date"])
    page.get_by_role("textbox", name="本次修改说明（可选）", exact=True).fill("【浏览器验收】本次复盘")
    page.get_by_role("button", name="保存目标复盘", exact=True).click()
    expect(page.get_by_text("目标复盘已保存，修改历史已保留。", exact=True)).to_be_visible()
    revised = next(g for g in state()["goals"] if g["id"] == secondary_id)
    assert revised["progress_status"] == "completed" and revised["status"] == "active"
    snapshot = next(t["goal_snapshot"] for t in state()["sessions"][0]["target_snapshots"] if t["goal_id"] == secondary_id)
    assert snapshot["criteria"] == secondary["criteria"]

    # A concurrent editor must not replace unsaved form contents on conflict.
    page.get_by_role("button", name="修改／复盘目标", exact=True).click()
    page.get_by_role("textbox", name="进度与复盘记录", exact=True).fill("【浏览器验收】我的未提交复盘")
    page.get_by_role("textbox", name="本次修改说明（可选）", exact=True).fill("【浏览器验收】我的修改")
    external = {key: value for key, value in revised.items() if key not in ("id", "level", "version")}
    external.update(expected_version=revised["version"], criteria="【浏览器验收】另一位记录者的标准", change_note="并发修改")
    with urlopen(Request(f"{BASE}/api/goals/{secondary_id}", data=json.dumps(external).encode(),
                         headers={"Content-Type": "application/json"}, method="PUT")) as response:
        assert response.status == 200
    page.get_by_role("button", name="保存目标复盘", exact=True).click()
    expect(page.get_by_role("button", name="备份我的填写并采用最新目标", exact=True)).to_be_visible()
    expect(page.get_by_role("textbox", name="进度与复盘记录", exact=True)).to_have_value("【浏览器验收】我的未提交复盘")
    expect(page.get_by_role("button", name="保存目标复盘", exact=True)).to_be_disabled()
    page.get_by_role("button", name="备份我的填写并采用最新目标", exact=True).click()
    expect(page.get_by_role("textbox", name="达成标准", exact=True)).to_have_value("【浏览器验收】另一位记录者的标准")
    assert page.evaluate("key => localStorage.getItem(key)", f"skarma:{SCOPE}:goal-conflict-backup:{secondary_id}")

    page.get_by_role("button", name="今天", exact=True).click()
    expect(page.get_by_text("目标复盘提醒", exact=True)).to_be_visible()
    page.get_by_role("button", name="今天不再弹出，稍后从列表查看", exact=True).click()
    expect(page.get_by_text("待复盘目标 · 1", exact=True)).to_be_visible()
    page.screenshot(path=str(OUTPUT / "goal-review.png"), full_page=True)
    page.get_by_role("button", name="＋ 新建干预计划", exact=True).click()
    page.get_by_role("button", name="＋ 主目标", exact=True).click()
    expect(page.get_by_role("button", name=secondary["title"], exact=True)).to_have_count(0)
    page.get_by_role("button", name="收起选择", exact=True).click()
    page.get_by_role("button", name="＋ 复习", exact=True).click()
    expect(page.get_by_role("button", name=secondary["title"], exact=True)).to_be_visible()
    assert not page.evaluate("document.documentElement.scrollWidth > innerWidth")
    assert not errors, f"Browser runtime errors: {errors}"
    browser.close()
print("PASS: mobile layout, independent class records and draft recovery, plan/refill identity, local draft recovery, lost-response replay, archive snapshot, follow-up, goal review/history, conflict preservation and role candidates")
