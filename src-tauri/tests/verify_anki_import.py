r"""Verify the RustLens TSV with Anki in a disposable collection.

Requires Anki's Python library. A packaged Windows Anki install may be supplied
with --anki-packages C:\Apps\Anki\app_packages, using a matching Python version.
The user's Anki profile is never opened.
"""
import argparse
import os
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
parser = argparse.ArgumentParser()
parser.add_argument("fixture", type=Path)
parser.add_argument("--anki-packages", type=Path)
args = parser.parse_args()
handles = []
if args.anki_packages:
    packages = args.anki_packages.resolve()
    sys.path.insert(0, str(packages))
    if os.name == "nt":
        for directory in [packages.parent, packages / "anki"]:
            handles.append(os.add_dll_directory(str(directory)))

from anki.collection import Collection
from anki.import_export_pb2 import ImportCsvRequest

with tempfile.TemporaryDirectory(prefix="rustlens-anki-import-") as directory:
    col = Collection(str(Path(directory) / "collection.anki2"), server=True)
    try:
        fixture = str(args.fixture.resolve())
        metadata = col.get_csv_metadata(fixture, delimiter=None)
        assert metadata.is_html
        assert list(metadata.column_labels) == ["Front", "Back", "Tags"]
        col.import_csv(ImportCsvRequest(path=fixture, metadata=metadata))
        assert col.note_count() == 2 and col.card_count() == 2
        notes = col.find_notes("tag:rustlens")
        assert len(notes) == 2
        fields = [col.get_note(note_id).fields for note_id in notes]
        assert any(row[0] == "Keep an eye on the gate." and "Стежте" in row[1] and "<br>" in row[1] for row in fields)
        assert any("&lt;player&gt;" in row[0] and "привіт" in row[1] for row in fields)
        cards_before = col.db.list("select id from cards order by id")
        col.import_csv(ImportCsvRequest(path=fixture, metadata=metadata))
        assert col.note_count() == 2 and col.card_count() == 2
        assert cards_before == col.db.list("select id from cards order by id")
        print("PASS: two notes/cards, Ukrainian/HTML/tags preserved; repeat import has no duplicates")
    finally:
        col.close()
