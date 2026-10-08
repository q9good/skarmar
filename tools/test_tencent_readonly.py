import tempfile
import unittest
from pathlib import Path

from tencent_readonly import credential, private_output, write_private, READ_PREFIX, MUTATION_WORDS
import json
import stat


class TencentClientTests(unittest.TestCase):
    def test_dummy_credentials_have_no_added_bearer_prefix(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / 'dummy.txt'
            for value in ['dummy-value', '\ufefftoken="dummy-value"\n', "Authorization: 'dummy-value'"]:
                path.write_text(value, encoding='utf-8')
                self.assertEqual(credential(path), 'dummy-value')
            path.write_text('dummy\nsecond-line', encoding='utf-8')
            with self.assertRaises(ValueError):
                credential(path)

    def test_outputs_cannot_be_saved_in_another_checkout(self):
        with tempfile.TemporaryDirectory() as root:
            repo = Path(root) / 'other-repo'
            repo.mkdir()
            (repo / '.git').write_text('gitdir: unused')
            with self.assertRaises(ValueError):
                private_output(repo / 'snapshot.json')

    def test_mutating_names_are_not_read_candidates(self):
        for name in ['query_records', 'get_fields', 'list_sheets']:
            self.assertTrue(READ_PREFIX.search(name) and not MUTATION_WORDS.search(name))
        for name in ['update_records', 'get_and_delete_records', 'execute_query']:
            self.assertFalse(READ_PREFIX.search(name) and not MUTATION_WORDS.search(name))

    def test_private_response_write_is_complete_and_restricted(self):
        with tempfile.TemporaryDirectory() as root:
            path = private_output(Path(root) / 'snapshot.json')
            write_private(path, {'tools': []})
            self.assertEqual(json.loads(path.read_text()), {'tools': []})
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
            self.assertFalse(path.with_suffix('.json.tmp').exists())


if __name__ == '__main__':
    unittest.main()
