"""Build an offline review from inspected synthetic screenshots; no network/data access."""
import base64
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'docs/artifacts/validation/ui-redesign-2026-11-03'
SCREENS = [
    ('01-today', '今天：两个同级主入口', '课堂蓝、家庭陶土棕，尺寸和字重相同；长期关注用完整边框区分。'),
    ('02-plan-first', '计划第一步：准备做什么', '唯一新增的必填项在最上面。空白不能保存；日期时段保留默认，可按需修改。'),
    ('03-plan-selection', '关键目标', '先看当前选择，主要练、顺便练可以直接调整；近30天实际家庭与机构次数分别显示。'),
    ('11-plan-methods', '准备使用的方法', '可选。完整做法按需展开，保留快捷新增；实际方法仍需在总结中确认。'),
    ('10-method-picker', '选择计划方法', '相关方法在前，可多选和搜索；完成后回到原计划。'),
    ('04-plan-knowledge', '关联知识：经验', '合并所选目标、上级方向和所选方法的关联，去重并注明来源；未选方法的私有细节不带入。'),
    ('13-plan-difficulty', '关联知识：困难', '只显示当前选择有关且尚未解决的困难，标明目标或方法；完整说明按需查看，返回不丢计划。'),
    ('05-goals', '目标：松绿主题', '新建为实色主按钮，筛选按需展开，详情与编辑保留明确入口。'),
    ('06-methods', '方法：靛蓝主题', '新建、查看关联目标和编辑延续同一功能色，保留现有内容与排序。'),
    ('12-lesson-editor', '记录课堂：蓝色表单', '课程和老师仍可选、可新增，保留双向历史排序；提交按钮突出。'),
    ('07-classroom', '机构课程记录', '机构与家庭为同级切换，课堂记录与课时核对仍在原位置。'),
    ('08-experiences', '经验：绿色主题', '经验与困难保持平级；新建是实色主按钮，查看为白底次按钮。'),
    ('09-difficulties', '困难：赭金主题', '与经验保留相同层级和结构，颜色、标题与选中状态一起区分。'),
]

def build():
    checks = json.loads((OUT / 'checks.json').read_text())
    assert checks['synthetic'] is True and checks['errors'] == []
    screens = []
    for key, title, caption in SCREENS:
        raw = (OUT / (key + '.png')).read_bytes()
        assert key + '.png' in checks['screenshots']
        screens.append(dict(id=key, title=title, caption=caption,
            sha256=hashlib.sha256(raw).hexdigest(), image='data:image/png;base64,'+base64.b64encode(raw).decode()))
    digest = hashlib.sha256(json.dumps([(s['id'],s['sha256']) for s in screens]).encode()).hexdigest()
    payload = json.dumps(dict(reviewId='skarmar-ui-2026-11-03', revision=digest, screens=screens),ensure_ascii=False).replace('<','\\u003c')
    html = '''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'"><title>skarmar · UI评审 · 2026-11-03</title>
<style>*{box-sizing:border-box}body{margin:0;background:#F4F5F2;color:#23332F;font:16px/1.5 -apple-system,BlinkMacSystemFont,"PingFang SC",sans-serif}header{padding:20px 24px;border-bottom:1px solid #D5DED9;background:white;display:flex;align-items:center;justify-content:space-between;gap:16px;flex-wrap:wrap}.brand{display:flex;align-items:center;gap:10px;font-weight:700;font-size:22px}.brand img{width:28px;height:28px}h1{font-size:24px;margin:0 0 8px}p{margin:8px 0}button,textarea{font:inherit}button{min-height:44px;cursor:pointer;padding:10px 14px;border:1px solid #BDD0C3;border-radius:10px;background:white;color:#25624F}button:hover{background:#EAF2ED}button:focus-visible,textarea:focus{outline:3px solid #8B4B32;outline-offset:2px}.primary{background:#25624F;color:white}.primary:hover{background:#215442}button:disabled{opacity:.5;cursor:default}main{max-width:1280px;margin:auto;padding:24px;display:grid;grid-template-columns:250px minmax(0,1fr);gap:28px}nav{display:flex;flex-direction:column;gap:6px}nav button{text-align:left;min-width:0}nav button[aria-current=true]{background:#25624F;color:white}.content{min-width:0}.preview{display:flex;align-items:flex-start;gap:24px;flex-wrap:wrap;margin-top:24px}.shot{width:min(100%,390px);height:auto;border:1px solid #D5DED9;border-radius:16px;box-shadow:0 10px 30px #23332F12}.notes{flex:1;min-width:min(280px,100%);background:white;border:1px solid #D5DED9;border-radius:16px;padding:20px}textarea{width:100%;min-height:200px;resize:vertical;border:1px solid #BDD0C3;border-radius:10px;padding:12px;background:#FCFDFB;color:#23332F}label{display:block;font-weight:600;margin-bottom:10px}.muted{color:#566460;font-size:14px}.actions{display:flex;gap:8px;flex-wrap:wrap;margin-top:16px}.tag{display:inline-block;background:#EAF2ED;color:#25624F;border-radius:6px;padding:4px 8px;font-size:14px}.status{min-height:24px;color:#566460;font-size:14px}.tools{display:flex;align-items:center;gap:10px;flex-wrap:wrap}.tools label{font-size:14px;font-weight:400;margin:0}.wide .shot{width:min(100%,520px)}@media(max-width:700px){main{padding:16px;display:block}nav{flex-direction:row;overflow-x:auto;margin-bottom:20px;padding-bottom:8px}nav button{flex-shrink:0}header{padding:16px}.preview{gap:16px}.shot{margin:auto}.notes{width:100%}}@media(prefers-reduced-motion:reduce){*{scroll-behavior:auto}}</style>
<header><div class="brand"><img alt="" src="__LOGO__">skarmar <span class="tag">UI评审 · 未部署</span></div><div class="tools"><label><input id="zoom" type="checkbox"> 放大截图</label><button class="primary" id="export">导出审核意见</button></div></header>
<main><nav aria-label="评审画面"></nav><section class="content"><h1 id="title"></h1><p id="caption"></p><p class="muted">截图仅含本机合成数据。此文件是离线评审材料；填写不会更新线上应用，也不会自动发送意见。</p><div class="actions"><button id="prev">上一张</button><button id="next">下一张</button><span id="position" class="tag"></span></div><div class="preview"><img id="screen" class="shot"><section class="notes"><label for="note">这张页面的审核意见</label><textarea id="note" placeholder="例如：指出具体文字、按钮或顺序，并说明希望怎样调整。"></textarea><p id="saved" class="status" role="status"></p><p class="muted">可逐张填写后一次导出JSON，再将JSON发回会话。浏览器允许时自动保存到本机。</p><p class="muted">本轮保留账号、权限、所有正式业务数据、历史、草稿与离线队列，不改数据库结构或数据代际。</p></section></div></section></main>
<script id="data" type="application/json">__DATA__</script><script>
const data=JSON.parse(document.getElementById('data').textContent),key=data.reviewId+':'+data.revision;let notes={},index=0,persistent=true;
try{notes=JSON.parse(localStorage.getItem(key)||'{}');if(!notes||Array.isArray(notes)||typeof notes!=='object')notes={};}catch{persistent=false;}
const $=id=>document.getElementById(id),nav=document.querySelector('nav');
data.screens.forEach((s,i)=>{const b=document.createElement('button');b.textContent=String(i+1).padStart(2,'0')+' '+s.title;b.onclick=()=>show(i);nav.append(b);});
function show(i){index=i;const s=data.screens[index];$('title').textContent=s.title;$('caption').textContent=s.caption;$('screen').src=s.image;$('screen').alt=s.title+'：实际本机合成截图';$('note').value=typeof notes[s.id]==='string'?notes[s.id]:'';$('position').textContent=(index+1)+' / '+data.screens.length;$('prev').disabled=index===0;$('next').disabled=index===data.screens.length-1;[...nav.children].forEach((b,j)=>b.setAttribute('aria-current',String(j===index)));$('saved').textContent=persistent?'仅保存在当前浏览器，导出后可分享。':'本机保存不可用，请完成后直接导出。';}
$('note').oninput=()=>{notes[data.screens[index].id]=$('note').value;try{localStorage.setItem(key,JSON.stringify(notes));persistent=true;$('saved').textContent='意见已保存在本机。';}catch{persistent=false;$('saved').textContent='暂存在当前页面，请直接导出。';}};
$('prev').onclick=()=>show(index-1);$('next').onclick=()=>show(index+1);$('zoom').onchange=e=>document.body.classList.toggle('wide',e.target.checked);
$('export').onclick=()=>{const result={schema:'skarmar.review.v1',reviewId:data.reviewId,revision:data.revision,syntheticScreenshots:true,notes:data.screens.filter(s=>typeof notes[s.id]==='string'&&notes[s.id].trim()).map(s=>({screenId:s.id,title:s.title,screenshotSha256:s.sha256,note:notes[s.id]}))};const a=document.createElement('a'),url=URL.createObjectURL(new Blob([JSON.stringify(result,null,2)+'\\n'],{type:'application/json'}));a.href=url;a.download='skarmar-review-notes-2026-11-03.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);$('saved').textContent='已导出，请把JSON文件发回会话。';};
show(0);
</script></html>'''
    logo='data:image/svg+xml;base64,'+base64.b64encode((ROOT/'apps/client/public/icon.svg').read_bytes()).decode()
    (OUT/'review.html').write_text(html.replace('__DATA__',payload).replace('__LOGO__',logo))
    print(f'Built offline review: {len(screens)} synthetic screens, revision {digest}')

if __name__ == '__main__':
    build()
