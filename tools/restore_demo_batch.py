"""Append a reviewed synthetic batch through the ordinary authenticated application API.

No DB restore, SQL, account creation, deletes or production credentials in files/logs.
Base phase works on the previous application; links phase requires T36 after release.
"""
import argparse
from http.cookiejar import CookieJar
import json
import os
from pathlib import Path
import sys
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import Request, build_opener, HTTPCookieProcessor, HTTPRedirectHandler, ProxyHandler

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'tools/fixtures/demo-20261023.json'
CONFIRM = 'skarma-demo-20261023'


class RestoreError(Exception):
    """Only deliberately non-sensitive diagnostics may be printed."""


def require(condition, message):
    if not condition:
        raise RestoreError(message)


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise RestoreError('Redirect refused; use the exact application origin.')


class Client:
    def __init__(self, origin, allow_local=False):
        target = urlsplit(origin)
        local = target.hostname == '127.0.0.1' and target.scheme == 'http' and bool(target.port)
        require(origin == 'https://skarma.vip' or (allow_local and local and not target.path and not target.username and not target.password),
                'Only https://skarma.vip or an explicitly allowed disposable localhost is supported.')
        self.local = local
        self.origin = origin
        handlers = [HTTPCookieProcessor(CookieJar()), NoRedirect()]
        if local:
            handlers.append(ProxyHandler({}))
        self.opener = build_opener(*handlers)

    def call(self, method, path, body=None):
        require(path.startswith('/api/') and '?' not in path and '#' not in path, 'Unexpected API path.')
        headers = {'Origin': self.origin, 'X-Skarma-Request': '1', 'User-Agent': 'SkarmaDemoRestore/1.0', 'Accept': 'application/json'}
        data = None
        if body is not None:
            headers['Content-Type'] = 'application/json'
            data = json.dumps(body, ensure_ascii=False).encode()
        try:
            with self.opener.open(Request(self.origin + path, data=data, headers=headers, method=method), timeout=45) as response:
                require(response.status == 200, f'{method} {path}: expected HTTP 200.')
                require('application/json' in response.headers.get('Content-Type', ''), 'API returned a non-JSON response.')
                raw = response.read(8 * 1024 * 1024 + 1)
                require(len(raw) <= 8 * 1024 * 1024, 'API response exceeded the restore reader limit.')
                return json.loads(raw)
        except HTTPError as exc:
            # The body, request and account identity are intentionally never printed.
            raise RestoreError(f'{method} {path}: HTTP {exc.code}; no response body logged.') from None
        except (URLError, TimeoutError, OSError, ValueError):
            raise RestoreError(f'{method} {path}: network or JSON failure; no response body logged.') from None

    def login(self):
        try:
            credentials = json.loads(os.environ.get('SKARMA_TEST_LOGIN', ''))
        except ValueError:
            raise RestoreError('Missing or invalid SKARMA_TEST_LOGIN; configure an existing approved application login in GitHub Actions Secrets.') from None
        require(isinstance(credentials, dict) and set(credentials) == {'username', 'password'} and
                all(isinstance(v, str) and v.strip() for v in credentials.values()), 'SKARMA_TEST_LOGIN must contain username and password only.')
        self.call('POST', '/api/auth/login', credentials)
        identity = self.call('GET', '/api/auth/me')
        require(identity.get('user', {}).get('role') in ('admin', 'member'), 'An existing approved account is required; no accounts will be created.')
        require(self.local or identity.get('space') == 'default', 'Refusing to append outside the existing default space.')


def equal_fields(actual, expected):
    optional_empty = {'difficulty_goal_ids', 'difficulty_pathway_ids', 'experience_goal_ids', 'experience_pathway_ids'}
    return all(actual.get(key, [] if key in optional_empty else None) == value for key, value in expected.items())


def lookup(state, kind, item_id):
    return next((v for v in state[kind] if v['id'] == item_id), None)


def source_item(state, kind, source, item_id):
    matches = [v for v in state[kind] if v.get(source) == item_id]
    require(len(matches) <= 1, 'Duplicate source followup; investigate before retrying.')
    return matches[0] if matches else None


def validate_state(state, batch):
    """Fail before writes if any batch ID was deleted or any restored row was edited."""
    managed = {kind: {v['id'] for v in batch[kind]} for kind in ['goals', 'pathways', 'experiences', 'sessions', 'lessons']}
    require(not any(row['id'] in managed.get(row['kind'], set()) for row in state.get('deleted', [])),
            'A batch record was deleted by an administrator; automatic resurrection is refused.')
    for kind in ['goals', 'pathways', 'experiences']:
        for expected in batch[kind]:
            old = lookup(state, kind, expected['id'])
            require(old is None or (old['version'] == 1 and equal_fields(old, expected)),
                    f'An existing batch {kind} record differs; leave it intact and choose a separately reviewed batch.')
    for expected in batch['sessions']:
        old = lookup(state, 'sessions', expected['id'])
        if old:
            require(old['version'] == 1 and old['status'] == expected['status'] and equal_fields(old, expected['input']),
                    'A batch training record was edited; no overwrite will be attempted.')
            for kind, flag, content in [('difficulties', 'has_difficulty', 'difficulty'), ('experiences', 'has_experience', 'experience')]:
                item = source_item(state, kind, 'source_session_id', old['id'])
                if expected['input'][flag]:
                    require(item is not None and item['version'] == 1 and item['status'] == 'pending' and
                            item['content'] == expected['input'][content] and
                            item['goal_ids'] == expected['input'][content + '_goal_ids'] and
                            item['pathway_ids'] == expected['input'][content + '_pathway_ids'],
                            'A training followup was edited or deleted; automatic restoration is refused.')
    for expected in batch['lessons']:
        old = lookup(state, 'lessons', expected['id'])
        if old:
            require(old['version'] in (1, 2) and equal_fields(old, expected['input']),
                    'A batch course record was edited; no overwrite will be attempted.')
            links = old.get('links')
            require((old['version'] == 1 and links is None) or (old['version'] == 2 and links == expected['links']),
                    'A batch course has unexpected links or a changed version.')
            item = source_item(state, 'difficulties', 'source_lesson_id', old['id'])
            if links and links['track_problem']:
                require(item is not None and item['version'] == 1 and item['status'] == 'pending' and
                        item['content'] == expected['input']['problems'] and item['goal_ids'] == links['problem_goal_ids'] and
                        item['pathway_ids'] == links['problem_pathway_ids'], 'A course followup was edited or deleted; it will not be recreated.')
    names = {('course', row['input']['course']) for row in batch['lessons']} | {('teacher', row['input']['teacher']) for row in batch['lessons']}
    require(not any((row['kind'], row['value']) in names and (not row['active'] or row.get('renamed_to')) for row in state.get('lesson_options', [])),
            'A batch catalog option was stopped or renamed; it will not be reactivated.')


def apply_batch(api, batch, phase):
    before = api('GET', '/api/state')
    validate_state(before, batch)
    if phase == 'links':
        require(all(lookup(before, kind, row['id']) is not None for kind in ['goals', 'pathways', 'experiences', 'sessions', 'lessons'] for row in batch[kind]),
                'Base records are incomplete; finish the base phase first.')
    added = {kind: 0 for kind in ['goals', 'pathways', 'experiences', 'sessions', 'lessons']}
    updated = 0
    if phase == 'base':
        for kind in ['goals', 'pathways', 'experiences']:
            for row in batch[kind]:
                if not lookup(before, kind, row['id']):
                    api('POST', '/api/' + kind, row)
                    added[kind] += 1
        for row in batch['sessions']:
            if not lookup(before, 'sessions', row['id']):
                api('PUT', '/api/sessions/' + row['id'], {'operation_id': row['operation_id'], 'action': 'complete' if row['status'] == 'completed' else 'save_draft', 'expected_version': None, 'session': row['input']})
                added['sessions'] += 1
        for row in batch['lessons']:
            if not lookup(before, 'lessons', row['id']):
                api('PUT', '/api/lessons/' + row['id'], {'expected_version': None, 'lesson': row['input']})
                added['lessons'] += 1
    else:
        for row in batch['lessons']:
            old = lookup(before, 'lessons', row['id'])
            if old.get('links') != row['links']:
                saved = api('PUT', '/api/lessons/' + row['id'], {'expected_version': old['version'], 'difficulty_version': None, 'lesson': {**row['input'], 'links': row['links']}})
                require(saved.get('links') == row['links'], 'The live application does not support course links yet; stop and verify the deployment.')
                updated += 1
    after = api('GET', '/api/state')
    validate_state(after, batch)
    require(all(lookup(after, kind, row['id']) is not None for kind in ['goals', 'pathways', 'experiences', 'sessions', 'lessons'] for row in batch[kind]), 'Some base records are missing; retry with the same batch.')
    if phase == 'links':
        require(all(lookup(after, 'lessons', row['id']).get('links') == row['links'] for row in batch['lessons']), 'Course links were not confirmed.')
    counts = {kind: sum(lookup(after, kind, row['id']) is not None for row in batch[kind]) for kind in ['goals', 'pathways', 'experiences', 'sessions', 'lessons']}
    counts['difficulties'] = sum(source_item(after, 'difficulties', source, row['id']) is not None for source, records in [('source_session_id', batch['sessions']), ('source_lesson_id', batch['lessons'])] for row in records)
    counts['experiences'] += sum(source_item(after, 'experiences', 'source_session_id', row['id']) is not None for row in batch['sessions'])
    return {'batch': batch['batch'], 'phase': phase, 'state': 'base_restored_pending_release' if phase == 'base' else 'restored_and_linked', 'added': added, 'updated_lessons': updated, 'batch_record_counts': counts}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--origin', default='https://skarma.vip')
    parser.add_argument('--phase', choices=['base', 'links'], required=True)
    parser.add_argument('--confirm-append', required=True)
    parser.add_argument('--allow-local', action='store_true')
    parser.add_argument('--receipt-dir', type=Path, default=ROOT / '.local/demo-restore')
    args = parser.parse_args()
    receipt = {'batch': CONFIRM, 'phase': args.phase, 'state': 'not_started'}
    client = None
    signed_in = False
    try:
        require(args.confirm_append == CONFIRM, 'Explicit confirmation must match the reviewed batch.')
        batch = json.loads(FIXTURE.read_text())
        require(batch['batch'] == CONFIRM and batch['synthetic'] is True, 'Unexpected fixture.')
        client = Client(args.origin, args.allow_local)
        client.login()
        signed_in = True
        receipt = apply_batch(client.call, batch, args.phase)
    except RestoreError as exc:
        receipt.update(state='stopped_check_batch_before_retry', error=str(exc))
    finally:
        if signed_in:
            try:
                client.call('POST', '/api/auth/logout')
            except RestoreError:
                receipt['logout_confirmed'] = False
            else:
                receipt['logout_confirmed'] = True
        args.receipt_dir.mkdir(parents=True, exist_ok=True)
        (args.receipt_dir / f'{args.phase}.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(receipt, ensure_ascii=False))
    return 0 if receipt['state'] in ('base_restored_pending_release', 'restored_and_linked') else 1


if __name__ == '__main__':
    sys.exit(main())
