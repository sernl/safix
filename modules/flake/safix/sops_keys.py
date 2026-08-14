"""Which keys a sops file holds, and which of them hold nothing, without
decrypting it.

Ported unchanged in substance from the shell-era implementation, and kept in
python for now on purpose: the planned rust rewrite of this reader is a later
change, and this file is the differential oracle it will be judged against, so
it stays until the two agree on every fixture.

`safix check` and `safix generate` both have to answer "does this name have a
value yet" for names whose files they may hold no identity for, and the answer
decides whether to offer to mint one. Decrypting to find out makes the question
unanswerable on any machine that is not the owner's, and would put plaintext on
a read path that has no business producing any.

It is answerable without decrypting because sops leaves the document's shape in
the clear: only the leaf values are enciphered, so the key names, and the fact
that a key's ciphertext encrypts the empty string, are both readable from the
bytes.

As a command:

    sops-keys-of <sops-file>

prints {"<key>": {"empty": <bool>}} for every top-level key except sops' own
metadata block. Nothing on any path through this is plaintext.
"""

import json
import re
import sys

import yaml

# `ENC[AES256_GCM,data:<base64>,iv:...,tag:...,type:str]` is the envelope sops
# writes around a leaf value. An empty `data:` segment is the encryption of the
# empty string: AES-GCM is a stream cipher construction, so ciphertext length
# equals plaintext length and zero bytes in means zero bytes out.
#
# The file `safix set` creates through sops for a name with no value yet holds
# exactly that, so the distinction between a key that was never written and a
# key holding the empty string is the difference between "no value" and "no
# value, and a file already exists to put one in".
EMPTY_CIPHERTEXT = re.compile(r"^ENC\[[A-Z0-9_]+,data:,")


def keys_of(text):
    """Top-level data keys of a sops document, each flagged empty or not.

    A document that is not a mapping has no keys to report and yields an empty
    result rather than raising: a governed path holding one is either plaintext
    someone committed by mistake or ciphertext from a store with a different
    shape, and the caller's business is the keys, not the diagnosis.
    """
    document = yaml.safe_load(text)
    if not isinstance(document, dict):
        return {}

    def empty(value):
        if not isinstance(value, str):
            return False
        return EMPTY_CIPHERTEXT.match(value) is not None

    return {
        key: {"empty": empty(value)}
        for key, value in document.items()
        if key != "sops"
    }


def main(argv):
    if len(argv) != 2:
        print("usage: sops-keys-of <sops-file>", file=sys.stderr)
        return 2
    with open(argv[1], encoding="utf-8") as handle:
        json.dump(keys_of(handle.read()), sys.stdout)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
