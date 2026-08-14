#!/usr/bin/env bash
# safix-selftest.sh — hermetic driver for the safix command's flake checks.
# Drives the real ./safix.sh (path in SAFIX_SH) against a throwaway git
# repository, throwaway age keys and a fixture .sops.yaml shaped like the
# generated one, asserting what the command owes its caller.
#
#   SAFIX_SH=/path/to/safix.sh safix-selftest.sh set-new
#   ... set-existing | refusals | recipient-drift | staged-bystander | abort
#   ... get-list | shared-placement | shared-shrink | shared-flip
#
# Every key here is minted in the sandbox at test time. No recipient, no
# ciphertext and no value from anywhere else appears in this file or in anything
# it writes.
#
# Only `nix` is stubbed. sops, age and git are the real binaries, because a stub
# standing in for the backend is what lets a check stay green over a command
# calling something the tree no longer contains; every claim below about
# ciphertext, about which key moved, and about what got committed is made
# against the tools that will run at the operator's terminal.
#
# The fixture .sops.yaml reproduces the two properties of the generated one that
# this command depends on: rules anchored with `^` and ending in `\.yaml$`, and
# no catch-all, so an unruled path fails closed exactly as it does in a real
# tree.
set -euo pipefail

mode="${1:?usage: safix-selftest.sh <set-new|set-existing|refusals|recipient-drift|staged-bystander|abort|get-list|generate|generate-refusals|generate-isolation|generate-cascade|governed-extras|adduser|adduser-refusals|adduser-hook|shared-placement|shared-shrink|shared-flip>}"
: "${SAFIX_SH:?SAFIX_SH must point at safix.sh}"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

fail() {
  printf 'selftest[%s]: %s\n' "$mode" "$1" >&2
  exit 1
}

BASH_BIN="$(command -v bash)"

emit_stub() { # <path> <body-on-stdin>
  {
    printf '#!%s\n' "$BASH_BIN"
    cat
  } >"$1"
  chmod +x "$1"
}

# The two people the fixture declares, and what each holds. ana's secrets sit in
# her own file; the pair shares one, which lands in the audience directory named
# for both in sorted order — the `,` separator the resolver picked because the
# name alphabet excludes it.
ANA_FILE="secrets/safix/users/ana/secrets.yaml"
SHARED_FILE="secrets/safix/shared/ana,bo/secrets.yaml"

# sha256 of one key's ciphertext line. Compares a bystander across a write
# without ever rendering its value. Hashes a file rather than stdin, because
# sha256sum is wrapped to reject stdin on some of the machines this runs on
# outside the sandbox.
key_digest() { # <file> <key>
  local scratch="$work/digest.tmp"
  grep -E "^$2:" "$1" >"$scratch"
  sha256sum "$scratch" | cut -d' ' -f1
}

# --- Fixture construction ---------------------------------------------------------
AGE_KEY="$work/age-key.txt"
AGE_PUB=""
# bo's half of the shared audience. Two distinct recipients rather than one key
# under two anchors, so "the created file took its recipients from the creation
# rule" is a claim a file encrypted to the operator alone would fail. Only ana's
# private half is ever given to sops, which is also how the fixture shows that
# writing to a shared file needs no recipient's key but the writer's own.
BO_PUB=""

# The fixture recipient policy, with the shared audience's rule parameterized by
# the anchors it grants. Written through one function rather than restated by the
# test that narrows it, so "the policy the command is driven against" stays a
# single definition: a rule granting fewer anchors than the declared audience is
# exactly the stale `.sops.yaml` a new file would be created through.
rules_block() { # <shared-anchor>...
  local anchor
  printf 'creation_rules:\n'
  printf '  - path_regex: ^secrets/safix/users/ana/[^/]*\\.yaml$\n'
  printf '    key_groups:\n'
  printf '      - age:\n'
  printf '          - *ana\n'
  printf '  - path_regex: ^secrets/safix/shared/ana,bo/[^/]*\\.yaml$\n'
  printf '    key_groups:\n'
  printf '      - age:\n'
  for anchor in "$@"; do
    printf '          - *%s\n' "$anchor"
  done
}

write_policy() { # <shared-anchor>...
  {
    printf 'keys:\n'
    printf '  - &ana %s\n' "$AGE_PUB"
    printf '  - &bo %s\n' "$BO_PUB"
    rules_block "$@"
  } >"$REPO/.sops.yaml"
}

setup_repo() { # -> sets REPO
  REPO="$work/repo"
  mkdir -p "$REPO"

  [ -s "$AGE_KEY" ] || age-keygen -o "$AGE_KEY" 2>/dev/null
  AGE_PUB="$(age-keygen -y "$AGE_KEY")"
  [ -n "$BO_PUB" ] || BO_PUB="$(age-keygen 2>/dev/null | age-keygen -y /dev/stdin)"
  export SOPS_AGE_KEY_FILE="$AGE_KEY"

  write_policy ana bo

  git -C "$REPO" init -q
  git -C "$REPO" config user.email selftest@example.invalid
  git -C "$REPO" config user.name selftest
  git -C "$REPO" add -A
  git -C "$REPO" commit -q -m "fixture: recipient policy"

  # `nix` is the only stubbed binary: a flake evaluation is what a sandbox
  # cannot do. The stub asserts the command asks for the attribute it claims to
  # read, so a rename of flake.safix.lib.placements fails here.
  mkdir -p "$work/bin"
  emit_stub "$work/bin/nix" <<'SH'
case "${1:-}" in
  eval)
    case " $* " in
      *"#safix.lib.placements"*) cat "$SAFIX_FIXTURE_PLACEMENTS" ;;
      *"#safix.lib.audiences"*) cat "$SAFIX_FIXTURE_AUDIENCES" ;;
      *"#safix.lib.recipients"*) cat "$SAFIX_FIXTURE_RECIPIENTS" ;;
      *"#safix.lib.governedFiles"*)
        # `required` is the audiences the declarations imply and `extra` is what
        # the consumer named, exactly as ./default.nix computes them; `managed`
        # is their union, which is the set `fix` re-wraps. Computed here rather
        # than fixed, so a mode that adds an extra file changes all three the
        # way an evaluation would.
        jq -n --slurpfile aud "$SAFIX_FIXTURE_AUDIENCES" \
              --slurpfile extra "$SAFIX_FIXTURE_EXTRAS" '
            ($aud[0] | keys) as $required
          | ($extra[0] | unique) as $extra
          | { required: $required,
              extra: $extra,
              managed: (($required + $extra) | unique) }'
        ;;
      *"#safix.lib.generatorPlan"*) cat "$SAFIX_FIXTURE_GENPLAN" ;;
      *"#safix.lib.nameRegex"*) printf '%s' '[a-z0-9][a-z0-9_-]*' ;;
      *"#safix.onboardingHook"*) cat "$SAFIX_FIXTURE_HOOK" ;;
      *"#safix.lib.policyText"*)
        # The generated policy is a function of the declarations, and a flake
        # evaluation sees only the files git tracks. Reproduced here rather than
        # asserted about, because that property is exactly what `adduser`'s
        # staging order exists for: a command that regenerates before staging its
        # scaffold writes a policy missing the person it has just declared, and
        # this stub is what notices.
        printf 'keys:\n'
        git -C "$SAFIX_REPO_ROOT" ls-files -- safix/users \
          | grep '\.nix$' \
          | while IFS= read -r m; do
            u="${m#safix/users/}"
            u="${u%.nix}"
            r="$(sed -n 's/^ *recipient = "\(.*\)";$/\1/p' "$SAFIX_REPO_ROOT/$m")"
            [ -n "$r" ] || continue
            printf '  - &%s %s\n' "$u" "$r"
          done
        # The rules half comes from the fixture rather than from the
        # declarations, because rendering it is ./policy.nix's claim and this
        # stub stands in for an evaluation, not for the renderer. What matters
        # here is that the anchors follow what git tracks, which is what the
        # staging-order drill turns on.
        cat "$SAFIX_FIXTURE_RULES"
        ;;
      *) echo "stub nix: unexpected attribute: $*" >&2; exit 1 ;;
    esac
    ;;
  shell)
    # `nix shell` is stubbed on the same grounds `nix eval` is: it resolves and
    # realises store paths, which a build sandbox cannot do. The stub asserts the
    # shape of the invocation instead — the flake the inputs are resolved from,
    # every spec being an attribute of that flake's nixpkgs, and the `-c` — so a
    # change in how a generator's runtimeInputs are requested fails here rather
    # than at an operator's rotation. What it cannot assert is that the packages
    # exist; the generator-tools check is what does that.
    shift
    [ "${1:-}" = --inputs-from ] || { echo "stub nix shell: expected --inputs-from, got '${1:-}'" >&2; exit 1; }
    [ "${2:-}" = "$SAFIX_REPO_ROOT" ] || { echo "stub nix shell: --inputs-from names '${2:-}', not the repository" >&2; exit 1; }
    shift 2
    while [ $# -gt 0 ] && [ "$1" != -c ]; do
      case "$1" in
        nixpkgs#*) ;;
        *) echo "stub nix shell: '$1' is not a nixpkgs#<attr> spec" >&2; exit 1 ;;
      esac
      shift
    done
    [ "${1:-}" = -c ] || { echo "stub nix shell: no -c in the invocation" >&2; exit 1; }
    shift
    exec "$@"
    ;;
  *) echo "stub nix: unexpected invocation: $*" >&2; exit 1 ;;
esac
SH

  # ana holds three of her own and one shared to her by bo; bo owns the shared
  # one. `no-rule-secret` resolves into a directory the fixture policy has no
  # creation rule for, which is the fail-closed case.
  cat >"$work/placements.json" <<EOF
{
  "ana": {
    "api-token":       { "file": "$ANA_FILE",    "key": "api-token",     "origin": "carries", "owner": "ana", "shared": false, "generator": null },
    "mail-password":   { "file": "$ANA_FILE",    "key": "mail-password", "origin": "private", "owner": "ana", "shared": false, "generator": null },
    "aliased-secret":  { "file": "$ANA_FILE",    "key": "custom-key",    "origin": "private", "owner": "ana", "shared": false, "generator": null },
    "wifi-psk":        { "file": "$SHARED_FILE", "key": "wifi-psk",      "origin": "shared",  "owner": "bo",  "shared": false, "generator": null },
    "no-rule-secret":  { "file": "secrets/safix/users/cy/secrets.yaml", "key": "no-rule-secret", "origin": "private", "owner": "ana", "shared": false, "generator": null },
    "not-yaml":        { "file": "secrets/safix/users/ana/secret.age", "key": "not-yaml", "origin": "private", "owner": "ana", "shared": false, "generator": null }
  },
  "bo": {
    "wifi-psk": { "file": "$SHARED_FILE", "key": "wifi-psk", "origin": "private", "owner": "bo", "shared": false, "generator": null }
  }
}
EOF

  # Who the declarations say can open each file they place a secret in — the
  # shape flake.safix.lib.audiences has. The fixture policy's creation rules
  # grant exactly these, so a file created or re-wrapped through those rules
  # agrees with this and only a file written around them drifts.
  cat >"$work/audiences.json" <<EOF
{
  "$ANA_FILE":    { "audience": ["ana"],       "dir": "secrets/safix/users/ana",      "recipients": ["$AGE_PUB"] },
  "$SHARED_FILE": { "audience": ["ana", "bo"], "dir": "secrets/safix/shared/ana,bo",  "recipients": ["$AGE_PUB", "$BO_PUB"] }
}
EOF

  # The run plan starts empty; each mode seeds exactly the generators it drives,
  # so a mode's `order` is what that mode actually runs and no other mode's
  # fixture can make a claim hold by accident.
  printf '{ "ana": { "order": [], "outputs": {}, "inputs": {} }, "bo": { "order": [], "outputs": {}, "inputs": {} } }\n' \
    >"$work/genplan.json"

  # No consumer-named extras and no onboarding hook by default, which is the
  # configuration every mode but two is driven under.
  printf '[]\n' >"$work/extras.json"
  printf 'null\n' >"$work/hook.json"

  # The creation rules the stub's rendered policy carries, kept as one
  # definition with the committed fixture so that regenerating and committing
  # the policy is a no-op wherever the declarations have not moved.
  rules_block ana bo >"$work/rules.txt"

  # Which key is whose, the direction the audience map cannot answer. Two
  # entries, because a check that reports who can open a stray copy has to tell
  # ana's key from bo's rather than report the key itself.
  cat >"$work/recipients.json" <<EOF
{ "ana": ["$AGE_PUB"], "bo": ["$BO_PUB"] }
EOF

  export SAFIX_FIXTURE_RECIPIENTS="$work/recipients.json"
  export SAFIX_FIXTURE_GENPLAN="$work/genplan.json"
  export SAFIX_FIXTURE_PLACEMENTS="$work/placements.json"
  export SAFIX_FIXTURE_AUDIENCES="$work/audiences.json"
  export SAFIX_FIXTURE_EXTRAS="$work/extras.json"
  export SAFIX_FIXTURE_HOOK="$work/hook.json"
  export SAFIX_FIXTURE_RULES="$work/rules.txt"
  export SAFIX_REPO_ROOT="$REPO"
  export SAFIX_NIX="$work/bin/nix"
  export USER=ana
}

# Write a real multi-key sops file at <root>/<rel> carrying exactly the named
# keys. `--config` is not overridden: the fixture policy is the one under test.
make_sops_file() { # <rel> <key>...
  local rel="$1" plain="$work/plain.yaml" key
  shift
  mkdir -p "$REPO/$(dirname "$rel")"
  : >"$plain"
  for key in "$@"; do
    printf '%s: "fixture-value-for-%s"\n' "$key" "$key" >>"$plain"
  done
  (cd "$REPO" && sops encrypt --filename-override "$rel" \
    --input-type yaml --output-type yaml "$plain") >"$REPO/$rel"
  git -C "$REPO" add -- "$rel"
  git -C "$REPO" commit -q -m "fixture: $rel"
}

# Declare one placement for a name that has no generator — the further outputs a
# multi-output generator writes are entries in their own right, which is what
# gives each its own key and its own file.
seed_output() { # <name> <file>
  jq --arg n "$1" --arg f "$2" \
    '.ana[$n] = { file: $f, key: $n, origin: "private", owner: "ana", shared: false, generator: null }' \
    "$work/placements.json" >"$work/p.tmp"
  mv "$work/p.tmp" "$work/placements.json"
}

# A catalogue entry both users carry and `shared = true` makes one value of. Two
# placements rather than one, each with its carrier as `owner`, because that is
# the shape `carries` resolves to: the catalogue entry has no owner, and a
# carrier owns their selection of it. Both name one file and one key, which is
# the property the shared modes are here to hold the command to.
seed_shared() { # <name> <file>
  jq --arg n "$1" --arg f "$2" '
      .ana[$n] = { file: $f, key: $n, origin: "carries", owner: "ana",
                   shared: true, generator: null }
    | .bo[$n]  = { file: $f, key: $n, origin: "carries", owner: "bo",
                   shared: true, generator: null }
  ' "$work/placements.json" >"$work/p.tmp"
  mv "$work/p.tmp" "$work/placements.json"
}

# Drop a carrier from a shared entry: they stop carrying it, and the audience
# that remains is one person, so the entry resolves into that person's own file.
# The declarations are the only thing edited — the ciphertext is left exactly
# where it was, which is the state a revocation is actually discovered in.
unshare_from() { # <name> <remaining-carrier> <their-file>
  jq --arg n "$1" --arg u "$2" --arg f "$3" '
      del(.bo[$n])
    | .[$u][$n].file = $f
  ' "$work/placements.json" >"$work/p.tmp"
  mv "$work/p.tmp" "$work/placements.json"
}

# Declare a generator, and derive its run-plan entry from the same generator
# record the command reads. `inputs` is computed here the way
# modules/flake/safix/resolve.nix computes it — prompts and dependencies in one
# name space, hyphens mapped to underscores — so a change to that mapping on one
# side and not the other fails these checks.
# The generator record arrives on standard input, as a quoted heredoc at the call
# site. Its `script` is bash for another shell to run, so it holds quotes and `$`
# this shell must not expand, and a quoted heredoc is the one spelling that
# passes both through untouched.
seed_generator() { # <name> <file> [<further-output-name>...] ; record on stdin
  local name="$1" file="$2" gen outs
  shift 2
  gen="$(cat)"
  outs="$(printf '%s\n' "$name" "$@" | jq -Rnc '[inputs]')"
  jq --arg n "$name" --arg f "$file" --argjson g "$gen" \
    '.ana[$n] = { file: $f, key: $n, origin: "private", owner: "ana", shared: false, generator: $g }' \
    "$work/placements.json" >"$work/p.tmp"
  mv "$work/p.tmp" "$work/placements.json"
  jq --arg n "$name" --argjson outs "$outs" --argjson g "$gen" '
      .ana.order += [$n]
    | .ana.outputs[$n] = $outs
    | .ana.inputs[$n] = (
        (($g.prompts // {}) | to_entries
          | map({ key: (.key | gsub("-"; "_")), value: { kind: "prompt", name: .key } }))
        + (($g.dependencies // []) | map({ key: (gsub("-"; "_")), value: { kind: "dependency", name: . } }))
        | from_entries)
  ' "$work/genplan.json" >"$work/g.tmp"
  mv "$work/g.tmp" "$work/genplan.json"
}

# Read one key back out of a file, as a digest, so a value is compared without
# being rendered into this check's output.
value_digest() { # <rel> <key>
  sops decrypt --extract "[\"$2\"]" "$REPO/$1" | sha256sum | cut -d' ' -f1
}

digest_of() { # <string>
  printf '%s' "$1" | sha256sum | cut -d' ' -f1
}

# Drive the command with the two prompt reads fed on stdin. There is no
# controlling terminal in the sandbox, which is the branch the command takes when
# /dev/tty cannot be opened; the reads themselves are the same `read -rs`.
run_set() { # <value> <args...>
  local value="$1"
  shift
  printf '%s\n%s\n' "$value" "$value" | bash "$SAFIX_SH" set "$@"
}

run_set_confirm() { # <value> <confirmation> <args...>
  local value="$1" confirm="$2"
  shift 2
  printf '%s\n%s\n' "$value" "$confirm" | bash "$SAFIX_SH" set "$@"
}

# --- Modes -------------------------------------------------------------------------

# The file does not exist: it is created through sops so the creation rules
# choose its recipients, the value lands under the right key, and the file is
# committed on its own with a message naming the secret and not the value.
test_set_new() {
  setup_repo
  local out rc=0
  out="$(run_set 'CANARY-shared-value' ana wifi-psk 2>&1)" || rc=$?
  [ "$rc" = 0 ] || { printf '%s\n' "$out" >&2; fail "set on a new file failed"; }

  [ -f "$REPO/$SHARED_FILE" ] || fail "the audience file was not created"

  # Created THROUGH sops: the file carries the creation rule's recipients rather
  # than whatever a hand-rolled encryption would have picked. Both halves of the
  # audience must be there — a file encrypted to the writer alone would satisfy
  # "it is encrypted" and hand the other party a file they cannot open, which is
  # the whole failure mode audience-keyed placement exists to prevent.
  grep -qF "$AGE_PUB" "$REPO/$SHARED_FILE" || fail "the new shared file is not encrypted to ana"
  grep -qF "$BO_PUB" "$REPO/$SHARED_FILE" || fail "the new shared file is not encrypted to bo"
  grep -q 'ENC\[AES256_GCM' "$REPO/$SHARED_FILE" || fail "the new file holds no sops ciphertext"

  # The value round-trips under the resolved key.
  [ "$(sops decrypt --extract '["wifi-psk"]' "$REPO/$SHARED_FILE")" = "CANARY-shared-value" ] \
    || fail "the value did not round-trip under the resolved key"

  # Committed, alone, with a message naming the secret and never the value.
  local subject files
  subject="$(git -C "$REPO" log -1 --format=%s)"
  [ "$subject" = "chore(safix): set wifi-psk for ana" ] \
    || fail "commit subject is '$subject'"
  files="$(git -C "$REPO" show --name-only --format= HEAD | grep -c .)"
  [ "$files" = 1 ] || fail "the commit touched $files files, expected 1"
  [ "$(git -C "$REPO" show --name-only --format= HEAD)" = "$SHARED_FILE" ] \
    || fail "the commit touched the wrong file"
  if git -C "$REPO" log -1 --format='%s%n%b' | grep -qF 'CANARY-shared-value'; then
    fail "the commit message carries the value"
  fi
  if [ -n "$(git -C "$REPO" status --porcelain)" ]; then
    git -C "$REPO" status --porcelain >&2
    fail "the working tree is not clean after the write"
  fi

  # The other half of one rule per audience: a file created for ana alone must
  # not name bo. A single rule covering both would hand a grant's recipient
  # everything its owner holds, which is the disclosure the audience split exists
  # to prevent, and it would pass every assertion above.
  make_sops_file "$ANA_FILE" api-token
  if grep -qF "$BO_PUB" "$REPO/$ANA_FILE"; then
    fail "ana's own file names bo as a recipient"
  fi

  echo "set-new: OK"
}

# The file exists and holds other keys: only the target key may move, the same
# value twice must leave the file byte-identical and produce no second commit,
# and an entry whose sopsKey differs from its name must land under the key rather
# than under the name.
test_set_existing() {
  setup_repo
  make_sops_file "$ANA_FILE" api-token mail-password bystander-one bystander-two

  local k before_head
  local -A before=()
  for k in mail-password bystander-one bystander-two; do
    before["$k"]="$(key_digest "$REPO/$ANA_FILE" "$k")"
  done
  before_head="$(git -C "$REPO" rev-parse HEAD)"

  run_set 'CANARY-api-v1' ana api-token >/dev/null 2>&1 || fail "set on an existing file failed"

  [ "$(sops decrypt --extract '["api-token"]' "$REPO/$ANA_FILE")" = "CANARY-api-v1" ] \
    || fail "the target key does not hold the new value"
  for k in mail-password bystander-one bystander-two; do
    [ "$(key_digest "$REPO/$ANA_FILE" "$k")" = "${before[$k]}" ] \
      || fail "bystander key '$k' was disturbed by the write"
  done
  [ "$(git -C "$REPO" rev-parse HEAD)" != "$before_head" ] || fail "the write produced no commit"

  # Idempotent: the same value again is byte-identical and commits nothing.
  #
  # The second boundary is what makes this severe. sops stamps `lastmodified` at
  # one-second resolution and reuses an unchanged value's IV, so a re-run inside
  # the same second is byte-identical whether or not `--idempotent` was passed,
  # and an assertion made without waiting would hold over a command that had
  # dropped the flag. Waiting is also the operator's real case: a re-run minutes
  # later either writes a diff that says nothing or writes nothing at all.
  local snapshot="$work/snapshot.yaml" head_after out
  cp "$REPO/$ANA_FILE" "$snapshot"
  head_after="$(git -C "$REPO" rev-parse HEAD)"
  sleep 1.1
  out="$(run_set 'CANARY-api-v1' ana api-token 2>&1)" || fail "the idempotent re-run failed"
  cmp -s "$snapshot" "$REPO/$ANA_FILE" \
    || fail "re-setting the same value rewrote the file (must be byte-identical)"
  [ "$(git -C "$REPO" rev-parse HEAD)" = "$head_after" ] \
    || fail "the idempotent re-run made a commit"
  printf '%s\n' "$out" | grep -qF 'unchanged' || {
    printf '%s\n' "$out" >&2
    fail "the idempotent re-run did not say the file was unchanged"
  }

  # A different value moves the target key and nothing else.
  for k in mail-password bystander-one bystander-two; do
    before["$k"]="$(key_digest "$REPO/$ANA_FILE" "$k")"
  done
  run_set 'CANARY-api-v2' ana api-token >/dev/null 2>&1 || fail "the rotation failed"
  [ "$(sops decrypt --extract '["api-token"]' "$REPO/$ANA_FILE")" = "CANARY-api-v2" ] \
    || fail "the rotation did not change the value"
  for k in mail-password bystander-one bystander-two; do
    [ "$(key_digest "$REPO/$ANA_FILE" "$k")" = "${before[$k]}" ] \
      || fail "bystander key '$k' was disturbed by the rotation"
  done

  # sopsKey: an entry may name a key that differs from the secret's name, and the
  # value must follow the key. Writing under the name instead would leave the
  # profile reading an absent key while this reported success.
  run_set 'CANARY-aliased' ana aliased-secret >/dev/null 2>&1 || fail "the aliased set failed"
  [ "$(sops decrypt --extract '["custom-key"]' "$REPO/$ANA_FILE")" = "CANARY-aliased" ] \
    || fail "an entry with a sopsKey did not land under that key"
  if grep -qE '^aliased-secret:' "$REPO/$ANA_FILE"; then
    fail "an entry with a sopsKey also wrote a key named after the secret"
  fi

  # A mistyped confirmation writes nothing at all.
  cp "$REPO/$ANA_FILE" "$snapshot"
  local rc=0
  run_set_confirm 'CANARY-typo-a' 'CANARY-typo-b' ana api-token >/dev/null 2>&1 || rc=$?
  [ "$rc" != 0 ] || fail "a mismatched confirmation was accepted"
  cmp -s "$snapshot" "$REPO/$ANA_FILE" || fail "a mismatched confirmation still wrote the file"

  echo "set-existing: OK"
}

# The refusals that must never be resolved by guessing: a name no declaration
# covers, and a path the recipient policy has no rule for.
test_refusals() {
  setup_repo
  local out rc

  rc=0
  out="$(run_set 'CANARY-unknown' ana not-declared-anywhere 2>&1)" || rc=$?
  [ "$rc" != 0 ] || fail "an undeclared name was accepted"
  printf '%s\n' "$out" | grep -qF 'flake.safix.catalogue.not-declared-anywhere' \
    || fail "the refusal does not name the catalogue surface"
  printf '%s\n' "$out" | grep -qF 'flake.safix.users.ana.private.not-declared-anywhere' \
    || fail "the refusal does not name the private surface"
  printf '%s\n' "$out" | grep -qF 'sharedWith.ana.not-declared-anywhere' \
    || fail "the refusal does not name the sharedWith surface"
  # Nothing in the message may name an option path belonging to a consumer.
  if printf '%s\n' "$out" | grep -qE 'flake\.(users|homeSecrets)\.'; then
    fail "the refusal names an option path outside safix's namespace"
  fi
  # No destination may be invented for it.
  [ -z "$(git -C "$REPO" status --porcelain)" ] || fail "the refused name still touched the tree"

  # A declared name whose file the fixture policy writes no rule for. The refusal
  # has to name the regenerator rather than write an unruled file.
  rc=0
  out="$(run_set 'CANARY-norule' ana no-rule-secret 2>&1)" || rc=$?
  [ "$rc" != 0 ] || fail "a path with no creation rule was accepted"
  printf '%s\n' "$out" | grep -qF 'safix fix' \
    || fail "the no-rule refusal does not name the command that regenerates the rules"
  printf '%s\n' "$out" | grep -qF 'no creation rule' \
    || fail "the no-rule refusal does not say what is missing"
  [ ! -e "$REPO/secrets/safix/users/cy/secrets.yaml" ] \
    || fail "an unruled file was created"
  [ -z "$(git -C "$REPO" status --porcelain)" ] || fail "the no-rule refusal left something behind"

  # A placement outside `*.yaml`. Every generated rule ends in `\.yaml$` so that
  # a sweep can never reach encrypted material safix did not place; a placement
  # there is a path no rule covers, and sops would guess its format from the
  # extension besides.
  rc=0
  out="$(run_set 'CANARY-notyaml' ana not-yaml 2>&1)" || rc=$?
  [ "$rc" != 0 ] || fail "a non-yaml placement was accepted"
  printf '%s\n' "$out" | grep -qF 'not a *.yaml path' || fail "the non-yaml refusal is not specific"
  [ ! -e "$REPO/secrets/safix/users/ana/secret.age" ] || fail "a non-yaml file was written"

  # An empty value is the written-but-empty state a truncated write leaves, and
  # a probe matching the key name alone would call it converged.
  rc=0
  printf '\n\n' | bash "$SAFIX_SH" set ana api-token >/dev/null 2>&1 || rc=$?
  [ "$rc" != 0 ] || fail "an empty value was accepted"

  # An unknown user is a distinct refusal from an unknown name.
  rc=0
  out="$(run_set 'CANARY-nouser' cy api-token 2>&1)" || rc=$?
  [ "$rc" != 0 ] || fail "an undeclared user was accepted"
  printf '%s\n' "$out" | grep -qF 'not a declared user' || fail "the unknown-user refusal is not specific"

  # A dirty target file is refused: committing it would carry an edit this
  # command did not make under a message naming only one secret.
  make_sops_file "$ANA_FILE" api-token
  printf 'hand edit\n' >>"$REPO/$ANA_FILE"
  rc=0
  out="$(run_set 'CANARY-dirty' ana api-token 2>&1)" || rc=$?
  [ "$rc" != 0 ] || fail "a dirty target file was accepted"
  printf '%s\n' "$out" | grep -qF 'uncommitted changes' || fail "the dirty-file refusal is not specific"
  git -C "$REPO" checkout -- "$ANA_FILE"

  # Mid-rebase and mid-merge: a partial commit means something else there.
  local gitdir
  gitdir="$(git -C "$REPO" rev-parse --absolute-git-dir)"
  : >"$gitdir/MERGE_HEAD"
  rc=0
  out="$(run_set 'CANARY-merge' ana api-token 2>&1)" || rc=$?
  [ "$rc" != 0 ] || fail "a run mid-merge was accepted"
  printf '%s\n' "$out" | grep -qF 'mid-MERGE_HEAD' || fail "the mid-merge refusal is not specific"
  rm -f "$gitdir/MERGE_HEAD"

  mkdir -p "$gitdir/rebase-merge"
  rc=0
  out="$(run_set 'CANARY-rebase' ana api-token 2>&1)" || rc=$?
  [ "$rc" != 0 ] || fail "a run mid-rebase was accepted"
  printf '%s\n' "$out" | grep -qF 'mid-rebase-merge' || fail "the mid-rebase refusal is not specific"
  rmdir "$gitdir/rebase-merge"

  # An unrecognised subcommand names the set it accepts rather than failing bare.
  rc=0
  out="$(bash "$SAFIX_SH" frobnicate 2>&1)" || rc=$?
  [ "$rc" != 0 ] || fail "an unknown subcommand was accepted"
  printf '%s\n' "$out" | grep -qF 'unknown subcommand' || fail "the unknown-subcommand refusal is not specific"
  printf '%s\n' "$out" | grep -qF 'adduser' || fail "the unknown-subcommand refusal does not name the subcommands it accepts"

  echo "refusals: OK"
}

# `sops set` on an existing file reuses that file's own recipient metadata, so a
# file whose recipients have drifted from the audience declared for it takes a
# new value and wraps it for the audience that used to be. This command commits
# what it writes, so that hands a removed reader a value minted after their
# removal, out of git history, before any drift check is ever run. The refusal
# must land before the rename and leave HEAD and the file exactly as it found
# them, and it must stop refusing once the drift is repaired.
test_recipient_drift() {
  setup_repo

  # An identity no audience in the fixture names. Encrypting straight to it goes
  # around the creation rule, which is the state a file is actually left in when
  # someone is dropped from sharedWith, the policy is regenerated, and the
  # ciphertext has not yet been re-wrapped.
  local stranger_pub
  stranger_pub="$(age-keygen 2>/dev/null | age-keygen -y /dev/stdin)"

  # `--config /dev/null` because the recipients here must come from `--age` and
  # from nothing else. sops otherwise searches upward from the working directory
  # for a `.sops.yaml` and fails when no rule matches the input path, so without
  # it this fixture would depend on whichever directory the driver was invoked
  # from — passing in the sandbox and failing inside a checkout, or the reverse.
  mkdir -p "$REPO/$(dirname "$ANA_FILE")"
  printf 'api-token: "fixture-value-for-api-token"\n' >"$work/drifted.yaml"
  sops --config /dev/null encrypt --age "$AGE_PUB,$stranger_pub" \
    --input-type yaml --output-type yaml "$work/drifted.yaml" >"$REPO/$ANA_FILE"
  git -C "$REPO" add -- "$ANA_FILE"
  git -C "$REPO" commit -q -m "fixture: recipients drifted from the declared audience"
  grep -qF "$stranger_pub" "$REPO/$ANA_FILE" || fail "the fixture file is not actually drifted"

  local head_before digest_before out rc=0
  head_before="$(git -C "$REPO" rev-parse HEAD)"
  digest_before="$(sha256sum "$REPO/$ANA_FILE" | cut -d' ' -f1)"

  out="$(run_set 'CANARY-DRIFT-abcdef' ana api-token 2>&1)" || rc=$?
  [ "$rc" != 0 ] || fail "a value was minted into a file whose recipients have drifted"
  printf '%s\n' "$out" | grep -qF "$stranger_pub" \
    || fail "the drift refusal does not name the recipient the audience does not"
  printf '%s\n' "$out" | grep -qF "$ANA_FILE" || fail "the drift refusal does not name the file"
  printf '%s\n' "$out" | grep -qF 'safix fix' \
    || fail "the drift refusal does not name the command that re-wraps the file"

  # The whole claim: the run left nothing behind. A refusal that had already
  # renamed the scratch file into place would fail here even while its message
  # read correctly.
  [ "$(git -C "$REPO" rev-parse HEAD)" = "$head_before" ] \
    || fail "the drift refusal still made a commit"
  [ "$(sha256sum "$REPO/$ANA_FILE" | cut -d' ' -f1)" = "$digest_before" ] \
    || fail "the drift refusal still rewrote the target file"
  [ -z "$(git -C "$REPO" status --porcelain)" ] || fail "the drift refusal left the tree dirty"
  if find "$REPO" -name '*safix-tmp*' -print -quit | grep -q .; then
    fail "the drift refusal left a scratch file beside the target"
  fi
  if grep -rlF 'CANARY-DRIFT-abcdef' "$REPO" >/dev/null 2>&1; then
    fail "the refused value survived inside the repository"
  fi

  # The other direction, and the other write path. A file that does not exist yet
  # takes its recipients from `.sops.yaml`, so the drift that reaches it is a
  # stale creation rule rather than stale metadata; narrowing the shared rule to
  # ana leaves bo in the declared audience while the rule no longer grants him.
  # This is the arm that judging the file already in place would miss entirely —
  # there is no file in place to read.
  write_policy ana
  git -C "$REPO" add -- .sops.yaml
  git -C "$REPO" commit -q -m "fixture: creation rule narrower than the declared audience"

  head_before="$(git -C "$REPO" rev-parse HEAD)"
  rc=0
  out="$(run_set 'CANARY-narrowed' ana wifi-psk 2>&1)" || rc=$?
  [ "$rc" != 0 ] || fail "a value was minted into a file one of its audience cannot open"
  printf '%s\n' "$out" | grep -qF "$BO_PUB" \
    || fail "the refusal does not name the audience member that cannot open the file"

  # Created and then refused must leave nothing: not the file, and not the
  # directories `mkdir -p` made for it. This is the rollback the scratch-file
  # shape is supposed to give for free, asserted rather than assumed.
  [ ! -e "$REPO/$SHARED_FILE" ] || fail "the refused creation left the file behind"
  [ ! -d "$REPO/secrets/safix/shared/ana,bo" ] \
    || fail "the refused creation left the audience directory behind"
  [ ! -d "$REPO/secrets/safix/shared" ] \
    || fail "the refused creation left the shared/ parent behind"
  [ "$(git -C "$REPO" rev-parse HEAD)" = "$head_before" ] \
    || fail "the refused creation still made a commit"
  [ -z "$(git -C "$REPO" status --porcelain)" ] || fail "the refused creation left the tree dirty"

  write_policy ana bo
  git -C "$REPO" add -- .sops.yaml
  git -C "$REPO" commit -q -m "fixture: creation rule back in step with the audience"

  # Repaired, the same set goes through. `sops updatekeys` is what `safix fix`
  # runs; the fixture rule grants ana alone, so it drops the extra identity and
  # the file agrees with its declared audience again.
  (cd "$REPO" && sops updatekeys -y "$ANA_FILE" >/dev/null 2>&1) \
    || fail "could not re-wrap the drifted file"
  git -C "$REPO" add -- "$ANA_FILE"
  git -C "$REPO" commit -q -m "fixture: re-wrapped to the declared audience"
  if grep -qF "$stranger_pub" "$REPO/$ANA_FILE"; then
    fail "the re-wrap did not drop the extra recipient"
  fi

  head_before="$(git -C "$REPO" rev-parse HEAD)"
  run_set 'CANARY-after-rewrap' ana api-token >/dev/null 2>&1 \
    || fail "the set was still refused after the drift was repaired"
  [ "$(git -C "$REPO" rev-parse HEAD)" != "$head_before" ] \
    || fail "the repaired set produced no commit"
  [ "$(sops decrypt --extract '["api-token"]' "$REPO/$ANA_FILE")" = "CANARY-after-rewrap" ] \
    || fail "the repaired set did not store the value"

  echo "recipient-drift: OK"
}

# Another path's staged change must survive the run staged and uncommitted. An
# unscoped `git commit` would sweep it into a commit whose message names one
# secret, and an unscoped `git diff --cached` would read it as this command's own
# work and commit on a run that wrote nothing.
test_staged_bystander() {
  setup_repo
  make_sops_file "$ANA_FILE" api-token

  printf 'unrelated work in progress\n' >"$REPO/unrelated.txt"
  git -C "$REPO" add -- unrelated.txt

  run_set 'CANARY-scoped' ana api-token >/dev/null 2>&1 || fail "the scoped set failed"

  local committed staged
  committed="$(git -C "$REPO" show --name-only --format= HEAD)"
  [ "$committed" = "$ANA_FILE" ] || fail "the commit touched '$committed', expected only $ANA_FILE"
  staged="$(git -C "$REPO" diff --cached --name-only)"
  [ "$staged" = "unrelated.txt" ] || fail "the unrelated staging did not survive (staged: '$staged')"
  [ "$(cat "$REPO/unrelated.txt")" = "unrelated work in progress" ] \
    || fail "the unrelated file's content was disturbed"

  # And with an unrelated path staged, an idempotent re-run must still commit
  # nothing: the emptiness check has to be scoped to the target.
  local head_before
  head_before="$(git -C "$REPO" rev-parse HEAD)"
  run_set 'CANARY-scoped' ana api-token >/dev/null 2>&1 || fail "the scoped re-run failed"
  [ "$(git -C "$REPO" rev-parse HEAD)" = "$head_before" ] \
    || fail "an unrelated staged path made the idempotent re-run commit"

  echo "staged-bystander: OK"
}

# Aborting must leave the tree as it was found and no plaintext anywhere,
# including $TMPDIR. Two aborts: a SIGINT while the prompt is waiting, and a
# backend that fails after the value has been read — the second is the path a
# `trap ... RETURN` would miss.
test_abort() {
  setup_repo
  local scratch="$work/tmp-abort" rc=0
  mkdir -p "$scratch"

  # SIGINT at the prompt. stdin is a fifo nobody writes to, so the read blocks
  # until the signal arrives.
  local fifo="$work/prompt.fifo"
  mkfifo "$fifo"
  # shellcheck disable=SC2094 # the fifo is opened for writing only to keep it from EOF
  exec 9<>"$fifo"
  rc=0
  TMPDIR="$scratch" timeout -s INT 5 bash "$SAFIX_SH" set ana wifi-psk <"$fifo" >/dev/null 2>&1 || rc=$?
  exec 9>&-
  [ "$rc" != 0 ] || fail "the interrupted run reported success"
  [ ! -e "$REPO/$SHARED_FILE" ] || fail "the interrupted run left a partial file behind"
  if find "$REPO" -name '*safix-tmp*' -print -quit | grep -q .; then
    fail "the interrupted run left a scratch file behind"
  fi
  [ ! -d "$REPO/secrets/safix/shared/ana,bo" ] \
    || fail "the interrupted run left the audience directory behind"
  # `mkdir -p` created two levels for a first shared audience, so both have to go.
  [ ! -d "$REPO/secrets/safix/shared" ] \
    || fail "the interrupted run left the shared/ parent behind"
  [ -z "$(git -C "$REPO" status --porcelain)" ] || fail "the interrupted run left the tree dirty"

  # A backend that fails after the value has been read. The value is a canary and
  # must appear nowhere on disk; the target file must be untouched so the next
  # run retries rather than finding a half-written one.
  make_sops_file "$ANA_FILE" api-token bystander-one
  local before
  before="$(key_digest "$REPO/$ANA_FILE" api-token)"
  emit_stub "$work/bin/sops-fails" <<'SH'
exit 1
SH
  rc=0
  TMPDIR="$scratch" SAFIX_SOPS="$work/bin/sops-fails" \
    run_set 'CANARY-LEAK-abcdef' ana api-token >/dev/null 2>&1 || rc=$?
  [ "$rc" != 0 ] || fail "a failing backend was reported as success"
  if grep -rlF 'CANARY-LEAK-abcdef' "$scratch" >/dev/null 2>&1; then
    fail "the value survived the aborted run under TMPDIR"
  fi
  if grep -rlF 'CANARY-LEAK-abcdef' "$REPO" >/dev/null 2>&1; then
    fail "the value survived the aborted run inside the repository"
  fi
  [ "$(key_digest "$REPO/$ANA_FILE" api-token)" = "$before" ] \
    || fail "the failing backend still moved the target key"
  # The scratch file beside the target must be gone, not merely unreferenced.
  if find "$REPO" -name '*safix-tmp*' -print -quit | grep -q .; then
    fail "a scratch file was left beside the target"
  fi
  [ -z "$(git -C "$REPO" status --porcelain)" ] || fail "the failed run left the tree dirty"

  echo "abort: OK"
}

# get round-trips the fixture value by digest, and list reports every name with
# the file serving it.
test_get_list() {
  setup_repo
  make_sops_file "$ANA_FILE" api-token mail-password
  make_sops_file "$SHARED_FILE" wifi-psk

  # Round-trip by digest, so the value is compared without being rendered into
  # the check's own output.
  local expected got
  printf 'fixture-value-for-api-token' >"$work/expected.bin"
  expected="$(sha256sum "$work/expected.bin" | cut -d' ' -f1)"
  bash "$SAFIX_SH" get ana api-token >"$work/got.bin" 2>/dev/null
  got="$(sha256sum "$work/got.bin" | cut -d' ' -f1)"
  [ "$expected" = "$got" ] || fail "get did not round-trip the fixture value"

  # Byte-for-byte, including the absence of a trailing newline: a value stored
  # exactly as typed must come back exactly as stored, and a stream that gained
  # a newline on the way out would still match a line-wise comparison.
  local roundtrip
  roundtrip="$work/roundtrip.bin"
  run_set 'CANARY-round-trip' ana mail-password >/dev/null 2>&1 || fail "the round-trip set failed"
  bash "$SAFIX_SH" get ana mail-password >"$roundtrip" 2>/dev/null
  printf 'CANARY-round-trip' >"$work/roundtrip-expected.bin"
  cmp -s "$roundtrip" "$work/roundtrip-expected.bin" \
    || fail "a value set and read back is not byte-identical (trailing newline?)"

  # Nothing but the value reaches standard output: everything else the command
  # says goes to standard error, so a pipe carries the secret alone.
  [ "$(wc -c <"$roundtrip")" = "$(wc -c <"$work/roundtrip-expected.bin")" ] \
    || fail "get mixed something else into the value stream"

  # A secret shared from another owner resolves to the shared file for the
  # recipient too, so both parties read one file.
  bash "$SAFIX_SH" get ana wifi-psk >"$work/shared.bin" 2>/dev/null
  printf 'fixture-value-for-wifi-psk' >"$work/shared-expected.bin"
  [ "$(sha256sum "$work/shared.bin" | cut -d' ' -f1)" \
    = "$(sha256sum "$work/shared-expected.bin" | cut -d' ' -f1)" ] \
    || fail "get did not round-trip a secret shared from another owner"

  # The default user is $USER when it is a declared user, so a bare name
  # resolves to the same value as the explicit form.
  bash "$SAFIX_SH" get api-token >"$work/default.bin" 2>/dev/null
  [ "$(sha256sum "$work/default.bin" | cut -d' ' -f1)" = "$expected" ] \
    || fail "the default user did not resolve to \$USER"

  local listing
  listing="$(bash "$SAFIX_SH" list ana 2>/dev/null)"
  printf '%s\n' "$listing" | grep -qE "api-token +carries +- +- +api-token +$ANA_FILE" \
    || fail "list does not report api-token against its file"
  # ORIGIN says how the name reached this user and SHARED says whether the entry
  # is one value; a secret granted through sharedWith is `shared` in the first
  # sense and not in the second, so the two columns must disagree here. The cell
  # reads `yes` rather than `shared` for that reason: one word in adjacent
  # columns meaning two things is how an operator misreads a table.
  printf '%s\n' "$listing" | grep -qE "wifi-psk +shared +- +- +wifi-psk +$SHARED_FILE" \
    || fail "list marks a granted secret as a shared entry, or misplaces its file"
  printf '%s\n' "$listing" | grep -qE "aliased-secret +private +- +- +custom-key " \
    || fail "list does not report the key an entry is read under"
  # An entry with no generator reads as `-`, which is the column's whole point:
  # it is what tells an operator whether a valueless name is `generate` or `set`.
  printf '%s\n' "$listing" | grep -qE "^NAME +ORIGIN +SHARED +GENERATOR +KEY +FILE" \
    || fail "list does not head the SHARED and GENERATOR columns"
  if printf '%s\n' "$listing" | grep -qF 'fixture-value-for'; then
    fail "list rendered a value"
  fi

  echo "get-list: OK"
}

# A generator with no inputs mints a value and commits it; one with a prompt
# reads it without echoing and derives from it; one with a dependency runs after
# the generator that writes what it reads, and sees that plaintext down a
# descriptor rather than out of a file; and one with several outputs writes both
# in a single commit even when they land in different files.
test_generate() {
  setup_repo
  make_sops_file "$ANA_FILE" api-token
  make_sops_file "$SHARED_FILE" wifi-psk

  seed_generator seeded "$ANA_FILE" <<'JSON'
{
  "script": "printf '%s\\n' minted-seed",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": [],
  "validation": null, "description": "a fixed seed"
}
JSON
  seed_generator derived "$ANA_FILE" <<'JSON'
{
  "script": "printf 'from-%s\\n' \"$(cat \"$in_seeded\")\"",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": ["seeded"], "files": [],
  "validation": null, "description": null
}
JSON
  seed_generator prompted "$ANA_FILE" <<'JSON'
{
  "script": "printf 'derived-%s\\n' \"$(cat \"$in_pass_phrase\")\"",
  "runtimeInputs": ["coreutils"],
  "prompts": { "pass-phrase": { "type": "hidden", "description": "the fixture passphrase" } },
  "dependencies": [], "files": [],
  "validation": null, "description": null
}
JSON
  # The two halves land in different files, which is what makes the single
  # commit a claim rather than a coincidence.
  seed_generator paired "$ANA_FILE" paired-pub <<'JSON'
{
  "script": "printf '%s' '{\"paired\":\"priv\",\"paired-pub\":\"pub\"}'",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": ["paired-pub"],
  "validation": null, "description": null
}
JSON
  seed_output paired-pub "$SHARED_FILE"

  local before after out
  before="$(git -C "$REPO" rev-parse HEAD)"
  # The passphrase is fed on stdin, which is the branch the command takes with no
  # controlling terminal. There is deliberately no environment variable that
  # would carry it instead.
  out="$(printf '%s\n' fixture-pass | bash "$SAFIX_SH" generate ana 2>&1)" \
    || fail "the bulk generate run failed: $out"

  # Dependency order, decided at evaluation and walked by the command: the
  # generator that reads `seeded` cannot have run before the one that writes it,
  # or `cat "$in_seeded"` would have had nothing to read and `derived` would not
  # hold the value it does.
  [ "$(value_digest "$ANA_FILE" seeded)" = "$(digest_of minted-seed)" ] \
    || fail "the plain generator did not store its output"
  [ "$(value_digest "$ANA_FILE" derived)" = "$(digest_of from-minted-seed)" ] \
    || fail "the dependent generator did not see its dependency's plaintext"
  [ "$(value_digest "$ANA_FILE" prompted)" = "$(digest_of derived-fixture-pass)" ] \
    || fail "the prompted generator did not see the answer it was given"
  [ "$(value_digest "$ANA_FILE" paired)" = "$(digest_of priv)" ] \
    || fail "the multi-output generator did not store its first output"
  [ "$(value_digest "$SHARED_FILE" paired-pub)" = "$(digest_of pub)" ] \
    || fail "the multi-output generator did not store its second output"

  # The prompt is not echoed, and no value reaches the command's own output.
  if printf '%s\n' "$out" | grep -qF 'fixture-pass'; then
    fail "generate rendered a prompt's answer"
  fi
  if printf '%s\n' "$out" | grep -qF 'minted-seed'; then
    fail "generate rendered a value"
  fi

  # One commit per generator, and the multi-output generator's two files in one
  # of them: a keypair split across two commits is a tree holding halves that do
  # not match.
  after="$(git -C "$REPO" rev-parse HEAD)"
  [ "$before" != "$after" ] || fail "generate committed nothing"
  local paired_commit files
  paired_commit="$(git -C "$REPO" log --format=%H --grep='generate paired, paired-pub' -1)"
  [ -n "$paired_commit" ] || fail "no commit names both outputs of the multi-output generator"
  files="$(git -C "$REPO" show --name-only --format= "$paired_commit" | sort | tr '\n' ' ')"
  [ "$files" = "$(printf '%s\n' "$ANA_FILE" "$SHARED_FILE" | sort | tr '\n' ' ')" ] \
    || fail "the multi-output commit carries '$files' rather than exactly its two files"
  git -C "$REPO" log --format=%s | grep -qF 'fixture-pass' && fail "a commit message carries a value"

  # A second bulk run mints nothing: every output already holds a value, and
  # that is the difference --regenerate is for.
  local head_before
  head_before="$(git -C "$REPO" rev-parse HEAD)"
  printf '%s\n' fixture-pass | bash "$SAFIX_SH" generate ana >/dev/null 2>&1
  [ "$(git -C "$REPO" rev-parse HEAD)" = "$head_before" ] \
    || fail "a second bulk run rewrote values that already existed"

  # --regenerate over one name rotates exactly that name. `seeded` is
  # deterministic, so the rotation that proves the point is the untouched
  # neighbour: its ciphertext must come through byte-identical.
  local other_before
  other_before="$(key_digest "$REPO/$ANA_FILE" api-token)"
  seed_generator rotating "$ANA_FILE" <<'JSON'
{
  "script": "printf '%s\\n' rotated",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": [],
  "validation": null, "description": null
}
JSON
  bash "$SAFIX_SH" generate ana rotating >/dev/null 2>&1
  bash "$SAFIX_SH" generate --regenerate ana rotating >/dev/null 2>&1
  [ "$(value_digest "$ANA_FILE" rotating)" = "$(digest_of rotated)" ] \
    || fail "--regenerate did not leave the target holding its generator's output"
  [ "$(key_digest "$REPO/$ANA_FILE" api-token)" = "$other_before" ] \
    || fail "--regenerate disturbed a neighbouring key's ciphertext"

  echo "generate: OK"
}

# Every way a generator's run is refused, and each one for its own reason. None
# may leave a value, a commit, a scratch file, or a partial multi-output write.
test_generate_refusals() {
  setup_repo
  make_sops_file "$ANA_FILE" api-token

  local head_before out
  head_before="$(git -C "$REPO" rev-parse HEAD)"

  # A name with no generator is refused by naming what to do instead, not by a
  # bare failure: the operator's next move differs depending on why the value
  # cannot be minted.
  out="$(bash "$SAFIX_SH" generate ana api-token 2>&1 || true)"
  printf '%s\n' "$out" | grep -qF 'has no generator' \
    || fail "a name with no generator is not refused as such"
  printf '%s\n' "$out" | grep -qF 'safix set ana api-token' \
    || fail "the no-generator refusal does not name the command that sets a value by hand"

  # Empty output. This is the state a truncated write leaves behind, so it may
  # never be stored as though it were a value.
  seed_generator blank "$ANA_FILE" <<'JSON'
{
  "script": "printf '%s' ''",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": [],
  "validation": null, "description": null
}
JSON
  out="$(bash "$SAFIX_SH" generate ana blank 2>&1 || true)"
  printf '%s\n' "$out" | grep -qF 'produced nothing' || fail "an empty generator output is not refused"

  # A non-zero exit from the script itself, which must not be reported as an
  # empty value: the two have different causes and different fixes.
  seed_generator broken "$ANA_FILE" <<'JSON'
{
  "script": "echo diagnostic-on-stderr >&2; exit 3",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": [],
  "validation": null, "description": null
}
JSON
  out="$(bash "$SAFIX_SH" generate ana broken 2>&1 || true)"
  printf '%s\n' "$out" | grep -qF 'exited 3' || fail "a failing generator is not reported by its exit status"
  printf '%s\n' "$out" | grep -qF 'diagnostic-on-stderr' \
    || fail "the generator's own diagnostics did not reach the operator"

  # Validation refuses a candidate the script was happy to produce, and refuses
  # before anything is written.
  seed_generator unvalidated "$ANA_FILE" <<'JSON'
{
  "script": "printf '%s\\n' bad-value",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": [],
  "validation": "grep -q ^good-", "description": null
}
JSON
  out="$(bash "$SAFIX_SH" generate ana unvalidated 2>&1 || true)"
  printf '%s\n' "$out" | grep -qF 'validation' || fail "a value the validation rejects is not refused"

  # A multi-output generator whose script prints the wrong keys writes neither
  # half: a partial keypair is worse than none.
  seed_generator halfpair "$ANA_FILE" halfpair-pub <<'JSON'
{
  "script": "printf '%s' '{\"halfpair\":\"only\"}'",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": ["halfpair-pub"],
  "validation": null, "description": null
}
JSON
  seed_output halfpair-pub "$ANA_FILE"
  out="$(bash "$SAFIX_SH" generate ana halfpair 2>&1 || true)"
  printf '%s\n' "$out" | grep -qF 'declares outputs' \
    || fail "a multi-output generator printing the wrong keys is not refused"

  # Not one of the five wrote, committed, or left anything behind.
  [ "$(git -C "$REPO" rev-parse HEAD)" = "$head_before" ] \
    || fail "a refused generator committed something"
  local strays
  strays="$(find "$REPO/secrets" -name '*safix-tmp*' | wc -l)"
  [ "$strays" = 0 ] || fail "a refused generator left $strays scratch file(s) beside a secrets file"
  local keys
  keys="$(sops-keys-of "$REPO/$ANA_FILE" | jq -r 'keys | join(" ")')"
  [ "$keys" = "api-token" ] || fail "a refused generator left keys '$keys' in the file"

  echo "generate-refusals: OK"
}

# What one generator's process may see of another's, across a bulk run. Both
# claims here are about the boundary between consecutive generators rather than
# about any one of them, so both need a run of several to be visible at all.
#
# The values these generators mint are descriptions of their own process, which
# is the only way to read a claim about a child's environment back out of a run
# that deliberately renders nothing: the description travels the same sops write
# path a secret does and is compared after decryption.
test_generate_isolation() {
  setup_repo
  make_sops_file "$ANA_FILE" api-token

  # `$$` rather than /proc/self, because /proc/self inside `$(readlink ...)` is
  # the readlink process and would describe that instead. Only the numbers are
  # reported: a pipe's inode moves between runs, and the claim is about which
  # descriptors are open and not about what they point at.
  # shellcheck disable=SC2016 # a generator script for another shell to run: $$ and $n must reach it unexpanded
  local fdlist='me=$$; out=; for f in /proc/$me/fd/*; do n=${f##*/}; [ "$n" -ge 3 ] || continue; out="$out$n "; done; printf %s "${out:-none}"'

  # ── a generator that eats standard input ──
  # Ordered before a generator with a prompt, which is the arrangement that
  # tells the two apart: the command's own stdin is where an operator's prompt
  # answers arrive, so a script inheriting it consumes the answer to every
  # prompt after it. The failure is silent rather than loud — a prompt that
  # reads end-of-input looks exactly like one nobody answered — so it has to be
  # asserted on the value the later generator stored, not on an exit status.
  seed_generator aaa-greedy "$ANA_FILE" <<'JSON'
{
  "script": "cat >/dev/null; printf '%s' ate-nothing",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": [],
  "validation": null, "description": null
}
JSON

  # ── the descriptor a generator is not given ──
  # Two probes of the same script, one before the generators that open
  # descriptors and one after. Comparing them rather than asserting a literal
  # set is what keeps the claim about leaked descriptors instead of about bash's
  # own bookkeeping, which contributes the same fds to both.
  seed_generator bbb-probe-first "$ANA_FILE" <<JSON
{
  "script": $(printf '%s' "$fdlist" | jq -Rs .),
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": [],
  "validation": null, "description": null
}
JSON

  # Two dependencies and a prompt, so three descriptors are open at once and all
  # three have to be closed before the next generator starts.
  seed_generator mmm-many "$ANA_FILE" <<'JSON'
{
  "script": "cat \"$in_aaa_greedy\" >/dev/null; cat \"$in_api_token\" >/dev/null; cat \"$in_secret\" >/dev/null; printf '%s' many-ok",
  "runtimeInputs": ["coreutils"],
  "prompts": { "secret": { "type": "hidden", "description": "the fixture passphrase" } },
  "dependencies": ["aaa-greedy", "api-token"], "files": [],
  "validation": null, "description": null
}
JSON

  seed_generator nnn-more "$ANA_FILE" <<'JSON'
{
  "script": "cat \"$in_mmm_many\" >/dev/null; cat \"$in_bbb_probe_first\" >/dev/null; printf '%s' more-ok",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": ["mmm-many", "bbb-probe-first"], "files": [],
  "validation": null, "description": null
}
JSON

  seed_generator zzz-probe-last "$ANA_FILE" <<JSON
{
  "script": $(printf '%s' "$fdlist" | jq -Rs .),
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": [],
  "validation": null, "description": null
}
JSON

  local out
  out="$(printf '%s\n' fixture-pass | bash "$SAFIX_SH" generate ana 2>&1)" \
    || fail "the bulk run failed: $out"

  # The prompt belongs to a generator ordered after one whose script read stdin
  # to end of input. It got its answer anyway, so the generator's stdin is not
  # the command's.
  [ "$(value_digest "$ANA_FILE" mmm-many)" = "$(digest_of many-ok)" ] \
    || fail "a generator ordered after one that consumed stdin did not get its prompt answered"
  [ "$(value_digest "$ANA_FILE" aaa-greedy)" = "$(digest_of ate-nothing)" ] \
    || fail "the stdin-consuming generator did not run"

  # Nothing that ran between them left a descriptor behind. Each of those
  # descriptors carried a decrypted value, so one surviving into a later
  # generator's process is that generator holding plaintext it never declared.
  local first last
  first="$(sops decrypt --extract '["bbb-probe-first"]' "$REPO/$ANA_FILE")"
  last="$(sops decrypt --extract '["zzz-probe-last"]' "$REPO/$ANA_FILE")"
  [ -n "$first" ] || fail "the first probe stored nothing"
  [ "$first" = "$last" ] \
    || fail "a generator running last sees descriptors '$last' where one running first sees '$first'"

  echo "generate-isolation: OK"
}

# Rotating a value every other value was derived from. The cascade is the claim:
# `--regenerate` of a named generator re-runs everything downstream of it, in
# order, after saying which and being told to go ahead.
#
# `base` mints a fresh random value on each run, which is what makes the
# derivation checkable: a downstream value is asserted to be a function of the
# value `base` holds *now*, so a downstream generator that did not re-run holds a
# function of the retired one and fails. That is the defect in the shape it
# actually takes — a hash of a rotated password is indistinguishable from a hash
# of the current one by inspection, and only re-deriving it tells them apart.
test_generate_cascade() {
  setup_repo
  make_sops_file "$ANA_FILE" api-token

  seed_generator base "$ANA_FILE" <<'JSON'
{
  "script": "head -c 18 /dev/urandom | base64 | tr -d '\\n'",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": [],
  "validation": null, "description": "a value everything else derives from"
}
JSON
  seed_generator middle "$ANA_FILE" <<'JSON'
{
  "script": "printf 'mid-%s' \"$(cat \"$in_base\" | sha256sum | cut -d' ' -f1)\"",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": ["base"], "files": [],
  "validation": null, "description": null
}
JSON
  seed_generator leaf "$ANA_FILE" <<'JSON'
{
  "script": "printf 'leaf-%s' \"$(cat \"$in_middle\" | sha256sum | cut -d' ' -f1)\"",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": ["middle"], "files": [],
  "validation": null, "description": null
}
JSON
  # Reads nothing of base's, so it is downstream of nothing and must be left
  # alone. Without it the cascade could be "re-run everything" and still pass.
  seed_generator aside "$ANA_FILE" <<'JSON'
{
  "script": "printf '%s' untouched",
  "runtimeInputs": ["coreutils"],
  "prompts": {}, "dependencies": [], "files": [],
  "validation": null, "description": null
}
JSON

  bash "$SAFIX_SH" generate ana >/dev/null 2>&1 || fail "the first bulk run failed"

  local base_before aside_before head_before out
  base_before="$(value_digest "$ANA_FILE" base)"
  aside_before="$(key_digest "$REPO/$ANA_FILE" aside)"

  # ── declining writes nothing ──
  # Asserted before the accepting run, because a cascade commits as it goes and
  # a decline that had already written could not be told from one that had not.
  head_before="$(git -C "$REPO" rev-parse HEAD)"
  out="$(printf '%s\n' n | bash "$SAFIX_SH" generate --regenerate ana base 2>&1 || true)"
  printf '%s\n' "$out" | grep -qF 'declined' || fail "answering no was not reported as declining: $out"
  [ "$(git -C "$REPO" rev-parse HEAD)" = "$head_before" ] || fail "a declined cascade committed something"
  [ "$(value_digest "$ANA_FILE" base)" = "$base_before" ] || fail "a declined cascade rotated its target"

  # ── the listing names the downstream set, in order, and nothing else ──
  out="$(printf '%s\n' y | bash "$SAFIX_SH" generate --regenerate ana base 2>&1)" \
    || fail "the accepted cascade failed: $out"
  local listed
  listed="$(printf '%s\n' "$out" | sed -n 's/^    \([a-z-]*\)$/\1/p' | tr '\n' ' ')"
  [ "$listed" = "base middle leaf " ] \
    || fail "the cascade listed '$listed' rather than 'base middle leaf ' in dependency order"

  # ── every downstream value is a function of the value base holds now ──
  local base_now mid_want leaf_want
  base_now="$(sops decrypt --extract '["base"]' "$REPO/$ANA_FILE" | sha256sum | cut -d' ' -f1)"
  mid_want="mid-$base_now"
  [ "$(value_digest "$ANA_FILE" base)" != "$base_before" ] || fail "the cascade did not rotate its own target"
  [ "$(value_digest "$ANA_FILE" middle)" = "$(digest_of "$mid_want")" ] \
    || fail "a generator downstream of the rotated value still holds a value derived from the retired one"
  leaf_want="leaf-$(printf '%s' "$mid_want" | sha256sum | cut -d' ' -f1)"
  [ "$(value_digest "$ANA_FILE" leaf)" = "$(digest_of "$leaf_want")" ] \
    || fail "the cascade stopped short of the second generation downstream"

  # ── and nothing else moved ──
  [ "$(key_digest "$REPO/$ANA_FILE" aside)" = "$aside_before" ] \
    || fail "the cascade re-ran a generator that reads nothing of the rotated value"

  # ── --yes answers the confirmation in advance ──
  # Driven with no stdin at all, so a run that still tried to read one fails
  # here rather than passing on an empty answer.
  bash "$SAFIX_SH" generate --regenerate --yes ana base </dev/null >/dev/null 2>&1 \
    || fail "--yes did not answer the cascade confirmation"

  # ── a generator nothing reads is not a cascade and asks nothing ──
  bash "$SAFIX_SH" generate --regenerate ana aside </dev/null >/dev/null 2>&1 \
    || fail "rotating a generator with no dependents asked for a confirmation"

  echo "generate-cascade: OK"
}

# The governed set is the union of what the declarations imply and what the
# consumer named, and the two halves are judged differently because they are
# different claims.
#
# A file named through extraGovernedFiles rides an existing rule and no
# declaration places a secret in it. So its keys are unclaimed by construction
# and must not be reported as findings, while its stanzas are still held to the
# rule that covers it — which is exactly what `fix` will re-wrap it to. Driving
# `fix` from `required` alone would leave such a file behind on every audience
# change, encrypted to whoever it was encrypted to when it was written.
test_governed_extras() {
  setup_repo
  seed_declarations
  make_sops_file "$ANA_FILE" api-token mail-password custom-key

  local extra="secrets/safix/users/ana/ops-tooling.yaml"
  printf '%s\n' "[\"$extra\"]" >"$work/extras.json"

  # Written through the same creation rule ana's own file is, so it agrees with
  # the rule that covers it from the start.
  printf 'shared-tooling-token: "fixture-value-for-tooling"\n' >"$work/extra.yaml"
  (cd "$REPO" && sops encrypt --filename-override "$extra" \
    --input-type yaml --output-type yaml "$work/extra.yaml") >"$REPO/$extra"
  git -C "$REPO" add -- "$extra"
  git -C "$REPO" commit -q -m "fixture: a file that rides ana's rule and no declaration names"

  local out rc=0
  out="$(bash "$SAFIX_SH" check 2>&1)" || rc=$?

  # An extra file in step with its rule is not a finding of any kind. Reporting
  # its keys as unclaimed would be a finding no declaration can ever resolve —
  # not naming them is what naming the file in extraGovernedFiles means.
  if printf '%s\n' "$out" | grep -qF "$extra"; then
    printf '%s\n' "$out" >&2
    fail "a well-formed extra governed file was reported"
  fi
  if printf '%s\n' "$out" | grep -qF 'shared-tooling-token'; then
    fail "an extra file's keys were reported as unclaimed"
  fi

  # Drift it. The rule that covers it is ana's, so an identity outside that rule
  # is drift in exactly the sense a required file's would be.
  local stranger_pub
  stranger_pub="$(age-keygen 2>/dev/null | age-keygen -y /dev/stdin)"
  sops --config /dev/null encrypt --age "$AGE_PUB,$stranger_pub" \
    --input-type yaml --output-type yaml "$work/extra.yaml" >"$REPO/$extra"
  git -C "$REPO" add -- "$extra"
  git -C "$REPO" commit -q -m "fixture: the extra file drifted from the rule that covers it"

  rc=0
  out="$(bash "$SAFIX_SH" check 2>&1)" || rc=$?
  [ "$rc" = 1 ] || fail "check did not report a drifted extra governed file (exit $rc)"
  printf '%s\n' "$out" | grep -qF "$extra is not encrypted to the audience declared for it" \
    || fail "check does not hold an extra file to the rule that covers it: $out"
  printf '%s\n' "$out" | grep -qF "$stranger_pub" \
    || fail "check does not name the identity outside the covering rule"

  # `fix` re-wraps it, which is the whole reason the union exists.
  bash "$SAFIX_SH" fix --yes >/dev/null 2>&1 || fail "fix failed over the governed set"
  if grep -qF "$stranger_pub" "$REPO/$extra"; then
    fail "fix did not re-wrap the consumer-named file, so the union narrowed to the derived half"
  fi

  # A path no rule's directory covers is its own finding: naming a file does not
  # create a rule for it.
  git -C "$REPO" checkout -- "$extra" 2>/dev/null || true
  local unruled="secrets/safix/users/cy/stranded.yaml"
  printf '%s\n' "[\"$unruled\"]" >"$work/extras.json"
  mkdir -p "$REPO/$(dirname "$unruled")"
  sops --config /dev/null encrypt --age "$AGE_PUB" \
    --input-type yaml --output-type yaml "$work/extra.yaml" >"$REPO/$unruled"
  rc=0
  out="$(bash "$SAFIX_SH" check 2>&1)" || rc=$?
  [ "$rc" = 1 ] || fail "check did not report a governed path no rule covers (exit $rc)"
  printf '%s\n' "$out" | grep -qF "no creation rule's directory covers it" \
    || fail "check does not say that naming a file creates no rule for it: $out"

  echo "governed-extras: OK"
}

# --- adduser ----------------------------------------------------------------------
# `adduser` declares a person who holds nothing, so none of the ciphertext
# fixture above is what it acts on: it reads a name alphabet and a hook, writes
# nix, and commits. What is asserted is what an operator cannot check by reading
# the output — that the generated nix parses, that the regenerated policy saw the
# scaffold, that the commit is the scaffold and nothing else, that nothing was
# minted, and that every refusal leaves the tree as it found it.
#
# The generated nix is parsed with the REAL nix-instantiate. A flake evaluation
# is what the sandbox cannot do and the stub stands in for; parsing a file needs
# no store and no daemon, so the claim "the scaffold is valid nix" is made
# against the parser that will read it rather than against a stub.

# A recipient minted in the sandbox. No recipient from anywhere else appears in
# any of this.
new_recipient() { age-keygen 2>/dev/null | age-keygen -y /dev/stdin; }

# The two people a scaffold joins, in the shape `adduser` writes. Tracked,
# because the policy stub reads what git tracks, and carrying the same two keys
# the fixture policy anchors, so that a regenerated policy and the committed one
# agree wherever nothing has been declared since.
seed_declarations() {
  local u pub
  mkdir -p "$REPO/safix/users"
  for u in ana bo; do
    case "$u" in
      ana) pub="$AGE_PUB" ;;
      bo) pub="$BO_PUB" ;;
      *) pub="$(new_recipient)" ;;
    esac
    {
      printf '{\n  flake.safix.users.%s = {\n' "$u"
      printf '    recipient = "%s";\n' "$pub"
      printf '    carries = { };\n    private = { };\n  };\n}\n'
    } >"$REPO/safix/users/$u.nix"
  done
  git -C "$REPO" add -A
  git -C "$REPO" commit -q -m "fixture: two declared people"
}

# No scaffold file and HEAD where it was. Every refusal owes this regardless of
# which check it failed.
assert_untouched() { # <head-before> <what>
  [ "$(git -C "$REPO" rev-parse HEAD)" = "$1" ] || fail "$2 committed something"
  [ ! -e "$REPO/safix/users/cy.nix" ] || fail "$2 left a scaffold behind"
  [ -z "$(git -C "$REPO" status --porcelain)" ] || fail "$2 left the tree dirty"
}

test_adduser() {
  setup_repo
  seed_declarations
  local pub head out
  pub="$(new_recipient)"

  # An unrelated staged change, to show the commit is scoped to the scaffold.
  printf 'bystander\n' >"$REPO/bystander.txt"
  git -C "$REPO" add -- bystander.txt

  out="$(bash "$SAFIX_SH" adduser cy "$pub" --yes 2>&1)" \
    || { printf '%s\n' "$out" >&2; fail "scaffolding a new person failed"; }

  grep -qF "recipient = \"$pub\";" "$REPO/safix/users/cy.nix" \
    || fail "the recipient handed in is not the one recorded"

  # Real nix, on a file the stub never sees.
  nix-instantiate --parse "$REPO/safix/users/cy.nix" >/dev/null \
    || fail "the generated declaration does not parse"

  grep -qF 'carries = { };' "$REPO/safix/users/cy.nix" \
    || fail "the scaffold carries something"
  grep -qF 'private = { };' "$REPO/safix/users/cy.nix" \
    || fail "the scaffold declares something"

  # The policy was regenerated from declarations that already contained cy,
  # which is only true if the scaffold was staged before the evaluation.
  grep -qF -- "- &cy $pub" "$REPO/.sops.yaml" \
    || fail "the regenerated .sops.yaml does not carry the person just declared"
  grep -qF -- '- &ana ' "$REPO/.sops.yaml" \
    || fail "the regenerated .sops.yaml dropped someone who was already declared"

  # A person who holds nothing gets an anchor and no rule: a rule comes from a
  # declaration with a secret in it and from nothing else.
  if grep -qF 'secrets/safix/users/cy/' "$REPO/.sops.yaml"; then
    fail "a person who holds nothing produced a creation rule"
  fi

  # Exactly the scaffold and the policy. Not the bystander.
  git -C "$REPO" show --name-only --format= HEAD | grep -v '^$' | sort >"$work/committed"
  printf '%s\n' \
    .sops.yaml \
    safix/users/cy.nix \
    | sort >"$work/expected"
  cmp -s "$work/committed" "$work/expected" \
    || fail "the commit is not exactly the scaffold and the regenerated policy: $(tr '\n' ' ' <"$work/committed")"

  git -C "$REPO" diff --cached --quiet -- bystander.txt \
    && fail "the bystander was swept into the commit"
  [ -z "$(git -C "$REPO" status --porcelain -- safix/users)" ] \
    || fail "part of the scaffold was left uncommitted"

  # No key material anywhere: the recipient is public and is the only key-shaped
  # string the run may have written.
  ! grep -rq 'AGE-SECRET-KEY' "$REPO" || fail "a private key reached the tree"

  # The output says what it did and what it did not, and names the sequence that
  # gives them their first secret.
  printf '%s\n' "$out" | grep -qF 'safix/users/cy.nix' \
    || fail "the output does not name the file it wrote"
  printf '%s\n' "$out" | grep -qF 'no key was minted' \
    || fail "the output does not say that nothing was minted"
  printf '%s\n' "$out" | grep -qF 'onboardingHook is unset' \
    || fail "the output does not say that no hook ran"
  printf '%s\n' "$out" | grep -qF 'safix fix' \
    || fail "the output does not name the command that writes their first rule"
  printf '%s\n' "$out" | grep -qF 'safix set cy' \
    || fail "the output does not name the command that gives them their first value"

  head="$(git -C "$REPO" rev-parse HEAD)"
  bash "$SAFIX_SH" adduser cy "$pub" --yes >/dev/null 2>&1 \
    && fail "scaffolding the same person twice was accepted"
  [ "$(git -C "$REPO" rev-parse HEAD)" = "$head" ] \
    || fail "the refusal to redeclare committed something"

  echo "adduser: OK"
}

test_adduser_refusals() {
  setup_repo
  seed_declarations
  local pub head
  pub="$(new_recipient)"
  head="$(git -C "$REPO" rev-parse HEAD)"

  # A name outside the alphabet. The refusal has to happen here: the name is not
  # a declared user yet, so no resolver check can reach it, and the commit that
  # would make it reachable is the one being refused.
  bash "$SAFIX_SH" adduser Cy "$pub" --yes >/dev/null 2>&1 \
    && fail "an uppercase name was accepted"
  assert_untouched "$head" "an uppercase name"
  bash "$SAFIX_SH" adduser 'cy/../root' "$pub" --yes >/dev/null 2>&1 \
    && fail "a name containing a path separator was accepted"
  assert_untouched "$head" "a name containing a path separator"
  bash "$SAFIX_SH" adduser '-cy' "$pub" --yes >/dev/null 2>&1 \
    && fail "a name starting outside the alphabet was accepted"

  # A recipient that is not one.
  bash "$SAFIX_SH" adduser cy 'not-an-age-key' --yes >/dev/null 2>&1 \
    && fail "a malformed recipient was accepted"
  assert_untouched "$head" "a malformed recipient"
  bash "$SAFIX_SH" adduser cy "${pub}extra" --yes >/dev/null 2>&1 \
    && fail "an over-long recipient was accepted"
  assert_untouched "$head" "an over-long recipient"

  # A hardware recipient, refused for what it cannot do rather than for its
  # shape: it is a well-formed recipient and activation still cannot use it.
  #
  # Synthetic, and only the `age1yubikey1` prefix is load-bearing — the refusal
  # fires on that and never reaches the bech32 check, so no plausible-looking
  # suffix is needed and none is used. A recipient copied from a real card would
  # name a real device.
  bash "$SAFIX_SH" adduser cy \
    age1yubikey1fixture000000000000000000000000000000000000000000000000000 \
    --yes >/dev/null 2>&1 \
    && fail "a recipient requiring a physical interaction was accepted as the primary one"
  assert_untouched "$head" "a hardware recipient"
  bash "$SAFIX_SH" adduser cy \
    age1yubikey1fixture000000000000000000000000000000000000000000000000000 \
    --yes >"$work/refusal" 2>&1 || true
  grep -qF 'recoveryRecipients' "$work/refusal" \
    || fail "the hardware-recipient refusal does not name where a card does belong"

  # An existing person.
  bash "$SAFIX_SH" adduser ana "$pub" --yes >/dev/null 2>&1 \
    && fail "redeclaring an existing person was accepted"
  assert_untouched "$head" "redeclaring an existing person"

  echo "adduser-refusals: OK"
}

# Host attachment is a consumer's, and reaches it through the hook or not at all.
# Two claims: --host with no hook is refused with the reason rather than silently
# ignored, and a configured hook receives the name, the recipient and the hosts
# after safix's own commit has landed — so that whatever it writes is its own to
# stage and safix's single-intent commit stays one.
test_adduser_hook() {
  setup_repo
  seed_declarations
  local pub head out
  pub="$(new_recipient)"
  head="$(git -C "$REPO" rev-parse HEAD)"

  out="$(bash "$SAFIX_SH" adduser cy "$pub" --host somebox --yes 2>&1 || true)"
  printf '%s\n' "$out" | grep -qF 'onboardingHook is unset' \
    || fail "--host without a hook is not refused by naming the hook: $out"
  printf '%s\n' "$out" | grep -qF 'onboarding without it succeeds' \
    || fail "the refusal does not say that no hook is a supported configuration"
  assert_untouched "$head" "--host with no hook configured"

  # A hook that records what it was handed. It writes into the repository
  # without staging anything, which is what lets the claim below distinguish
  # "ran after the commit" from "ran before it".
  jq -Rs . >"$work/hook.json" <<'HOOK'
{
  printf 'name=%s\n' "$1"
  printf 'recipient=%s\n' "$2"
  shift 2
  for host in "$@"; do printf 'host=%s\n' "$host"; done
} >hook-log.txt
HOOK

  out="$(bash "$SAFIX_SH" adduser cy "$pub" --host somebox --host otherbox --yes 2>&1)" \
    || { printf '%s\n' "$out" >&2; fail "onboarding with a hook failed"; }

  [ -f "$REPO/hook-log.txt" ] || fail "the hook did not run"
  grep -qxF 'name=cy' "$REPO/hook-log.txt" || fail "the hook was not given the new person's name"
  grep -qxF "recipient=$pub" "$REPO/hook-log.txt" || fail "the hook was not given the recipient"
  grep -qxF 'host=somebox' "$REPO/hook-log.txt" || fail "the hook was not given the first --host"
  grep -qxF 'host=otherbox' "$REPO/hook-log.txt" || fail "the hook was not given the second --host"

  # safix's commit is still exactly its own scaffolding, and the hook's output
  # is uncommitted: the package makes no assumption about what the hook does, so
  # it cannot claim its work in a message naming only what safix did.
  git -C "$REPO" show --name-only --format= HEAD | grep -v '^$' | sort >"$work/committed"
  printf '%s\n' .sops.yaml safix/users/cy.nix | sort >"$work/expected"
  cmp -s "$work/committed" "$work/expected" \
    || fail "safix's commit carried the hook's work: $(tr '\n' ' ' <"$work/committed")"
  git -C "$REPO" status --porcelain | grep -qF 'hook-log.txt' \
    || fail "the hook's output was not left uncommitted"

  echo "adduser-hook: OK"
}

# One value, one file, one key, for every carrier. Written by one of them and
# read back by the other, because that round trip is the whole of what `shared`
# promises and the only form of the claim a placement that agreed with itself
# but not with the ciphertext could not satisfy.
test_shared_placement() {
  setup_repo
  seed_shared fleet-token "$SHARED_FILE"
  make_sops_file "$SHARED_FILE" wifi-psk

  local ana_row bo_row
  ana_row="$(bash "$SAFIX_SH" list ana 2>/dev/null | grep -E '^fleet-token ')"
  bo_row="$(bash "$SAFIX_SH" list bo 2>/dev/null | grep -E '^fleet-token ')"
  printf '%s\n' "$ana_row" | grep -qE "fleet-token +carries +yes +- +fleet-token +$SHARED_FILE" \
    || fail "ana's row does not mark fleet-token shared against the audience file: $ana_row"
  printf '%s\n' "$bo_row" | grep -qE "fleet-token +carries +yes +- +fleet-token +$SHARED_FILE" \
    || fail "bo's row does not mark fleet-token shared against the audience file: $bo_row"

  local rc=0 out
  out="$(run_set 'CANARY-one-value-for-both' ana fleet-token 2>&1)" || rc=$?
  [ "$rc" = 0 ] || { printf '%s\n' "$out" >&2; fail "a carrier could not set the shared value"; }

  # bo's placement resolves the file ana wrote, so bo reads what ana minted. A
  # per-carrier copy would leave this reading nothing at all.
  bash "$SAFIX_SH" get bo fleet-token >"$work/bo.bin" 2>/dev/null \
    || fail "bo cannot read the value his fellow carrier minted"
  printf 'CANARY-one-value-for-both' >"$work/want.bin"
  [ "$(sha256sum "$work/bo.bin" | cut -d' ' -f1)" \
    = "$(sha256sum "$work/want.bin" | cut -d' ' -f1)" ] \
    || fail "the two carriers do not read one value"

  # And exactly one file holds it: a second copy anywhere is the defect the
  # check below exists to report, and must not be what a plain `set` creates.
  local holders
  holders="$(git -C "$REPO" ls-files -- 'secrets/**/*.yaml' \
    | while IFS= read -r f; do
        grep -qE '^fleet-token:' "$REPO/$f" && printf '%s\n' "$f"
      done | wc -l)"
  [ "$holders" = 1 ] || fail "$holders files hold the shared key, not 1"

  echo "shared-placement: OK"
}

# A carrier dropped from a shared entry is a revocation, and the signal is the
# ciphertext rather than a record of what the audience used to be: the file that
# still holds the value is encrypted to someone the declared audience no longer
# names. `check` must say so and must not offer `fix` as the remedy.
test_shared_shrink() {
  setup_repo
  seed_shared fleet-token "$SHARED_FILE"
  make_sops_file "$SHARED_FILE" wifi-psk fleet-token
  make_sops_file "$ANA_FILE" api-token mail-password

  unshare_from fleet-token ana "$ANA_FILE"

  local out rc=0
  out="$(bash "$SAFIX_SH" check 2>&1)" || rc=$?
  [ "$rc" = 1 ] || fail "check did not report the revocation (exit $rc)"

  printf '%s\n' "$out" | grep -qF 'This is a revocation.' \
    || fail "check does not call a dropped carrier a revocation: $out"
  printf '%s\n' "$out" | grep -qF "$SHARED_FILE still holds a value under 'fleet-token'" \
    || fail "check does not name the file the revoked copy is in"
  # Named, not printed as a key. An operator reading an age public key has to go
  # and look up whose it is, which is the moment a revocation is misjudged.
  printf '%s\n' "$out" | grep -qE '^ *- bo$' \
    || fail "check does not name bo as the reader outside the audience: $out"
  printf '%s\n' "$out" | grep -qF "$BO_PUB" \
    && fail "check printed a raw recipient key instead of the person's name"

  # The remedy is a new value, and it is the `set` form because this entry has no
  # generator. `fix` may appear only as the last convergence step, never as the
  # answer: re-wrapping a data key bo has already held revokes nothing.
  printf '%s\n' "$out" | grep -qF 'safix set ana fleet-token' \
    || fail "check does not offer minting a new value as the remedy: $out"
  printf '%s\n' "$out" | grep -qF 'fix is not the remedy' \
    || fail "check does not say that fix will not revoke: $out"

  # Reported once. The stray is an unclaimed value too, and the two remedies
  # disagree — one says delete it, the other says declare it.
  [ "$(printf '%s\n' "$out" | grep -cF "and no declaration claims it")" = 0 ] \
    || fail "the revoked copy was reported a second time as an unclaimed value"

  echo "shared-shrink: OK"
}

# Flipping an entry to shared over values that are already there. Every reader of
# the copy left behind is still in the audience, so nothing has been disclosed;
# what is wrong is that the audience's own file holds no value and the per-carrier
# copies can disagree with each other. The tool must not pick which one wins.
test_shared_flip() {
  setup_repo
  seed_shared fleet-token "$SHARED_FILE"
  make_sops_file "$SHARED_FILE" wifi-psk
  make_sops_file "$ANA_FILE" api-token fleet-token

  local out rc=0
  out="$(bash "$SAFIX_SH" check 2>&1)" || rc=$?
  [ "$rc" = 1 ] || fail "check did not report the pending migration (exit $rc)"

  printf '%s\n' "$out" | grep -qF "$ANA_FILE holds a value under 'fleet-token' of its own" \
    || fail "check does not name the per-carrier copy: $out"
  printf '%s\n' "$out" | grep -qF 'migration rather than a disclosure' \
    || fail "check does not distinguish a migration from a revocation: $out"
  printf '%s\n' "$out" | grep -qF 'This is a revocation.' \
    && fail "check called a migration a revocation"
  printf '%s\n' "$out" | grep -qF 'Which one should win is yours to say' \
    || fail "check does not leave the choice of value to the operator: $out"
  printf '%s\n' "$out" | grep -qF 'safix set ana fleet-token' \
    || fail "check does not say how to mint the value the audience will share"

  # The audience's own file is reported empty as well: the migration is not done
  # until a value is there, and that finding is what says so for each carrier.
  printf '%s\n' "$out" | grep -qF "flake.safix.users.bo declares 'fleet-token' and $SHARED_FILE holds no value" \
    || fail "check does not report the audience file as valueless for every carrier"

  echo "shared-flip: OK"
}

case "$mode" in
  set-new) test_set_new ;;
  set-existing) test_set_existing ;;
  refusals) test_refusals ;;
  recipient-drift) test_recipient_drift ;;
  staged-bystander) test_staged_bystander ;;
  abort) test_abort ;;
  get-list) test_get_list ;;
  generate) test_generate ;;
  generate-refusals) test_generate_refusals ;;
  generate-isolation) test_generate_isolation ;;
  generate-cascade) test_generate_cascade ;;
  governed-extras) test_governed_extras ;;
  adduser) test_adduser ;;
  adduser-refusals) test_adduser_refusals ;;
  adduser-hook) test_adduser_hook ;;
  shared-placement) test_shared_placement ;;
  shared-shrink) test_shared_shrink ;;
  shared-flip) test_shared_flip ;;
  *) fail "unknown mode" ;;
esac
