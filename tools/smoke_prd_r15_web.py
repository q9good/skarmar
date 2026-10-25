"""R15 accessibility, reflow and conditional-refresh checks on a disposable localhost PRD database."""
import json,os,re
from urllib.parse import urlparse
from playwright.sync_api import expect,sync_playwright

BASE=os.environ['SKARMA_TEST_URL'].rstrip('/')
assert urlparse(BASE).hostname in {'localhost','127.0.0.1','::1'}

CONTRAST_JS="""(el,pseudo)=>{
 const rgba=value=>{const m=value.match(/[\\d.]+/g);return m?m.map(Number):[0,0,0,0];};
 const linear=value=>{value/=255;return value<=.04045?value/12.92:Math.pow((value+.055)/1.055,2.4);};
 const luminance=color=>{const [r,g,b]=rgba(color);return .2126*linear(r)+.7152*linear(g)+.0722*linear(b);};
 const foreground=getComputedStyle(el,pseudo||null).color;
 let node=el,background='rgba(0, 0, 0, 0)';
 while(node){const candidate=getComputedStyle(node).backgroundColor;if(rgba(candidate)[3]!==0){background=candidate;break;}node=node.parentElement;}
 const values=[luminance(foreground),luminance(background)].sort((a,b)=>b-a);
 return (values[0]+.05)/(values[1]+.05);
}"""

with sync_playwright() as p:
 browser=p.chromium.launch(executable_path=os.environ.get('CHROMIUM_EXECUTABLE'),headless=True,args=['--no-sandbox'])
 context=browser.new_context(viewport={'width':360,'height':640})
 page=context.new_page()
 errors=[]
 page.on('pageerror',lambda error:errors.append(str(error)))
 def button(name,root=None):
  return (root or page).get_by_role('button',name=name,exact=True)
 def field(name,root=None):
  return (root or page).get_by_role('textbox',name=name,exact=True)
 def no_horizontal_overflow():
  assert page.evaluate('document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1')
 page.goto(BASE)
 field('账号').fill('owner')
 page.get_by_label('密码',exact=True).fill('synthetic-prd-ui-password')
 button('登录').click()
 expect(button('记录上课')).to_be_enabled()
 expect(button('刷新')).to_have_count(0)

 # Refresh is available only after an actual sync failure.
 page.route('**/api/prd/state',lambda route:route.fulfill(status=503,content_type='application/json',body='{"error":"合成同步失败"}'),times=1)
 with page.expect_response(re.compile('/api/prd/state$')):
  page.evaluate("window.dispatchEvent(new Event('focus'))")
 expect(page.get_by_role('alert')).to_have_text('合成同步失败')
 expect(button('重试同步')).to_be_visible()
 button('重试同步').click()
 expect(page.get_by_role('alert')).to_have_count(0)
 expect(button('重试同步')).to_have_count(0)

 # Required CSS viewports reflow without a page-wide horizontal scrollbar.
 for width,height in [(360,640),(430,932),(320,740)]:
  page.set_viewport_size({'width':width,'height':height})
  no_horizontal_overflow()
 page.set_viewport_size({'width':360,'height':640})

 button('目标').click()
 button('新建目标').click()
 dialog=page.get_by_role('dialog',name='填写目标',exact=True)
 title=field('本领名称',dialog)
 expect(title).to_have_value('')
 assert title.get_attribute('placeholder').startswith('例如：')
 title.fill('R15合成目标')
 button('补充目标说明（可选）',dialog).click()
 meaning=field('生活意义',dialog)
 criteria=field('观察标准',dialog)
 meaning.fill('帮助孩子更独立地参与吃饭')
 expect(dialog.get_by_text('这会帮助哪方面生活？',exact=True)).to_be_visible()
 button('查看填写示例：生活意义',dialog).click()
 expect(dialog.get_by_text('合成示例 · 例如：更独立地吃饭',exact=True)).to_be_visible()
 expect(meaning).to_have_value('帮助孩子更独立地参与吃饭')
 expect(criteria).to_have_value('')

 # AC-R15-04: measure actual computed sizes and contrast, including placeholder.
 label=dialog.get_by_text('本领名称',exact=True)
 question=dialog.get_by_text('想帮助孩子做到什么？',exact=True)
 save=button('保存',dialog)
 save_text=dialog.get_by_text('保存',exact=True)
 assert float(title.evaluate("el=>getComputedStyle(el).fontSize.replace('px','')"))>=16
 assert float(label.evaluate("el=>getComputedStyle(el).fontSize.replace('px','')"))>=17
 assert float(question.evaluate("el=>getComputedStyle(el).fontSize.replace('px','')"))>=14
 assert float(save_text.evaluate("el=>getComputedStyle(el).fontSize.replace('px','')"))>=17
 save_box=save.bounding_box()
 assert save_box['width']>=44 and save_box['height']>=44
 for locator,pseudo in [(title,None),(title,'::placeholder'),(label,None),(question,None),(save_text,None)]:
  assert locator.evaluate(CONTRAST_JS,pseudo)>=4.5

 # AC-R15-03: Chromium 200% page-scale simulation at an effective 320 CSS px.
 page.set_viewport_size({'width':640,'height':900})
 cdp=context.new_cdp_session(page)
 cdp.send('Emulation.setPageScaleFactor',{'pageScaleFactor':2})
 page.wait_for_function('window.visualViewport && window.visualViewport.scale >= 1.9')
 no_horizontal_overflow()
 expect(save).to_be_in_viewport(ratio=1)
 cdp.send('Emulation.setPageScaleFactor',{'pageScaleFactor':1})
 page.wait_for_function('!window.visualViewport || window.visualViewport.scale < 1.1')

 # Desktop keyboard/visualViewport simulation: shrink after focus and ensure focus and footer do not overlap.
 page.set_viewport_size({'width':360,'height':360})
 meaning.focus()
 meaning.scroll_into_view_if_needed()
 expect(meaning).to_be_focused()
 expect(meaning).to_be_in_viewport(ratio=.98)
 expect(save).to_be_in_viewport(ratio=.98)
 input_box,footer_box=meaning.bounding_box(),save.bounding_box()
 visual=page.evaluate("({top:visualViewport?.offsetTop||0,height:visualViewport?.height||innerHeight})")
 assert input_box['y']>=visual['top']-1
 assert input_box['y']+input_box['height']<=visual['top']+visual['height']+1
 assert footer_box['y']+footer_box['height']<=visual['top']+visual['height']+1
 assert input_box['y']+input_box['height']<=footer_box['y']+1
 no_horizontal_overflow()

 page.set_viewport_size({'width':360,'height':640})
 save.click()
 expect(dialog).to_have_count(0)
 state=context.request.get(BASE+'/api/prd/state').json()
 goal=next(goal for goal in state['goals'] if goal['input']['title']=='R15合成目标')
 assert goal['input']['criteria']==''
 assert '例如：' not in json.dumps(goal,ensure_ascii=False)
 assert not errors,errors
 print('PASS R15: examples stay out of values/requests; labels and examples persist; 320/360/430 reflow, 200% desktop scale, reduced visual viewport, font sizes, hit targets and 4.5:1 contrast',flush=True)
 browser.close()
