"""Independent OOXML reader for synthetic export contracts; no Excel package required."""
import base64
import io
import json
import sys
import zipfile
from xml.etree import ElementTree as ET

NS = {'s': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
SHEETS = ['说明', '目标', '训练记录', '上课记录', '逐目标明细', '困难', '经验', '目标历史', '途径', '目的途径关系', '经验关联', '课程逐目标']


def read_workbook(data):
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        assert archive.testzip() is None
        book = ET.fromstring(archive.read('xl/workbook.xml'))
        names = [sheet.attrib['name'] for sheet in book.findall('s:sheets/s:sheet', NS)]
        assert names == SHEETS, names
        strings = []
        if 'xl/sharedStrings.xml' in archive.namelist():
            root = ET.fromstring(archive.read('xl/sharedStrings.xml'))
            strings = [''.join(si.itertext()) for si in root.findall('s:si', NS)]
        result = {}
        for index, name in enumerate(names, 1):
            root = ET.fromstring(archive.read(f'xl/worksheets/sheet{index}.xml'))
            assert root.find('.//s:f', NS) is None, 'user content must not become a formula'
            assert root.find('s:hyperlinks', NS) is None
            pane = root.find('s:sheetViews/s:sheetView/s:pane', NS)
            assert pane is not None and pane.attrib['ySplit'] == '1'
            assert root.find('s:autoFilter', NS) is not None
            rows = []
            for row in root.findall('s:sheetData/s:row', NS):
                cells = {}
                for cell in row.findall('s:c', NS):
                    column = 0
                    for letter in cell.attrib['r'].rstrip('0123456789'):
                        column = column * 26 + ord(letter) - ord('A') + 1
                    value = cell.find('s:v', NS)
                    cell_type = cell.attrib.get('t')
                    if value is None and cell_type is None:
                        cells[column - 1] = ''  # Formatted empty cell, not a numeric value.
                        continue
                    assert cell_type in ('s', 'inlineStr'), 'IDs/dates/content must remain text'
                    cells[column - 1] = strings[int(value.text)] if cell_type == 's' else ''.join(cell.find('s:is', NS).itertext())
                rows.append(cells)
            headers = [rows[0][i] for i in range(len(rows[0]))]
            assert len(headers) == len(set(headers)), name
            result[name] = [dict(zip(headers, [row.get(i, '') for i in range(len(headers))])) for row in rows[1:]]
        for name in archive.namelist():
            if name.endswith('.rels'):
                root = ET.fromstring(archive.read(name))
                assert all(rel.attrib.get('TargetMode') != 'External' for rel in root), 'no external links'
        return result


def verify_fixture(payload):
    sheets = read_workbook(base64.b64decode(payload['file']))
    state = payload['state']
    expected = {'目标': ('goals', '目标ID'), '训练记录': ('sessions', '训练ID'), '上课记录': ('lessons', '记录ID'), '困难': ('difficulties', '记录ID'), '经验': ('experiences', '记录ID'), '途径': ('pathways', '途径ID')}
    for name, (key, id_column) in expected.items():
        assert {row[id_column] for row in sheets[name]} == {item['id'] for item in state[key]}, name
        assert len(sheets[name]) == len(state[key]), name
    assert len(sheets['逐目标明细']) == sum(len(session['targets']) for session in state['sessions'])
    assert len(sheets['课程逐目标']) == sum(len(lesson.get('links', {}).get('targets', [])) for lesson in state['lessons'])
    for lesson in state['lessons']:
        links = lesson.get('links') or {}
        row = next(row for row in sheets['上课记录'] if row['记录ID'] == lesson['id'])
        assert row['实际方法ID'] == '\n'.join(links.get('pathway_ids', []))
        assert row['困难关联目标ID'] == '\n'.join(links.get('problem_goal_ids', []))
        assert row['困难关联方法ID'] == '\n'.join(links.get('problem_pathway_ids', []))
        assert row['加入困难跟进'] == ('是' if links.get('track_problem') else '否')
        for target in links.get('targets', []):
            detail = next(row for row in sheets['课程逐目标'] if row['上课ID'] == lesson['id'] and row['目标ID'] == target['goal_id'])
            assert detail['逐目标内容'] == target['plan']
            assert detail['逐目标观察'] == target['progress']
            assert detail['目标角色'] == target['role']
            snapshot = next((v['goal_snapshot'] for v in lesson.get('target_snapshots', []) if v['goal_id'] == target['goal_id']), None)
            assert detail['当时目标名称'] == (snapshot['title'] if snapshot else '')
            assert detail['当时达成标准'] == (snapshot['criteria'] if snapshot else '')
    for name, key in [('困难', 'difficulties'), ('经验', 'experiences')]:
        for item in state[key]:
            row = next(row for row in sheets[name] if row['记录ID'] == item['id'])
            assert row['来源上课ID'] == (item.get('source_lesson_id') or '')
    revisions = payload['history']
    assert {(row['目标ID'], row['版本']) for row in sheets['目标历史']} == {(r['goal']['id'], str(r['goal']['version'])) for r in revisions}
    assert len(sheets['目标历史']) == len(revisions)
    assert {(row['目的ID'], row['途径ID']) for row in sheets['目的途径关系']} == {(goal, pathway['id']) for pathway in state['pathways'] for goal in pathway['goal_ids']}
    assert len(sheets['目的途径关系']) == sum(len(pathway['goal_ids']) for pathway in state['pathways'])
    links = {(row['经验ID'], row['关联类型'], row['关联ID']) for row in sheets['经验关联']}
    assert links == {(item['id'], kind, target) for item in state['experiences'] for kind, key in [('目的', 'goal_ids'), ('途径', 'pathway_ids')] for target in item[key]}
    assert len(sheets['经验关联']) == sum(len(item['goal_ids']) + len(item['pathway_ids']) for item in state['experiences'])
    if 'fixture' in payload:
        fixture = payload['fixture']
        goal = next(row for row in sheets['目标'] if row['目标ID'] == fixture['goal'])
        assert goal['目标名称'] == '复盘后的当前名称'
        assert goal['版本'] == '2'
        detail = next(row for row in sheets['逐目标明细'] if row['训练ID'] == fixture['completed'] and row['目标ID'] == fixture['goal'])
        assert detail['当时目标名称'] == fixture['original_title']
        assert detail['当时达成标准'] == '原始标准\n保留中文与换行 😀'
        assert detail['当时版本'] == '1'
        assert detail['逐目标安排'] == '+原样保存'
        assert detail['实际进度'] == '-观察记录'
        assert detail['下一步'] == '@后续安排'
        completed = next(row for row in sheets['训练记录'] if row['训练ID'] == fixture['completed'])
        assert completed['训练状态'] == '已完成'
        assert completed['整次观察'] == '实际观察\n保留换行 😀'
        assert completed['训练活动'] == '游戏\n生活场景'
        draft = next(row for row in sheets['训练记录'] if row['训练ID'] == fixture['draft'])
        assert draft['训练状态'] == '已保存草稿'
        draft_detail = next(row for row in sheets['逐目标明细'] if row['训练ID'] == fixture['draft'])
        assert draft_detail['回填结论'] == ''
        assert draft_detail['实际进度'] == ''
        assert draft_detail['当时版本'] == '2'
        difficulty = next(row for row in sheets['困难'] if row['来源训练ID'] == fixture['completed'])
        assert difficulty['跟进状态'] == '已解决'
        assert difficulty['跟进结论／验证依据'] == '采用分步提示后改善'
        assert difficulty['下次跟进日期'] == '2026-10-20'
        experience = next(row for row in sheets['经验'] if row['来源训练ID'] == fixture['completed'])
        assert experience['经验类型'] == '中短期任务'
        assert experience['跟进状态'] == '已验证'
        history = [row for row in sheets['目标历史'] if row['目标ID'] == fixture['goal']]
        assert {row['目标名称'] for row in history} == {fixture['original_title'], '复盘后的当前名称'}
    all_text = json.dumps(sheets, ensure_ascii=False)
    for private in ['password_hash', 'token_hash', 'auth_sessions', 'synthetic-owner-contract-password']:
        assert private not in all_text
    print('PASS Excel: twelve sheets, complete purpose/pathway/experience relations, historical snapshots and literal text')


if __name__ == '__main__':
    verify_fixture(json.load(sys.stdin))
