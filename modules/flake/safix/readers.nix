# The two ciphertext readers, built once and used twice: the command's closure
# needs them, and the checks that drive the command need the same binaries.
#
# A plain function rather than an option, because there is nothing here for a
# consumer to configure — the alternative would be an option whose only correct
# value is the one this file computes.
#
# The real readers in both places, never a stub. The claim a check makes is that
# a drifted file is refused, and a stub answering "which recipients does this
# file name" is exactly what would let that claim hold over a command that could
# no longer tell.
{ pkgs }:
{
  # Which recipients a document's stanzas name, and how that set differs from a
  # declared audience. One implementation, so the refusal before a write and the
  # check after one cannot reach different answers about the same bytes.
  sops-recipients-of = pkgs.writers.writePython3Bin "sops-recipients-of" {
    libraries = [ pkgs.python3Packages.pyyaml ];
  } (builtins.readFile ./sops_recipients.py);

  # Which keys a file already holds, and which of those hold nothing, read off
  # the ciphertext without decrypting. `check` and `generate` ask that of files
  # they may hold no identity for — an operator auditing another person's
  # declared-but-valueless names has no way to decrypt and no business doing so
  # — so the reader has to work from the document's shape rather than its
  # contents.
  sops-keys-of = pkgs.writers.writePython3Bin "sops-keys-of" {
    libraries = [ pkgs.python3Packages.pyyaml ];
  } (builtins.readFile ./sops_keys.py);
}
