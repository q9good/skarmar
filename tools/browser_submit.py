"""Hold a real synthetic save response so transient UI states are observable."""
import json
from playwright.sync_api import expect


def submit_while_pending(page, button, *, lose_response=False, screenshot=None):
    held = []

    def hold(route):
        if route.request.method != 'PUT':
            route.continue_()
            return
        response = route.fetch()
        assert response.ok
        held.append((route, response))

    pattern = '**/api/sessions/*'
    page.route(pattern, hold)
    released = False
    try:
        button.click()
        hint = page.get_by_text('正在保存，请稍候…', exact=True)
        expect(hint).to_be_visible()
        expect(page.get_by_role('button', name='正在保存…', exact=True)).to_be_disabled()
        expect(page.get_by_role('button', name='重试同一次提交', exact=True)).to_have_count(0)
        expect(page.get_by_text('上次提交结果尚未确认，请重试同一次提交。', exact=True)).to_have_count(0)
        assert len(held) == 1
        route, response = held[0]
        payload = route.request.post_data_json
        if screenshot:
            hint.scroll_into_view_if_needed()
            page.screenshot(path=str(screenshot))
        if lose_response:
            route.fulfill(status=503, content_type='application/json', body=json.dumps({'error': '验收：网关未确认提交结果'}))
        else:
            route.fulfill(response=response)
        released = True
        return payload
    finally:
        if not released:
            for route, _ in held:
                route.abort()
        page.unroute(pattern, hold)
