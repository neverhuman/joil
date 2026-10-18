"""Independent software metadata and stale-source controls; no physical claims."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from tools import v5_explorer_manifest as explorer


class ExplorerManifestTests(unittest.TestCase):
    def fixture(self):
        return {'progress': {'V5-fixture': {'tasks': {
            'a': {'status': 'done'}, 'b': {'status': 'blocked'}, 'c': {'status': 'review'}}}},
            'scientific_status': {'phase': 'unqualified_fixture', 'campaign_admission': 'sandbox_only',
                                  'campaign_blockers': ['missing measurements']}}

    def test_exact_source_counts_do_not_invent_campaigns_or_observations(self):
        manifest, raw = explorer.view(self.fixture())
        self.assertEqual(manifest['campaign']['tasks_completed'], 1)
        self.assertEqual(manifest['campaign']['tasks_total'], 3)
        for field in ('attempts', 'spend', 'reservations', 'checkpoint'):
            self.assertIsNone(manifest['campaign'][field])
        for field in ('subjects', 'values', 'archive'):
            self.assertEqual(manifest[field], [])
        self.assertEqual(manifest['revision'], hashlib.sha256(raw).hexdigest())
        self.assertIn('unqualified_fixture', manifest['campaign']['scientific_release'])

    def test_changed_state_changes_revision_and_stale_output_is_rejected(self):
        source = self.fixture()
        first, _ = explorer.view(source)
        source['progress']['V5-fixture']['tasks']['b']['status'] = 'done'
        second, _ = explorer.view(source)
        self.assertNotEqual(first['revision'], second['revision'])
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            files = {Path('manifest.json'): explorer.encoded(first)}
            explorer.apply(files, root)
            explorer.apply(files, root, check=True)
            (root/'manifest.json').write_text('{}')
            with self.assertRaisesRegex(ValueError, 'stale explorer'):
                explorer.apply(files, root, check=True)

    def test_committed_view_matches_canonical_records(self):
        explorer.apply(explorer.generated(), check=True)


if __name__ == '__main__':
    unittest.main()
