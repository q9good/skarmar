"""Shared navigation for the compact PRD home and single date control."""
from playwright.sync_api import expect


def start_plan(page):
    # History retains its new-plan entry; today offers a chooser and an extra-plan link.
    history = page.get_by_role('button', name='＋ 新建干预计划', exact=True)
    extra = page.get_by_role('button', name='另建干预计划', exact=True)
    if history.count():
        history.click()
    elif extra.count():
        extra.click()
    else:
        page.get_by_role('button', name='记录一次家庭干预', exact=True).click()
        page.get_by_role('button', name='准备去做：新建计划', exact=True).click()
    expect(page.get_by_role('textbox', name='整次训练安排', exact=True)).to_be_visible()


def date_digits(page, label):
    toggle = page.get_by_role('button', name=label + '：改用数字输入', exact=True)
    if toggle.count():
        toggle.click()
    return page.get_by_role('textbox', name=label, exact=True)
