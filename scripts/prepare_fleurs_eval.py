#!/usr/bin/env python3
"""Extract a small, repeatable EN/UK FLEURS test set from local archives.

Download each language's test.tsv and audio/test.tar.gz from
https://huggingface.co/datasets/google/fleurs into
asr/.local/hard_eval/fleurs-parakeet as {en_us,uk_ua}-test.{tsv,tar.gz}.
"""

import argparse
import json
import tarfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DATA_DIR = ROOT / "asr" / ".local" / "hard_eval" / "fleurs-parakeet"
LANGUAGES = {"en_us": "english", "uk_ua": "ukrainian"}


def prepare_language(code: str, language: str, count: int) -> list[dict]:
    tsv_path = DATA_DIR / f"{code}-test.tsv"
    archive_path = DATA_DIR / f"{code}-test.tar.gz"
    rows = [line.split("\t") for line in tsv_path.read_text(encoding="utf-8").splitlines()]
    if len(rows) < count:
        raise ValueError(f"{tsv_path} has only {len(rows)} rows")

    # Spread examples across the test set and keep the selection stable.
    indexes = [i * (len(rows) - 1) // count for i in range(count)]
    selected = {rows[index][1]: rows[index] for index in indexes}
    output_dir = DATA_DIR / code
    output_dir.mkdir(parents=True, exist_ok=True)
    found = set()
    with tarfile.open(archive_path, "r:gz") as archive:
        for member in archive:
            filename = Path(member.name).name
            if filename not in selected or not member.isfile():
                continue
            source = archive.extractfile(member)
            if source is None:
                continue
            target = output_dir / filename
            with target.open("wb") as output:
                output.write(source.read())
            found.add(filename)
    if found != selected.keys():
        raise ValueError(f"Missing audio in {archive_path}: {sorted(selected.keys() - found)}")

    return [
        {
            "language": language,
            "id": filename.removesuffix(".wav"),
            "path": str(output_dir / filename),
            "reference": row[2],
            "duration_seconds": int(row[5]) / 16_000,
            "source": "google/fleurs test",
        }
        for filename, row in selected.items()
    ]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--count-per-language", type=int, default=5)
    args = parser.parse_args()
    if args.count_per_language < 1:
        parser.error("--count-per-language must be positive")
    manifest = DATA_DIR / "manifest.jsonl"
    samples = [
        sample
        for code, language in LANGUAGES.items()
        for sample in prepare_language(code, language, args.count_per_language)
    ]
    manifest.write_text(
        "".join(json.dumps(sample, ensure_ascii=False) + "\n" for sample in samples),
        encoding="utf-8",
    )
    print(f"Wrote {len(samples)} samples to {manifest}")


if __name__ == "__main__":
    main()
