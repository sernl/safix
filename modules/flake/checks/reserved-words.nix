# Holds the reserved mapping words equal across the two halves.
#
# ../safix/reserved.nix is the one declaration of the list, and
# ../../crates/safix-core/src/bridge.rs's `RESERVED_MAPPING_WORDS` is the
# runtime's copy of it. Both halves need the list — evaluation refuses a
# declared mapping id spelled one of the words, and the runtime meets a mapping
# name off a projection no `violationsOf` ever saw — so neither may be the one
# that drifts, and nothing inside either half can notice if one does.
#
# Modelled on `safix-installer-schema`'s shape: render the nix value, extract
# the committed artifact, and diff the two whole rather than assert a property
# of each.
#
# ── severity: proven by perturbation, one drill per claim ──
# Reordering two words in ../safix/reserved.nix without reordering the Rust
# array turns this check red while every test in `cargo test -p safix-core`
# stays green — including `bridge::tests`'s own
# `the_reserved_words_are_the_six_the_declaration_carries`, which compares the
# Rust literal against a Rust literal and can say nothing about the nix half.
# That pair is the whole of why this check exists rather than a unit test.
# Dropping a word from the Rust array turns it red the same way, on the line
# count rather than on the order.
{
  perSystem =
    { pkgs, lib, ... }:
    {
      checks.safix-reserved-words =
        pkgs.runCommand "safix-reserved-words"
          {
            declared = lib.concatMapStrings (id: "${id}\n") (import ../safix/reserved.nix).ids;
            runtime = ../../../crates/safix-core/src/bridge.rs;
            meta.description = "the reserved mapping words, nix against the Rust literal";
          }
          ''
            printf '%s' "$declared" > declared

            # The literal is one array spanning one or two source lines, so the
            # words are taken from between the brackets rather than by parsing
            # rust: read the whole file as one line, drop everything before the
            # constant and everything from the array's closing bracket on, and
            # print one quoted word per line.
            tr '\n' ' ' < "$runtime" \
              | sed 's/.*pub const RESERVED_MAPPING_WORDS//' \
              | sed 's/\];.*//' \
              | grep -o '"[^"]*"' \
              | tr -d '"' > runtime

            if [ ! -s runtime ]; then
              echo "safix-reserved-words: no RESERVED_MAPPING_WORDS literal was found in"
              echo "safix-reserved-words: crates/safix-core/src/bridge.rs, so nothing below is evidence"
              exit 1
            fi

            if ! diff -u declared runtime; then
              echo ""
              echo "safix-reserved-words: the two halves of the reserved mapping word list disagree"
              echo "safix-reserved-words: the declaration is modules/flake/safix/reserved.nix (left)"
              echo "safix-reserved-words: the runtime's copy is RESERVED_MAPPING_WORDS in"
              echo "safix-reserved-words: crates/safix-core/src/bridge.rs (right)"
              echo "safix-reserved-words: both carry the same words in the same order, and a change"
              echo "safix-reserved-words: to either is a change to both, in the same commit"
              exit 1
            fi

            touch $out
          '';
    };
}
