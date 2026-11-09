"""Navigate an actual labelled disclosure before acting on its retained fields.

Functional tests use this for input/actions; visibility assertions still inspect
the unmodified page directly. Nothing changes field values or server responses.
"""
import json
import re


def reveal_control(scope, role, name, exact=True):
    # These existing flows now enter the combined page before its maintenance tab.
    if role == 'button' and isinstance(name, str) and name in {'目标', '方法'}:
        legacy = scope.get_by_role(role, name=name, exact=exact)
        combined = scope.get_by_role('button', name='目标与方法', exact=True)
        legacy.or_(combined).first.wait_for(state='visible')
        if legacy.count() == 0 and combined.count() == 1:
            combined.click()
    # User-requested item editor and single Continue picker: navigate real controls.
    themes={'老师交代的练习或建议','我看到需要支持或提高的','我看到的好方法','做得更好了／在别处也会了','感兴趣、主动尝试、和人互动的','今天课上怎么样？','原因猜测（可选）'}
    if role=='textbox' and isinstance(name,str) and name in themes and (scope.get_by_role('checkbox',name=('原因猜测（待核实）' if name=='原因猜测（可选）' else name),exact=True).count()==1 or scope.get_by_role('textbox',name=name+' 1',exact=True,include_hidden=True).count()==1):
        title='原因猜测（待核实）' if name=='原因猜测（可选）' else name
        item=scope.get_by_role('textbox',name=title+' 1',exact=True,include_hidden=True)
        if item.count()==0:
            toggle=scope.get_by_role('checkbox',name=title,exact=True)
            if toggle.count()==1 and toggle.get_attribute('aria-checked')!='true':toggle.click()
            add=scope.get_by_role('button',name='添加一条'+title,exact=True)
            if add.count()==1:add.click()
        name=title+' 1'
    if role=='button' and isinstance(name,str) and name.startswith('继续课记：'):
        scope.get_by_role('button',name='继续课记',exact=True).click()
        title=name.split('：',1)[1]
        return scope.get_by_role('button',name=re.compile('^继续：.*'+(re.escape(title) if title!='整理或核对' else '(整理|核对)'))).first
    if role=='button' and isinstance(name,str) and ('填写示例：' in name) and scope.get_by_role(role,name=name,exact=True).count()==0:
        if scope.get_by_role(role,name=name+' 1',exact=True).count()==1:name+=' 1'
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
