"""The recipients a sops file's ciphertext actually names, and how that set
differs from the audience safix declares for it.

Ported unchanged in substance from the shell-era implementation, and kept in
python for now on purpose: the planned rust rewrite of this reader is a later
change, and this file is the differential oracle it will be judged against, so
it stays until the two agree on every fixture.

Two callers read this, and a disagreement between them would be the whole
defect. `safix check` judges every governed file after the fact; `safix set`
judges the document it is about to stage, before it stages it. A second reader
with its own idea of what a stanza list means would let the command write and
commit a file the check then calls drifted.

As a command:

    sops-recipients-of <sops-file> <declared-recipients-json>

prints {"actual": [...], "extra": [...], "missing": [...]} to stdout. Every
string on every path through this is an age public key, which is public data:
no plaintext, no private key and no decryption is involved in answering the
question, because a file's stanzas name its recipients in the clear.
"""

import json
import sys

import yaml

NO_METADATA = "<file carries no sops age metadata>"


def recipients_of(text):
    """The age recipients a file's ciphertext actually names.

    A file with no `sops:` block reports the sentinel rather than raising: a
    governed path holding one is either plaintext someone committed by mistake
    or ciphertext from a store with a different metadata shape, and both
    must be reported against the audience rather than crashing the reader
    that was asked to inspect them.
    """
    document = yaml.safe_load(text)
    metadata = document.get("sops") if isinstance(document, dict) else None
    stanzas = metadata.get("age") if isinstance(metadata, dict) else None
    if not isinstance(stanzas, list):
        return [NO_METADATA]
    return sorted(stanza["recipient"] for stanza in stanzas)


def drift(actual, declared):
    """Which recipients each side holds that the other does not.

    `extra` can open the file and is not in its audience; `missing` is in the
    audience and cannot open the file.
    """
    return {
        "extra": sorted(set(actual) - set(declared)),
        "missing": sorted(set(declared) - set(actual)),
    }


def main(argv):
    if len(argv) != 3:
        print(
            "usage: sops-recipients-of <sops-file> <declared-recipients-json>",
            file=sys.stderr,
        )
        return 2
    with open(argv[1], encoding="utf-8") as handle:
        actual = recipients_of(handle.read())
    with open(argv[2], encoding="utf-8") as handle:
        declared = json.load(handle)
    result = drift(actual, declared)
    result["actual"] = actual
    json.dump(result, sys.stdout)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
