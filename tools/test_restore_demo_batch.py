"""Synthetic guard tests. Never connect to production or use real credentials."""
from copy import deepcopy
import json
import unittest
from unittest.mock import patch
import restore_demo_batch as restore

BATCH = json.loads(restore.FIXTURE.read_text())


def base_state():
    state = {key: [] for key in ['goals', 'pathways', 'experiences', 'sessions', 'lessons', 'difficulties', 'deleted', 'lesson_options']}
    for kind in ['goals', 'pathways', 'experiences']:
        for row in BATCH[kind]:
            state[kind].append({**deepcopy(row), 'version': 1})
    for row in BATCH['sessions']:
        state['sessions'].append({**deepcopy(row['input']), 'id': row['id'], 'version': 1, 'status': row['status']})
        if row['status'] == 'completed':
            for kind, field in [('difficulties', 'difficulty'), ('experiences', 'experience')]:
                state[kind].append({'id': field, 'version': 1, 'status': 'pending', 'source_session_id': row['id'], 'content': row['input'][field], 'goal_ids': row['input'][field+'_goal_ids'], 'pathway_ids': row['input'][field+'_pathway_ids']})
    for row in BATCH['lessons']:
        state['lessons'].append({**deepcopy(row['input']), 'id': row['id'], 'version': 1})
    return state


class Guards(unittest.TestCase):
    def test_batch_is_fixed_and_all_new_business_content_marked_synthetic(self):
        self.assertEqual(BATCH['batch'], restore.CONFIRM)
        self.assertTrue(BATCH['synthetic'])
        self.assertEqual([len(BATCH[k]) for k in ['goals','pathways','sessions','lessons','experiences']], [6,1,2,2,1])
        self.assertTrue(all(row['title'].startswith('【演示测试1023】') for row in BATCH['goals'] + BATCH['pathways']))

    def test_only_fixed_production_or_explicit_local_target(self):
        for origin in ['http://skarma.vip', 'https://evil.test', 'https://skarma.vip/other', 'https://user:pass@skarma.vip', 'http://localhost:3001']:
            with self.subTest(origin=origin), self.assertRaises(restore.RestoreError):
                restore.Client(origin, allow_local=True)
        restore.Client('https://skarma.vip')
        restore.Client('http://127.0.0.1:31337', allow_local=True)
        with self.assertRaises(restore.RestoreError):
            restore.Client('http://127.0.0.1:31337')

    def test_approved_account_and_default_space_required_before_reading_business(self):
        for identity in [{'user':{'role':'guest'},'space':'default'}, {'user':{'role':'admin'},'space':'different'}]:
            with self.subTest(identity=identity), patch.dict('os.environ', {'SKARMA_TEST_LOGIN': '{"username":"synthetic","password":"synthetic"}'}):
                client=restore.Client('https://skarma.vip')
                with patch.object(client,'call',side_effect=[{},identity]) as call:
                    with self.assertRaises(restore.RestoreError): client.login()
                    self.assertEqual(call.call_count,2)

    def test_existing_member_is_sufficient_for_append_without_admin_credentials(self):
        with patch.dict('os.environ', {'SKARMA_TEST_LOGIN': '{"username":"synthetic-member","password":"synthetic"}'}):
            client=restore.Client('https://skarma.vip')
            with patch.object(client,'call',side_effect=[{},{'user':{'role':'member'},'space':'default'}]):
                client.login()

    def test_existing_batches_are_read_only_on_repeated_base_restore(self):
        state=base_state();calls=[]
        def api(method,path,body=None):
            calls.append(method)
            self.assertEqual((method,path),('GET','/api/state'))
            return deepcopy(state)
        result=restore.apply_batch(api,BATCH,'base')
        self.assertEqual(result['added'],dict.fromkeys(['goals','pathways','experiences','sessions','lessons'],0))
        self.assertEqual(calls,['GET','GET'])

    def test_changed_or_deleted_rows_and_followups_stop_before_any_write(self):
        for change in [lambda s:s['goals'][0].update(version=2),
                       lambda s:s['pathways'][0].update(title='user edited'),
                       lambda s:s['sessions'][0].update(version=2),
                       lambda s:s['lessons'][0].update(version=3),
                       lambda s:s['difficulties'][0].update(content='user curated'),
                       lambda s:s['experiences'].pop(),
                       lambda s:s['deleted'].append({'kind':'goals','id':BATCH['goals'][0]['id']}),
                       lambda s:s['lesson_options'].append({'kind':'teacher','value':BATCH['lessons'][0]['input']['teacher'],'active':False})]:
            state=base_state();change(state);calls=[]
            def api(method,path,body=None):
                calls.append(method);return deepcopy(state)
            with self.subTest(change=change),self.assertRaises(restore.RestoreError):restore.apply_batch(api,BATCH,'base')
            self.assertEqual(calls,['GET'])

    def test_serialized_empty_session_bindings_default_to_empty_lists(self):
        state=base_state()
        for key in ['difficulty_goal_ids','difficulty_pathway_ids','experience_goal_ids','experience_pathway_ids']:
            del state['sessions'][1][key]
        restore.validate_state(state,BATCH)

    def test_incomplete_base_refuses_link_phase(self):
        state=base_state();state['goals'].pop()
        with self.assertRaises(restore.RestoreError): restore.apply_batch(lambda *args:state,BATCH,'links')

    def test_links_are_applied_only_to_existing_fixed_batch_courses(self):
        state=base_state();writes=[]
        def api(method,path,body=None):
            if method=='GET':return deepcopy(state)
            writes.append((method,path))
            self.assertEqual(method,'PUT')
            row=next(v for v in state['lessons'] if path=='/api/lessons/'+v['id'])
            self.assertEqual(body['expected_version'],1)
            row.update(deepcopy(body['lesson']),version=2)
            links=row['links']
            if links['track_problem']:
                state['difficulties'].append({'id':'lesson-difficulty','source_lesson_id':row['id'],'version':1,'status':'pending','content':row['problems'],'goal_ids':links['problem_goal_ids'],'pathway_ids':links['problem_pathway_ids']})
            return deepcopy(row)
        result=restore.apply_batch(api,BATCH,'links')
        self.assertEqual(len(writes),2)
        self.assertEqual(sum(result['batch_record_counts'].values()),15)
        self.assertEqual(restore.apply_batch(api,BATCH,'links')['updated_lessons'],0)
        self.assertEqual(len(writes),2)


if __name__=='__main__':unittest.main()
