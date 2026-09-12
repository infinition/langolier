import json
import tempfile
import unittest
from pathlib import Path
from prepare_dataset import prepare


class DatasetTests(unittest.TestCase):
    def test_groups_do_not_leak_and_duplicate_prompts_are_removed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rows = []
            for i in range(20):
                for j in range(2):
                    rows.append({"conversation_id": str(i), "messages": [{"role": "user", "content": f"Question {i}/{j}"}, {"role": "assistant", "content": "Reviewed answer"}]})
            rows.append(rows[0])
            source = root / "input.jsonl"
            source.write_text("\n".join(json.dumps(r) for r in rows))
            result = prepare(source, root / "data")
            self.assertEqual(sum(result["examples"].values()), 40)
            groups = []
            for split in ["train", "valid", "test"]:
                records = [json.loads(line) for line in (root / "data" / f"{split}.jsonl").read_text().splitlines()]
                groups.append({r["messages"][0]["content"].split('/')[0] for r in records})
            self.assertFalse(groups[0] & groups[1] or groups[1] & groups[2] or groups[0] & groups[2])
            self.assertEqual(prepare(source, root / "repeat"), result)


if __name__ == '__main__':
    unittest.main()
