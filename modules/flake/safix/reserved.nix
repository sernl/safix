# The mapping ids no consumer may declare, in one place.
#
# These are the words `sync` and `audit` read as a target keyword rather than
# as a mapping name: `safix sync pass` names a target, and a mapping literally
# named `pass` would make that first argument ambiguous with no way to tell
# which was meant. Each mapping module refuses a declared id spelled one of
# these, and each keeps its own sentence rather than importing one, because
# each sentence names its own option path — what was duplicated is the list.
#
# The list is complete before three of the targets answer to it. `pass`,
# `bitwarden` and `1password` are reserved here rather than by the three
# changes that add those targets, because three changes each appending one word
# would move the same equality check three times: three chances for the list
# and the Rust literal to disagree, and three reviews of the same diff.
# Reserving a word for a target that does not answer yet costs a consumer
# nothing but the name, and `sync pass` refusing with "no such target" is the
# better of the two confusions.
#
# `op` is deliberately absent. One spelling per target is the rule and
# `1password` is the spelling, so reserving a two-letter word an operator might
# reasonably name a mapping buys nothing.
#
# `crates/safix-core/src/bridge.rs`'s `RESERVED_MAPPING_WORDS` carries the same
# six words in the same order, and `checks.safix-reserved-words` is what holds
# the two equal — the runtime reaches a mapping name from a projection no
# `violationsOf` ever saw, so both halves need the list and neither may be the
# one that drifts.
{
  ids = [
    "clan"
    "keepassxc"
    "pass"
    "bitwarden"
    "1password"
    "all"
  ];
}
