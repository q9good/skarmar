"""Navigate an actual labelled disclosure before acting on its retained fields.

Functional tests use this for input/actions; visibility assertions still inspect
the unmodified page directly. Nothing changes field values or server responses.
"""
import json


def reveal_control(scope, role, name, exact=True):
    # These existing flows now enter the combined page before its maintenance tab.
    if role == 'button' and isinstance(name, str) and name in {'目标', '方法'}:
        legacy = scope.get_by_role(role, name=name, exact=exact)
        combined = scope.get_by_role('button', name='目标与方法', exact=True)
        legacy.or_(combined).first.wait_for(state='visible')
        if legacy.count() == 0 and combined.count() == 1:
            combined.click()
    target = scope.get_by_role(role, name=name, exact=exact, include_hidden=True)
    if target.count() == 1 and not target.is_visible():
        ids = target.evaluate("""e => {
            const ids=[];
            for(let p=e.parentElement;p;p=p.parentElement)
                if(p.id && getComputedStyle(p).display==='none')ids.unshift(p.id);
            return ids;
        }""")
        page = target.page
        for identifier in ids:
            toggle = page.locator('[aria-controls=' + json.dumps(identifier) + ']')
            if toggle.count() == 1:
                toggle.click()
    return scope.get_by_role(role, name=name, exact=exact)
