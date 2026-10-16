"""DOC-06: docs/GENOME.md must describe the genome the code implements.

Rows whose Pending cell names a workplan item may describe a field or operator another lane
is removing; anything else that drifts fails with the exact name to fix.
"""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
GENOME = (ROOT / 'docs/GENOME.md').read_text(encoding='utf-8')
RUST = '\n'.join(p.read_text(encoding='utf-8') for p in sorted((ROOT / 'crates').rglob('*.rs')))
ITEM = re.compile(r'\b[A-Z]{2,3}-\d{2}\b')


def struct_fields(name):
    m = re.search(r'pub struct ' + name + r' \{(.*?)\n\}', RUST, re.S)
    if not m:
        return None
    return re.findall(r'^\s*pub (\w+): ([^,\n]+),', m.group(1), re.M)


def section(heading):
    start = GENOME.index(heading)
    nxt = GENOME.find('\n## ', start + len(heading))
    return GENOME[start: nxt if nxt >= 0 else len(GENOME)]


def table_rows(text):
    """(first-cell code span, last cell) for each data row of the markdown tables in `text`."""
    rows = []
    for line in text.splitlines():
        cells = [c.strip() for c in re.split(r'(?<!\\)\|', line.strip())[1:-1]]
        m = re.fullmatch(r'`([^`]+)`', cells[0]) if cells else None
        if m:
            rows.append((m.group(1), cells[-1]))
    return rows


def code_genome_paths():
    paths = set()
    for field, ty in struct_fields('DesignProgram'):
        inner = re.fullmatch(r'Vec<(\w+)>', ty.strip())
        sub = struct_fields(inner.group(1) if inner else ty.strip())
        if sub is None:
            paths.add(field)
        else:
            prefix = f'{field}[].' if inner else f'{field}.'
            paths.update(prefix + f for f, _ in sub)
    return paths


class GenomeDocTests(unittest.TestCase):
    def assert_matches(self, documented, in_code, what):
        missing_doc = sorted(set(in_code) - {name for name, _ in documented})
        self.assertEqual(missing_doc, [], f'{what} in code but not in docs/GENOME.md')
        stale = sorted(name for name, pending in documented
                       if name not in in_code and not ITEM.search(pending))
        self.assertEqual(stale, [], f'{what} documented in docs/GENOME.md but absent from code '
                                    '(tag the Pending cell with the workplan item removing it)')

    def test_every_serialized_field_is_documented(self):
        self.assertIsNotNone(struct_fields('DesignProgram'))
        paths = code_genome_paths()
        self.assertGreaterEqual(len(paths), 30)
        self.assert_matches(table_rows(section('## 2. Field reference')), paths, 'genome fields')

    def test_family_labels_match_code(self):
        rows = table_rows(section('## 4. Derived label'))
        m = re.search(r'CANONICAL_11_FAMILIES: \[&\'static str; \d+\] = \[(.*?)\];', RUST, re.S)
        if m is None:
            self.assertIn('IR-06', section('## 4. Derived label'))
            return
        families = re.findall(r'"([^"]+)"', m.group(1))
        self.assertEqual(sorted(name for name, _ in rows), sorted(families))

    def test_descriptor_names_match_code(self):
        rows = table_rows(section('## 5. Gene-derived'))
        fields = struct_fields('MechanismDescriptors') or struct_fields('PriorDescriptors')
        if fields is None:
            self.assertIn('IR-07', section('## 5. Gene-derived'))
            return
        self.assertEqual(sorted(name for name, _ in rows), sorted(f for f, _ in fields))

    def test_documented_operators_exist(self):
        rows = table_rows(section('## 6. Variation operators'))
        self.assertGreaterEqual(len(rows), 10)
        stale = [name for name, pending in rows
                 if not re.search(r'\bfn ' + name + r'\b', RUST) and not ITEM.search(pending)]
        self.assertEqual(stale, [])

    def test_proposed_v4_design_is_labelled_proposed(self):
        v4 = (ROOT / 'docs/design/GENOME_V4.md').read_text(encoding='utf-8')
        self.assertIn('PROPOSED', v4.splitlines()[0])
        self.assertIn('design/GENOME_V4.md', GENOME)


if __name__ == '__main__':
    unittest.main()
