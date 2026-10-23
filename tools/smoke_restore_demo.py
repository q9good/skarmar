"""Run on disposable authenticated native/Worker spaces, never on production."""
import json
import os
from pathlib import Path
import uuid
from restore_demo_batch import Client, FIXTURE, RestoreError, apply_batch

BASE = os.environ['SKARMA_TEST_URL']
OUT = Path(os.environ['SKARMA_RELATION_OUTPUT'])
OUT.mkdir(parents=True, exist_ok=True)
batch = json.loads(FIXTURE.read_text())
client = Client(BASE, allow_local=True)
client.login()
try:
    # An unrelated record proves this is an append rather than a whole-space restore.
    control = client.call('POST', '/api/goals', {'id': str(uuid.uuid4()), 'title': '合成隔离测试：保留既有目标', 'level': 'long', 'parent_id': None, 'area': '', 'criteria': ''})
    baseline = client.call('GET', '/api/state')
    base = apply_batch(client.call, batch, 'base')
    after_base = client.call('GET', '/api/state')
    assert sum(base['batch_record_counts'].values()) == 14
    assert all('links' not in row for row in after_base['lessons'])
    replay = apply_batch(client.call, batch, 'base')
    assert not any(replay['added'].values())
    assert client.call('GET', '/api/state') == after_base
    complete = apply_batch(client.call, batch, 'links')
    assert sum(complete['batch_record_counts'].values()) == 15
    final = client.call('GET', '/api/state')
    assert next(v for v in final['goals'] if v['id'] == control['id']) == control
    assert apply_batch(client.call, batch, 'links')['updated_lessons'] == 0
    assert client.call('GET', '/api/state') == final
    # An independently edited test row must make the batch refuse all writes.
    goal = next(v for v in final['goals'] if v['id'] == batch['goals'][0]['id'])
    client.call('PUT', '/api/goals/' + goal['id'], {**goal, 'title': goal['title'] + ' 已独立维护', 'expected_version': goal['version'], 'change_note': '合成保护检查'})
    curated = client.call('GET', '/api/state')
    try:
        apply_batch(client.call, batch, 'base')
    except RestoreError:
        pass
    else:
        raise AssertionError('Edited data should have blocked restoration')
    assert client.call('GET', '/api/state') == curated
    (OUT / 'result.json').write_text(json.dumps({'synthetic': True, 'snapshot': curated, 'base': base, 'links': complete, 'checks': ['authenticated existing admin', 'preserves unrelated records', 'base compatible free-text lessons', '15 linked synthetic records', 'idempotent phases', 'refuses overwrite after independent edits']}, ensure_ascii=False, indent=2))
finally:
    client.call('POST', '/api/auth/logout')
print('PASS synthetic batch append: authenticated API, base then course links, idempotence, unrelated record preservation and edited-row refusal')
