#!/usr/bin/env python3
"""Validate reviewed chat exports and split by conversation without overlap."""
import argparse
import hashlib
import json
import random
from pathlib import Path


def prepare(source: Path, destination: Path, seed: int = 42):
    groups = {}
    seen = set()
    for line_no, line in enumerate(source.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        row = json.loads(line)
        messages = row.get("messages", [])
        if not messages or messages[-1].get("role") != "assistant":
            raise ValueError(f"Line {line_no}: expected a conversation ending with an assistant response")
        if any(m.get("role") not in {"system", "user", "assistant"} or not isinstance(m.get("content"), str) or not m["content"].strip() for m in messages):
            raise ValueError(f"Line {line_no}: invalid or empty message")
        prompt = " ".join(m["content"] for m in messages if m["role"] == "user")
        fingerprint = hashlib.sha256(" ".join(prompt.casefold().split()).encode()).hexdigest()
        if fingerprint in seen:
            continue
        seen.add(fingerprint)
        group = row.get("conversation_id", fingerprint)
        groups.setdefault(group, []).append({"messages": messages})
    if len(groups) < 10:
        raise ValueError("At least 10 distinct reviewed conversations are required for a meaningful three-way split")
    keys = sorted(groups)
    random.Random(seed).shuffle(keys)
    held_out = max(1, round(len(keys) * 0.1))
    splits = {"test": keys[:held_out], "valid": keys[held_out:2 * held_out], "train": keys[2 * held_out:]}
    destination.mkdir(parents=True, exist_ok=True)
    counts = {}
    for split, selected in splits.items():
        records = [record for group in selected for record in groups[group]]
        (destination / f"{split}.jsonl").write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in records), encoding="utf-8")
        counts[split] = len(records)
    manifest = {"seed": seed, "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(), "examples": counts, "groups": {k: len(v) for k, v in splits.items()}, "policy": "Conversation-disjoint split; normalized exact prompt deduplication before splitting. Review near-duplicates and source leakage separately."}
    (destination / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()
    print(json.dumps(prepare(args.source, args.destination, args.seed), indent=2))
