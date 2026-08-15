#!/usr/bin/env bash
# safix-differential.sh — the shell runtime as the oracle, the rust runtime as
# the subject, one fixture fleet, four channels.
#
#   SAFIX_SH=/path/to/safix.sh SAFIX_RS=/path/to/safix-rs \
#     safix-differential.sh <mode>
#
# Read-path modes:  clean missing drift orphan unknown norule
# Write-path modes: write refuse guard converge abort pipes
# Self-drill mode:  drills
#
# The gate that permits retiring the shell runtime is not a claim that the two
# agree; it is this, running. Each mode builds one fixture repository, gives
# each runtime its own pristine copy of it, hands both an identical argv, and
# compares:
#
#   stdout   byte for byte, with no normalization at all. This is the
#            machine-readable channel — a value from `get`, a table from `list`
#            — and a difference here is a defect with no argument available.
#   stderr   byte for byte, with SAFIX_ERROR_FORMAT=plain set on the rust side
#            only. The plain reporter is code rather than a comparison rule: a
#            regular expression normalizing miette's graphical rendering would
#            be a comparison whose strictness nobody could state.
#   status   exactly.
#            One substitution is applied before this comparison and it is the
#            only normalization in the harness: `set` prints the abbreviated
#            object name of the commit it just made, and two correct runs cannot
#            print the same one — a value written now takes a fresh
#            initialization vector, so the two trees differ and so do the two
#            commits. Each side's own `git rev-parse --short HEAD` is what is
#            substituted, per side, and anything left looking like an
#            abbreviated object name afterwards fails the comparison. So a
#            runtime that named someone else's commit, an older commit, or no
#            commit where the other named one is still caught; what is given up
#            is only the ability to compare two hashes that cannot be equal.
#   effects  through one projection applied to both sides — ordered commits with
#            their per-path status, the full porcelain status, the tree's paths
#            and modes, every governed file's decrypted plaintext, and every
#            governed file's recipients. Not the ciphertext bytes: a newly
#            written value takes a fresh IV and moves the MAC and `lastmodified`
#            with it, so comparing bytes would compare sops' random number
#            generator.
#
# Three assertions sit beside them. After both runs, neither runtime's own
# temporary directory holds any fixture value; neither repository holds a
# candidate document left beside its target, which the effect projection cannot
# catch on its own because two runtimes that both leave one agree about it; and
# for `set`, neither runtime disturbed a key in the file that it was not asked
# to set.
#
# ── what is real here and what is not ──
# The age identities are minted in this run's scratch directory and exist for
# the length of it. sops, age and git are the real binaries; only `nix` is
# stubbed, because a flake evaluation is what a build sandbox cannot do, and the
# stub asserts the attribute each call names so that a rename fails a check
# rather than an operator's terminal. Every value written is set through the
# shell runtime itself, so the fixture is what the oracle produces rather than
# what this script thinks the oracle produces.
#
# Both runtimes are given every fixture identity. The claim that `check` needs
# none is the shell self-test's and is not restated here; what this compares is
# two runtimes over one repository, and giving them different key material would
# make a difference in output ambiguous between a defect and a fixture.
set -euo pipefail

mode="${1:?usage: safix-differential.sh <clean|missing|drift|orphan|unknown|norule|drills>}"
: "${SAFIX_SH:?SAFIX_SH must point at safix.sh}"
: "${SAFIX_RS:?SAFIX_RS must point at the rust binary}"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

BASH_BIN="$(command -v bash)"
REAL_SOPS="$(command -v sops)"

# What both runtimes read on standard input, and anything else both are given.
# Globals rather than parameters because `compare` already takes a label and an
# argv, and threading two more through every call site would bury the argv.
COMPARE_INPUT=""
EXTRA_ENV=()

# The lines an invocation is fed, each terminated. `set` reads two lines, so a
# feed built with a command substitution would lose the trailing newline and
# turn every comparison into the one about a confirmation that never arrived.
with_input() { # <line>...
  COMPARE_INPUT="$work/input"
  : >"$COMPARE_INPUT"
  local line
  for line in "$@"; do printf '%s\n' "$line" >>"$COMPARE_INPUT"; done
}

# A feed with no terminating newline, which is how a stream that ends mid-value
# is spelled.
with_unterminated_input() { # <text>
  COMPARE_INPUT="$work/input"
  printf '%s' "$1" >"$COMPARE_INPUT"
}

no_input() { COMPARE_INPUT=""; }

input_path() { printf '%s' "${COMPARE_INPUT:-/dev/null}"; }

# The feed reaches both runtimes down a PIPE and never as a seekable file, and
# that is a fixture decision with a finding behind it.
#
# `safix.sh` reads its two entries with `read ... </dev/stdin`, which re-opens
# standard input for each read. Re-opening a pipe yields another handle on the
# same stream, so the second read gets the second line; re-opening a regular file
# yields a fresh description at offset zero, so the second read gets the FIRST
# line again — the confirmation compares equal to the value whatever was typed on
# the second line, and the double entry stops checking anything. A harness that
# fed a file would be comparing the two runtimes over an oracle behaviour no
# operator can reach, and would report the rust runtime's sequential read as the
# divergence.
#
# A pipe is also the only non-terminal case an operator reaches, and it is how
# this script's own fixture builder drives the oracle.

fail() {
  printf 'differential[%s]: %s\n' "$mode" "$1" >&2
  exit 1
}

note() { printf 'differential[%s]: %s\n' "$mode" "$1" >&2; }

# --- The fleet ------------------------------------------------------------------
# ana, bo and cy, matching the fleet this repository declares into its own
# `flake.safix.*`, so that what the harness drives and what the structural checks
# judge are the same three people.
ANA_FILE="secrets/safix/users/ana/secrets.yaml"
BO_FILE="secrets/safix/users/bo/secrets.yaml"
SHARED_FILE="secrets/safix/shared/ana,bo/secrets.yaml"
EXTRA_FILE="secrets/elsewhere/notes.yaml"

mint() { # <name> -> public half on stdout, private half at $work/<name>.txt
  age-keygen -o "$work/$1.txt" 2>/dev/null
  age-keygen -y "$work/$1.txt"
}

setup_keys() {
  ANA_PUB="$(mint ana)"
  ESCROW_PUB="$(mint escrow)"
  BO_PUB="$(mint bo)"
  CY_PUB="$(mint cy)"
  cat "$work/ana.txt" "$work/escrow.txt" "$work/bo.txt" "$work/cy.txt" >"$work/keys.txt"
}

# The four JSON attributes and the policy text the stubbed `nix` serves.
#
# Written as files rather than computed in the stub so that a mode can perturb
# one of them — a governed file no rule covers, a placement outside `*.yaml` —
# without the stub growing a mode of its own.
write_fixture() {
  mkdir -p "$work/fixture"

  jq -n \
    --arg ana_file "$ANA_FILE" --arg bo_file "$BO_FILE" --arg shared "$SHARED_FILE" \
    '{
      ana: {
        "ana-alone":    { file: $ana_file, key: "ana_alone", origin: "private", owner: "ana", shared: false, generator: null },
        "api-token":    { file: $ana_file, key: "api-token", origin: "private", owner: "ana", shared: false,
                          generator: { dependencies: [], description: "a token minted from nothing", files: [],
                                       prompts: {}, runtimeInputs: ["coreutils"], script: "printf %s fixture", validation: null } },
        "ops-handover": { file: $shared,   key: "ops-handover", origin: "private", owner: "ana", shared: false, generator: null },
        "ops-tooling":  { file: $ana_file, key: "ops_tooling", origin: "carries", owner: "ana", shared: false, generator: null },
        "team-vault":   { file: $shared,   key: "team-vault", origin: "carries", owner: "ana", shared: true, generator: null }
      },
      bo: {
        "bo-service":   { file: $bo_file,  key: "bo-service", origin: "private", owner: "bo", shared: false, generator: null },
        "ops-handover": { file: $shared,   key: "ops-handover", origin: "shared", owner: "ana", shared: false, generator: null },
        "ops-tooling":  { file: $bo_file,  key: "ops_tooling", origin: "carries", owner: "bo", shared: false, generator: null },
        "team-vault":   { file: $shared,   key: "team-vault", origin: "carries", owner: "bo", shared: true, generator: null }
      },
      cy: {}
    }' | jq -S . >"$work/fixture/placements.json"

  jq -n \
    --arg ana_file "$ANA_FILE" --arg bo_file "$BO_FILE" --arg shared "$SHARED_FILE" \
    --arg ana "$ANA_PUB" --arg escrow "$ESCROW_PUB" --arg bo "$BO_PUB" \
    '{
      ($shared):   { audience: ["ana","bo"], dir: ($shared | sub("/[^/]*$"; "")),   recipients: [$escrow, $ana, $bo] },
      ($ana_file): { audience: ["ana"],      dir: ($ana_file | sub("/[^/]*$"; "")), recipients: [$escrow, $ana] },
      ($bo_file):  { audience: ["bo"],       dir: ($bo_file | sub("/[^/]*$"; "")),  recipients: [$bo] }
    }' | jq -S . >"$work/fixture/audiences.json"

  jq -n --arg ana_file "$ANA_FILE" --arg bo_file "$BO_FILE" --arg shared "$SHARED_FILE" \
    '{ extra: [], managed: [$shared, $ana_file, $bo_file], required: [$shared, $ana_file, $bo_file] }' \
    >"$work/fixture/governed.json"

  jq -n --arg ana "$ANA_PUB" --arg escrow "$ESCROW_PUB" --arg bo "$BO_PUB" --arg cy "$CY_PUB" \
    '{ ana: [$escrow, $ana], bo: [$bo], cy: [$cy] }' | jq -S . >"$work/fixture/recipients.json"

  write_policy
  assert_attribute_order
}

# The recipient policy, in the shape the generator renders: anchors for every
# declared key, one rule per audience directory, start-anchored and terminating
# on the extension, and no catch-all.
write_policy() {
  {
    printf 'keys:\n'
    printf '  - &ana %s\n' "$ANA_PUB"
    printf '  - &ana-escrow %s\n' "$ESCROW_PUB"
    printf '  - &bo %s\n' "$BO_PUB"
    printf '  - &cy %s\n' "$CY_PUB"
    printf 'creation_rules:\n'
    printf '  - path_regex: ^%s/[^/]*\\.yaml$\n' "$(dirname "$SHARED_FILE")"
    printf '    key_groups:\n      - age:\n'
    printf '          - *ana-escrow\n          - *ana\n          - *bo\n'
    printf '  - path_regex: ^%s/[^/]*\\.yaml$\n' "$(dirname "$ANA_FILE")"
    printf '    key_groups:\n      - age:\n'
    printf '          - *ana-escrow\n          - *ana\n'
    printf '  - path_regex: ^%s/[^/]*\\.yaml$\n' "$(dirname "$BO_FILE")"
    printf '    key_groups:\n      - age:\n'
    printf '          - *bo\n'
  } >"$work/fixture/policy.yaml"
}

emit_stub() { # <path> <body-on-stdin>
  {
    printf '#!%s\n' "$BASH_BIN"
    cat
  } >"$1"
  chmod +x "$1"
}

# `nix` is the only stubbed binary. It asserts the flake reference it is handed
# is the repository the caller named and dispatches on the attribute, so a
# rename of `flake.safix.lib.placements` fails here rather than silently serving
# the wrong document to one runtime.
write_stub_nix() {
  mkdir -p "$work/bin"
  emit_stub "$work/bin/nix" <<'SH'
set -eu
[ "${1:-}" = eval ] || { echo "stub nix: expected eval, got '${1:-}'" >&2; exit 1; }
shift
format="${1:-}"; shift
target="${1:-}"
root="${target%#*}"
attribute="${target#*#}"
[ "$root" = "$SAFIX_REPO_ROOT" ] \
  || { echo "stub nix: '$root' is not the repository under test" >&2; exit 1; }
case "$format:$attribute" in
  --json:safix.lib.placements)    cat "$SAFIX_FIXTURE/placements.json" ;;
  --json:safix.lib.audiences)     cat "$SAFIX_FIXTURE/audiences.json" ;;
  --json:safix.lib.governedFiles) cat "$SAFIX_FIXTURE/governed.json" ;;
  --json:safix.lib.recipients)    cat "$SAFIX_FIXTURE/recipients.json" ;;
  --raw:safix.lib.policyText)     cat "$SAFIX_FIXTURE/policy.yaml" ;;
  *) echo "stub nix: unexpected invocation: $format $attribute" >&2; exit 1 ;;
esac
SH
}

# --- The repository ---------------------------------------------------------------
setup_repo() {
  REPO="$work/repo"
  mkdir -p "$REPO"
  cp "$work/fixture/policy.yaml" "$REPO/.sops.yaml"
  git -C "$REPO" init -q
  git -C "$REPO" config user.email differential@example.invalid
  git -C "$REPO" config user.name differential
  git -C "$REPO" add -A
  git -C "$REPO" commit -q -m "fixture: recipient policy"
}

# The environment both runtimes are given, but for the reporter and the paths
# that differ per side. An array rather than a string, so a scratch directory
# whose path contains a space reaches the child as one assignment rather than as
# two.
RUNTIME_ENV=()
runtime_env() { # <repo> <tmpdir>
  RUNTIME_ENV=(
    "PATH=$work/bin:$PATH"
    "HOME=$work/home"
    "TMPDIR=$2"
    "USER=ana"
    "SAFIX_NIX=$work/bin/nix"
    "SAFIX_FIXTURE=$work/fixture"
    "SAFIX_REPO_ROOT=$1"
    "SOPS_AGE_KEY_FILE=$work/keys.txt"
  )
  if [ "${#EXTRA_ENV[@]}" -gt 0 ]; then
    RUNTIME_ENV+=("${EXTRA_ENV[@]}")
  fi
}

# Set one value through the shell runtime, which is what makes the fixture the
# oracle's own output rather than this script's idea of it.
set_value() { # <user> <name> <value>
  mkdir -p "$work/home" "$work/settmp"
  runtime_env "$REPO" "$work/settmp"
  printf '%s\n%s\n' "$3" "$3" \
    | env "${RUNTIME_ENV[@]}" bash "$SAFIX_SH" set "$1" "$2" >/dev/null 2>&1 \
    || fail "the oracle refused to set '\''$2'\'' for $1 while building the fixture"
}

seed_values() {
  set_value ana ana-alone value-ana-alone
  set_value ana api-token value-api-token
  set_value ana ops-tooling value-ana-ops-tooling
  set_value ana ops-handover value-ops-handover
  set_value ana team-vault value-team-vault
  set_value bo bo-service value-bo-service
  set_value bo ops-tooling value-bo-ops-tooling
}

# --- The projection ---------------------------------------------------------------
# One program, both sides. The reason it is a projection rather than a byte
# comparison is sops' own: a value written now takes a fresh initialization
# vector, and the message authentication code and `lastmodified` move with it, so
# two correct runs leave files that differ byte for byte and agree on everything
# that matters.
project() { # <repo>
  local repo="$1" file
  printf '=== commits ===\n'
  git -C "$repo" log --reverse --format='commit %s' --name-status
  printf '=== status ===\n'
  git -C "$repo" status --porcelain=v2 --untracked-files=all
  printf '=== tree ===\n'
  (cd "$repo" && find . -path ./.git -prune -o -printf '%M %p\n' -print0 2>/dev/null | tr -d '\0') \
    | LC_ALL=C sort
  printf '=== values ===\n'
  while IFS= read -r file; do
    [ -e "$repo/$file" ] || continue
    printf '%s ' "$file"
    if SOPS_AGE_KEY_FILE="$work/keys.txt" sops decrypt --output-type json "$repo/$file" 2>/dev/null; then
      :
    else
      printf '<no identity opens it>\n'
    fi
  done < <(governed_paths)
  printf '=== recipients ===\n'
  while IFS= read -r file; do
    [ -e "$repo/$file" ] || continue
    printf '%s %s\n' "$file" "$(sops-recipients-of "$repo/$file" "$work/empty.json" | jq -c '.actual')"
  done < <(governed_paths)
}

governed_paths() { jq -r '.managed[]' "$work/fixture/governed.json"; }

# --- The comparison ---------------------------------------------------------------
COMPARED=0
rs_status_last=0

compare() { # <label> <argument>...
  local label="$1"
  shift
  local sh="$work/run/sh" rs="$work/run/rs" sh_status=0 rs_status=0

  rm -rf "$work/run"
  mkdir -p "$sh/tmp" "$rs/tmp" "$work/home"
  cp -a "$REPO" "$sh/repo"
  cp -a "$REPO" "$rs/repo"

  local input
  input="$(input_path)"

  runtime_env "$sh/repo" "$sh/tmp"
  env "${RUNTIME_ENV[@]}" \
    bash "$SAFIX_SH" "$@" < <(cat "$input") >"$sh/out" 2>"$sh/err" || sh_status=$?
  runtime_env "$rs/repo" "$rs/tmp"
  env "${RUNTIME_ENV[@]}" SAFIX_ERROR_FORMAT=plain \
    "$SAFIX_RS" "$@" < <(cat "$input") >"$rs/out" 2>"$rs/err" || rs_status=$?
  rs_status_last="$rs_status"

  normalize_run "$sh/repo" "$sh/out" "$sh/err"
  normalize_run "$rs/repo" "$rs/out" "$rs/err"

  cmp -s "$sh/out" "$rs/out" || {
    printf '%s\n' '--- shell stdout ---' >&2
    cat "$sh/out" >&2
    printf '%s\n' '--- rust stdout ---' >&2
    cat "$rs/out" >&2
    fail "stdout differs for [$label]: safix $*"
  }

  cmp -s "$sh/err" "$rs/err" || {
    printf '%s\n' '--- shell stderr ---' >&2
    cat "$sh/err" >&2
    printf '%s\n' '--- rust stderr ---' >&2
    cat "$rs/err" >&2
    fail "stderr differs for [$label]: safix $*"
  }

  [ "$sh_status" = "$rs_status" ] \
    || fail "exit status differs for [$label]: safix $* — shell $sh_status, rust $rs_status"

  project "$sh/repo" >"$sh/projection"
  project "$rs/repo" >"$rs/projection"
  cmp -s "$sh/projection" "$rs/projection" || {
    diff -u "$sh/projection" "$rs/projection" >&2 || true
    fail "repository effects differ for [$label]: safix $*"
  }

  residue_free "$sh/tmp" "shell" "$label"
  residue_free "$rs/tmp" "rust" "$label"
  no_scratch_left "$sh/repo" "shell" "$label"
  no_scratch_left "$rs/repo" "rust" "$label"

  reporter_changes_stderr_alone "$label" "$@"

  COMPARED=$((COMPARED + 1))
}

# Selecting a reporter is allowed to change the bytes on standard error and
# nothing else. Without this the plain reporter would be a hole in the
# comparison rather than a rendering of it: a binary that behaved differently
# when the harness was watching would pass every other assertion here.
#
# The run is the same invocation with the variable unset, compared against the
# run just made on standard output, exit status and repository effects — every
# channel but the one the variable exists to change.
reporter_changes_stderr_alone() { # <label> <argument>...
  local label="$1"
  shift
  local plain="$work/run/rs" fancy="$work/run/rs-graphical" status=0

  rm -rf "$fancy"
  mkdir -p "$fancy/tmp"
  cp -a "$REPO" "$fancy/repo"
  runtime_env "$fancy/repo" "$fancy/tmp"
  env "${RUNTIME_ENV[@]}" "$SAFIX_RS" "$@" < <(cat "$(input_path)") \
    >"$fancy/out" 2>"$fancy/err" || status=$?
  # Standard output alone: the graphical rendering of standard error is the one
  # channel this harness does not compare — it is pinned by the command's own
  # snapshots instead — and its line wrapping breaks a path across two lines,
  # which a substitution over whole paths cannot see.
  normalize_run "$fancy/repo" "$fancy/out"

  cmp -s "$plain/out" "$fancy/out" \
    || fail "selecting a reporter changed standard output for [$label]: safix $*"
  [ "$status" = "$rs_status_last" ] \
    || fail "selecting a reporter changed the exit status for [$label]: safix $*"
  project "$fancy/repo" >"$fancy/projection"
  cmp -s "$plain/projection" "$fancy/projection" \
    || fail "selecting a reporter changed the repository for [$label]: safix $*"
}

# What the oracle has to be saying for a comparison of it to mean anything.
#
# Two runtimes that both print nothing agree perfectly, so every perturbed mode
# asserts that the shell runtime does produce the finding the mode exists to
# produce before the comparison is credited with having compared it. Without
# this a fixture that silently stopped perturbing anything would keep the
# harness green, which is the failure mode a differential harness has and a
# unit test does not.
expect_oracle() { # <label> <fragment> <argument>...
  local label="$1" fragment="$2"
  shift 2
  local dir="$work/expect"
  rm -rf "$dir"
  mkdir -p "$dir/tmp"
  cp -a "$REPO" "$dir/repo"
  runtime_env "$dir/repo" "$dir/tmp"
  env "${RUNTIME_ENV[@]}" bash "$SAFIX_SH" "$@" < <(cat "$(input_path)") \
    >"$dir/out" 2>"$dir/err" || true
  grep -qF -- "$fragment" "$dir/out" "$dir/err" \
    || fail "the $label fixture drew no '$fragment' from the oracle, so comparing it is vacuous"
}

# Every value this fixture ever holds, so that a leak of any of them is caught
# rather than only a leak of the one the invocation touched.
VALUES=(
  value-ana-alone value-api-token value-ana-ops-tooling value-ops-handover
  value-team-vault value-bo-service value-bo-ops-tooling value-reset
  value-orphan value-extra
)

grep_for_values() { # <path>...
  local pattern=() value
  for value in "${VALUES[@]}"; do pattern+=(-e "$value"); done
  grep -rIl "${pattern[@]}" "$@" 2>/dev/null | grep -q .
}

residue_free() { # <dir> <side> <label>
  local dir="$1" side="$2" label="$3"
  if grep_for_values "$dir"; then
    fail "the $side runtime left a plaintext value in its temporary directory for [$label]"
  fi
}

# The one piece of residue the effect projection cannot catch on its own: a
# candidate document left beside its target is a file both runtimes could leave,
# and two repositories that both hold one compare equal. `set` names its
# candidate after the target with the suffix below, so the tree is asked
# directly.
no_scratch_left() { # <repo> <side> <label>
  if find "$1" -name '*safix-tmp*' -print -quit 2>/dev/null | grep -q .; then
    fail "the $2 runtime left a candidate document beside its target for [$3]"
  fi
}

# The one substitution this harness applies, and it carries its own proof.
#
# `set` prints the abbreviated object name of the commit it just made. Two
# correct runs cannot print the same one: a value written now takes a fresh
# initialization vector, so the two trees differ, so the two commits differ. What
# is substituted is each side's OWN `git rev-parse --short HEAD`, so a runtime
# that printed a hash which is not its repository's HEAD — a stale one, the other
# side's, or one where the other side printed none — leaves something still
# shaped like an object name behind, and that is a failure rather than a
# normalization.
# The second is the repository root. Each side is handed its own copy of the
# fixture at its own path, so a message that names the repository — the marker of
# an operation in progress, the file `fix` wrote — names two different absolute
# paths for one reason that has nothing to do with the runtimes. Substituted per
# side from that side's own root, and anything still naming a path inside this
# harness's scratch directory afterwards is a runtime talking about somewhere
# other than the repository it was given, which fails.
normalize_run() { # <repo> <file>...
  local repo="$1" head file
  shift
  head="$(git -C "$repo" rev-parse --short HEAD 2>/dev/null || true)"
  for file in "$@"; do
    [ -e "$file" ] || continue
    sed -i "s|$repo|<repo>|g" "$file"
    if [ -n "$head" ]; then
      sed -i "s/$head/<head>/g" "$file"
    fi
    if grep -qE 'committed [0-9a-f]{4,}' "$file"; then
      fail "a runtime named a commit that is not its own repository's HEAD, in $file"
    fi
    if grep -qF -- "$work/" "$file"; then
      fail "a runtime named a path outside the repository it was given, in $file"
    fi
  done
}

# What `set` promises about the keys it did not name.
#
# sops reuses each unchanged value's original initialization vector, so every
# other key in the file keeps byte-identical ciphertext; only the named key's
# line, the message authentication code and `lastmodified` may move. Judged
# against the fixture each side started from rather than across the two sides,
# because the two sides necessarily differ on the key that WAS set — comparing
# them there would compare sops' random number generator.
bystanders_untouched() { # <repo> <side> <relpath> <key> <label>
  local repo="$1" side="$2" rel="$3" key="$4" label="$5" line
  [ -e "$REPO/$rel" ] || return 0
  [ -e "$repo/$rel" ] || fail "the $side runtime removed $rel for [$label]"
  while IFS= read -r line; do
    case "$line" in
      "< $key:"* | "> $key:"*) ;;
      "< "*"mac: ENC["* | "> "*"mac: ENC["*) ;;
      "< "*"lastmodified:"* | "> "*"lastmodified:"*) ;;
      *) fail "the $side runtime disturbed a key it was not asked to set in $rel for [$label]: $line" ;;
    esac
  done < <(diff "$REPO/$rel" "$repo/$rel" | grep '^[<>]' || true)
}

# Both sides of the run just compared, held to the promise above.
both_bystanders_untouched() { # <relpath> <key> <label>
  bystanders_untouched "$work/run/sh/repo" shell "$1" "$2" "$3"
  bystanders_untouched "$work/run/rs/repo" rust "$1" "$2" "$3"
}

# --- Fixture perturbations ---------------------------------------------------------
# Recipients that disagree with the declared audience, produced the way real
# drift is produced: the policy is narrowed, sops re-wraps to it, and the policy
# is put back. Nothing decrypts to a file on the way past, so the drifted fixture
# costs no plaintext.
drift_recipients() { # <relpath> <anchor>...
  local file="$1" anchor
  shift
  {
    printf 'keys:\n'
    printf '  - &ana %s\n' "$ANA_PUB"
    printf '  - &ana-escrow %s\n' "$ESCROW_PUB"
    printf '  - &bo %s\n' "$BO_PUB"
    printf '  - &cy %s\n' "$CY_PUB"
    printf 'creation_rules:\n'
    printf '  - path_regex: ^%s/[^/]*\\.yaml$\n' "$(dirname "$file")"
    printf '    key_groups:\n      - age:\n'
    for anchor in "$@"; do printf '          - *%s\n' "$anchor"; done
  } >"$REPO/.sops.yaml"
  # sops resolves a rule's `path_regex` against the path relative to the config
  # it read, so the narrowed policy has to sit where the real one sits. Putting
  # it there and taking it away again is also what real drift is: a policy that
  # changed, ciphertext re-wrapped to it, and the policy since regenerated.
  SOPS_AGE_KEY_FILE="$work/keys.txt" sops --config "$REPO/.sops.yaml" updatekeys -y "$REPO/$file" \
    >/dev/null || fail "could not re-wrap $file for the drift fixture"
  cp "$work/fixture/policy.yaml" "$REPO/.sops.yaml"
  git -C "$REPO" add -- "$file" .sops.yaml
  git -C "$REPO" commit -q -m "fixture: drift $file" -- "$file" .sops.yaml
}

add_orphan_key() { # <relpath> <key>
  # A JSON scalar on the pipe, because that is what `sops set` takes and what
  # the shell runtime hands it: the runtime pipes `jq -Rs .` of the typed value,
  # so the orphan arrives the same way a declared value would.
  printf 'value-orphan' | jq -Rs . \
    | SOPS_AGE_KEY_FILE="$work/keys.txt" sops --config "$REPO/.sops.yaml" \
      set --value-stdin --input-type yaml --output-type yaml "$REPO/$1" "[\"$2\"]" \
    || fail "could not add the orphan key to $1"
  git -C "$REPO" add -- "$1"
  git -C "$REPO" commit -q -m "fixture: orphan key in $1" -- "$1"
}

# A file named in `extraGovernedFiles` that no rule's directory covers. It is
# encrypted to a named recipient rather than through a creation rule, because
# having no rule is the whole point of it.
add_ungovernable_extra() {
  mkdir -p "$REPO/$(dirname "$EXTRA_FILE")"
  printf 'note: "value-extra"\n' \
    | SOPS_AGE_KEY_FILE="$work/keys.txt" sops encrypt --age "$ANA_PUB" \
      --input-type yaml --output-type yaml /dev/stdin >"$REPO/$EXTRA_FILE" \
    || fail "could not create the ungovernable extra file"
  jq --arg f "$EXTRA_FILE" '.extra = [$f] | .managed = (.managed + [$f] | sort)' \
    "$work/fixture/governed.json" >"$work/fixture/governed.tmp"
  mv "$work/fixture/governed.tmp" "$work/fixture/governed.json"
  git -C "$REPO" add -- "$EXTRA_FILE"
  git -C "$REPO" commit -q -m "fixture: a file no rule covers" -- "$EXTRA_FILE"
}

# A placement outside the `*.yaml` suffix every generated rule ends in.
add_unruled_placement() {
  jq '.bo["bad-path"] = { file: "secrets/safix/users/bo/notes.txt", key: "bad-path",
                          origin: "private", owner: "bo", shared: false, generator: null }' \
    "$work/fixture/placements.json" | jq -S . >"$work/fixture/placements.tmp"
  mv "$work/fixture/placements.tmp" "$work/fixture/placements.json"
  assert_attribute_order
}

# A placement in a directory no creation rule covers, whose file does not exist.
# `set` there has to fail closed at creation rather than acquire a default
# recipient set, and both runtimes have to say so in the same words.
add_ungoverned_placement() {
  jq '.cy["stray"] = { file: "secrets/elsewhere/notes.yaml", key: "stray",
                       origin: "private", owner: "cy", shared: false, generator: null }' \
    "$work/fixture/placements.json" | jq -S . >"$work/fixture/placements.tmp"
  mv "$work/fixture/placements.tmp" "$work/fixture/placements.json"
  assert_attribute_order
}

# The repository part-way through an operation a commit would disturb. The
# marker's existence is the whole signal — `set` reads it and refuses before it
# reaches the working tree — so an empty one is the honest fixture.
mark_mid_merge() { : >"$REPO/.git/MERGE_HEAD"; }

# `nix eval --json` emits every attribute set with its names sorted, and the two
# runtimes read that order differently: the shell's `list` renders in the
# document's own order, while its own refusals — and everything the rust runtime
# does — render sorted. The two coincide over nix's output and over nothing else,
# so the stub has to emit what nix emits, and a perturbation that appends a
# placement has to be held to it rather than quietly producing a fixture the two
# runtimes are entitled to disagree about.
assert_attribute_order() {
  local file
  for file in placements audiences recipients; do
    jq -S . "$work/fixture/$file.json" >"$work/fixture/$file.sorted"
    cmp -s "$work/fixture/$file.json" "$work/fixture/$file.sorted" \
      || fail "the $file fixture is not in the attribute order nix emits"
    rm -f "$work/fixture/$file.sorted"
  done
}

# --- Modes -------------------------------------------------------------------------
# The read surface, over every fixture: the invocations that succeed and the
# invocations that refuse, driven against both runtimes in one list so that a
# mode's whole surface is compared rather than the one call it was written for.
compare_read_surface() { # <label-prefix>
  local at="$1"
  compare "$at/list-default" list
  compare "$at/list-ana" list ana
  compare "$at/list-bo" list bo
  compare "$at/list-cy" list cy
  compare "$at/check" check
  compare "$at/check-ana" check ana
  compare "$at/check-bo" check bo
  compare "$at/check-cy" check cy
  compare "$at/get-ana-alone" get ana ana-alone
  compare "$at/get-shared-from-bo" get bo ops-handover
  compare "$at/get-default-user" get team-vault
}

# `fix` is a convergence, and the claim is not that two runtimes print the same
# thing — `compare` already judges that — but that either one, run once, leaves a
# repository the drift report is then silent about. Asserted per side, because a
# pair that converged to two different repositories would still have to be
# caught, and then compared.
converges() { # <label> <fix-argument>...
  local label="$1" side dir status
  shift
  rm -rf "$work/converge"
  for side in sh rs; do
    dir="$work/converge/$side"
    mkdir -p "$dir/tmp"
    cp -a "$REPO" "$dir/repo"
    runtime_env "$dir/repo" "$dir/tmp"
    status=0
    if [ "$side" = sh ]; then
      env "${RUNTIME_ENV[@]}" bash "$SAFIX_SH" "$@" \
        </dev/null >"$dir/fix.out" 2>"$dir/fix.err" || status=$?
    else
      env "${RUNTIME_ENV[@]}" SAFIX_ERROR_FORMAT=plain "$SAFIX_RS" "$@" \
        </dev/null >"$dir/fix.out" 2>"$dir/fix.err" || status=$?
    fi
    [ "$status" = 0 ] || {
      cat "$dir/fix.err" >&2
      fail "the $side runtime's [$label] exited $status"
    }

    status=0
    if [ "$side" = sh ]; then
      env "${RUNTIME_ENV[@]}" bash "$SAFIX_SH" check \
        </dev/null >"$dir/check.out" 2>"$dir/check.err" || status=$?
    else
      env "${RUNTIME_ENV[@]}" SAFIX_ERROR_FORMAT=plain "$SAFIX_RS" check \
        </dev/null >"$dir/check.out" 2>"$dir/check.err" || status=$?
    fi
    [ "$status" = 0 ] || {
      cat "$dir/check.err" >&2
      fail "the $side runtime's [$label] did not converge: check still reports drift"
    }
    no_scratch_left "$dir/repo" "$side" "$label"
    project "$dir/repo" >"$dir/projection"
  done
  cmp -s "$work/converge/sh/projection" "$work/converge/rs/projection" \
    || {
      diff -u "$work/converge/sh/projection" "$work/converge/rs/projection" >&2 || true
      fail "the two runtimes converged to different repositories for [$label]"
    }
  COMPARED=$((COMPARED + 1))
  note "[$label] both runtimes converged, and check is silent about both"
}

# A divergence this harness records rather than reconciles.
#
# `safix.sh` drives its re-wrap loop with `done < <(jq -r '.managed[]' ...)`,
# which makes the loop's standard input the pipe carrying the governed file
# names — and `sops updatekeys`, run inside that loop, inherits it. Without
# `--yes` sops therefore reads its confirmation from that pipe rather than from
# the operator: the answer to the prompt for one file is the NAME of the next
# file, which is never `y`, and the list is consumed by the answers instead of by
# the loop. Interactive `fix` in the shell runtime never reaches a terminal, so
# it can neither be confirmed nor declined on purpose.
#
# The rust runtime hands sops the run's own standard input, so the prompt is
# answerable. The difference is not reconcilable in the rust runtime's favour or
# against it: reproducing the oracle would mean reproducing a prompt nobody can
# answer, and a re-wrap is not something to leave unanswerable.
#
# So the divergence is pinned rather than compared. What both runtimes must still
# agree on is the outcome that matters — with no answer available, neither
# re-wraps anything and both refuse — and the shape of the difference is asserted
# so that an oracle which later stops stealing its own file list fails here and
# is looked at again.
interactive_fix_diverges() {
  local side dir status prompts
  rm -rf "$work/interactive"
  declare -A seen_prompts=()
  for side in sh rs; do
    dir="$work/interactive/$side"
    mkdir -p "$dir/tmp"
    cp -a "$REPO" "$dir/repo"
    runtime_env "$dir/repo" "$dir/tmp"
    status=0
    if [ "$side" = sh ]; then
      env "${RUNTIME_ENV[@]}" bash "$SAFIX_SH" fix \
        < <(cat /dev/null) >"$dir/out" 2>"$dir/err" || status=$?
    else
      env "${RUNTIME_ENV[@]}" SAFIX_ERROR_FORMAT=plain "$SAFIX_RS" fix \
        < <(cat /dev/null) >"$dir/out" 2>"$dir/err" || status=$?
    fi
    [ "$status" != 0 ] \
      || fail "the $side runtime's interactive fix succeeded with no answer available"

    prompts="$(grep -o 'Is this okay' "$dir/out" | grep -c . || true)"
    seen_prompts[$side]="$prompts"

    recipients_unchanged "$dir/repo" "$side"
    no_scratch_left "$dir/repo" "$side" converge/fix-interactive
  done

  [ "${seen_prompts[sh]}" -gt 1 ] \
    || fail "the oracle no longer answers its own prompt from its file list; re-examine the divergence"
  [ "${seen_prompts[rs]}" = 1 ] \
    || fail "the rust runtime prompted ${seen_prompts[rs]} times for one file"
  COMPARED=$((COMPARED + 1))
  note "[converge/fix-interactive] divergence pinned: the oracle prompted ${seen_prompts[sh]} times over a stdin carrying no answer, the rust runtime once; neither re-wrapped anything"
}

# Every governed file still wrapped for exactly the recipients it was wrapped for
# before the run, which is what "nothing was re-wrapped" means.
recipients_unchanged() { # <repo> <side>
  local repo="$1" side="$2" file before after
  while IFS= read -r file; do
    [ -e "$REPO/$file" ] || continue
    before="$(sops-recipients-of "$REPO/$file" "$work/empty.json" | jq -c '.actual')"
    after="$(sops-recipients-of "$repo/$file" "$work/empty.json" | jq -c '.actual')"
    [ "$before" = "$after" ] \
      || fail "the $side runtime re-wrapped $file without an answer to its confirmation"
  done < <(governed_paths)
}

prepare() {
  setup_keys
  write_fixture
  write_stub_nix
  printf '[]\n' >"$work/empty.json"
  setup_repo
}

case "$mode" in
  # Every value set, the policy in step, no copy anywhere it should not be.
  # `check` reports nothing, and the two runtimes agree on that as much as they
  # agree on a report full of findings.
  clean)
    prepare
    seed_values
    expect_oracle clean 'no drift.' check
    compare_read_surface clean
    ;;

  # Two declared names with no value, one with a generator and one without,
  # which is the distinction the report exists to draw.
  missing)
    prepare
    set_value ana ana-alone value-ana-alone
    set_value ana ops-tooling value-ana-ops-tooling
    set_value ana ops-handover value-ops-handover
    set_value ana team-vault value-team-vault
    expect_oracle missing 'It has a generator.' check
    expect_oracle missing 'It has no generator.' check
    expect_oracle missing 'has no value. Set one with' get bo bo-service
    compare_read_surface missing
    compare missing/get-valueless get bo bo-service
    ;;

  # A file whose stanzas disagree with the audience declared for it, in both
  # directions at once: cy can open ana's file and ana's escrow cannot.
  drift)
    prepare
    seed_values
    drift_recipients "$ANA_FILE" ana cy
    expect_oracle drift 'is not encrypted to the audience declared for it.' check
    expect_oracle drift 'is in its audience and cannot open it:' check
    compare_read_surface drift
    ;;

  # A value in a governed file that no declaration claims.
  orphan)
    prepare
    seed_values
    add_orphan_key "$ANA_FILE" orphan_key
    expect_oracle orphan 'and no declaration claims it.' check
    compare_read_surface orphan
    ;;

  # Names nobody declared, argument lists no subcommand takes, and the help each
  # ported subcommand prints.
  unknown)
    prepare
    seed_values
    expect_oracle unknown 'is not a declared user of flake.safix.users.' list dee
    expect_oracle unknown 'is not a secret flake.safix.users.ana holds.' get ana no-such-secret
    compare unknown/list-user list dee
    compare unknown/check-user check dee
    compare unknown/get-user get dee ana-alone
    compare unknown/get-name get ana no-such-secret
    compare unknown/get-name-cy get cy anything
    compare unknown/usage-list list ana bo
    compare unknown/usage-get get
    compare unknown/usage-get-many get ana bo cy
    compare unknown/usage-check check ana bo
    compare unknown/help-list list -h
    compare unknown/help-get get --help
    compare unknown/help-check check -h
    compare unknown/help-list-trailing list ana -h
    ;;

  # A governed file no creation rule's directory covers, and a placement outside
  # the suffix every rule ends in. Neither can be repaired by `fix`, and both
  # runtimes have to say so in the same words.
  norule)
    prepare
    seed_values
    add_ungovernable_extra
    add_unruled_placement
    expect_oracle norule 'extraGovernedFiles and no creation rule' check
    expect_oracle norule 'which is not a *.yaml path' get bo bad-path
    compare_read_surface norule
    compare norule/get-unruled get bo bad-path
    ;;

  # The write path that lands: a value replaced in a file that exists, a value
  # re-entered unchanged, a file created through the creation rules, and the
  # default user resolving to the one the environment names. A staged change to
  # a path `set` does not name sits in the index throughout, because surviving
  # there — staged, and not swept into a commit whose message names one secret —
  # is part of what `set` promises.
  write)
    prepare
    set_value ana ana-alone value-ana-alone
    set_value ana api-token value-api-token
    set_value ana ops-tooling value-ana-ops-tooling
    set_value ana ops-handover value-ops-handover
    set_value ana team-vault value-team-vault
    printf 'a note nobody asked for\n' >"$REPO/notes.md"
    git -C "$REPO" add -- notes.md

    with_input value-reset value-reset
    expect_oracle write 'committed' set ana ana-alone
    compare write/set-existing set ana ana-alone
    both_bystanders_untouched "$ANA_FILE" ana_alone write/set-existing

    with_input value-ana-ops-tooling value-ana-ops-tooling
    expect_oracle write 'unchanged — the file already holds this value' set ana ops-tooling
    compare write/set-idempotent set ana ops-tooling
    both_bystanders_untouched "$ANA_FILE" ops_tooling write/set-idempotent

    with_input value-bo-service value-bo-service
    expect_oracle write 'does not exist yet; creating it through sops' set bo bo-service
    compare write/set-new-file set bo bo-service

    with_input value-team-vault value-team-vault
    compare write/set-default-user set team-vault
    both_bystanders_untouched "$SHARED_FILE" team-vault write/set-default-user

    no_input
    compare write/help-set set -h
    compare write/help-fix fix -h
    ;;

  # Everything `set` refuses about how it was asked, and about what was typed.
  # Nothing here perturbs the fixture: these are the refusals a correct
  # repository still produces.
  refuse)
    prepare
    seed_values

    no_input
    expect_oracle refuse 'is not a declared user of flake.safix.users.' set dee ana-alone
    compare refuse/unknown-user set dee ana-alone
    compare refuse/unknown-name set ana no-such-secret
    compare refuse/usage-none set
    compare refuse/usage-many set ana bo cy
    compare refuse/usage-fix fix --no
    compare refuse/usage-fix-many fix --yes --yes

    with_input "" ""
    expect_oracle refuse 'the value is empty' set ana ana-alone
    compare refuse/empty-value set ana ana-alone

    with_input one two
    expect_oracle refuse 'the two entries differ' set ana ana-alone
    compare refuse/entries-differ set ana ana-alone

    with_input
    expect_oracle refuse 'no value read' set ana ana-alone
    compare refuse/no-value-read set ana ana-alone

    with_unterminated_input 'half a value'
    expect_oracle refuse 'no value read' set ana ana-alone
    compare refuse/unterminated-value set ana ana-alone

    with_input one
    expect_oracle refuse 'no confirmation read' set ana ana-alone
    compare refuse/no-confirmation-read set ana ana-alone
    ;;

  # The four states a write is refused in because of what the repository or the
  # declarations are, rather than because of what was typed. Each is arranged in
  # turn and the earlier ones are left in place, so the order is the order in
  # which they stop being reachable: drift on ana's file, then an uncommitted
  # change to bo's, then two placements no rule can serve, then a merge in
  # progress, which refuses everything after it.
  guard)
    prepare
    seed_values

    drift_recipients "$ANA_FILE" ana cy
    with_input value-reset value-reset
    expect_oracle guard 'is not encrypted to the audience declared for it.' set ana ana-alone
    compare guard/recipient-drift set ana ana-alone

    printf '# an edit sops did not make\n' >>"$REPO/$BO_FILE"
    with_input value-reset value-reset
    expect_oracle guard 'already has uncommitted changes' set bo bo-service
    compare guard/uncommitted-changes set bo bo-service

    add_unruled_placement
    with_input value-reset value-reset
    expect_oracle guard 'which is not a *.yaml path' set bo bad-path
    compare guard/not-a-yaml-path set bo bad-path

    add_ungoverned_placement
    with_input value-reset value-reset
    expect_oracle guard 'has no creation rule for' set cy stray
    compare guard/no-creation-rule set cy stray

    mark_mid_merge
    with_input value-reset value-reset
    expect_oracle guard 'Finish or abort it before setting a secret.' set ana ana-alone
    compare guard/mid-merge set ana ana-alone
    ;;

  # `fix`, over a fixture that has drifted in both directions at once. Compared
  # as an invocation, and then asserted as a convergence: run once, `check` has
  # nothing left to report. Both bounds of the re-wrap fan-out are exercised,
  # because the bound is what decides whether sops holds the operator's own
  # streams or a pipe.
  converge)
    prepare
    seed_values
    drift_recipients "$ANA_FILE" ana cy

    no_input
    expect_oracle converge 'sops updatekeys' fix --yes
    expect_oracle converge 'It does not revoke' fix --yes

    compare converge/fix-yes fix --yes
    converges converge/fix-yes fix --yes

    EXTRA_ENV=("SAFIX_FIX_CONCURRENCY=1")
    compare converge/fix-yes-serial fix --yes
    converges converge/fix-yes-serial fix --yes
    EXTRA_ENV=()

    interactive_fix_diverges
    ;;

  # The harness is not trusted until it has been shown to fail. Each drill puts
  # a deliberately wrong runtime in the rust side's place and asserts that the
  # comparison fails, and that it fails on the channel that exists to catch that
  # mutation rather than incidentally on another.
  drills)
    prepare
    seed_values
    real_rs="$SAFIX_RS"

    export REAL_RS="$real_rs"

    # Each drill runs one comparison with a wrapper in the rust side's place and
    # asserts two things: that the comparison failed, and that the message names
    # the channel the mutation belongs to. The second half is what makes the
    # drill evidence about that channel rather than about the harness in
    # general, and `compare` reports the first channel that differs, so a
    # mutation caught under another channel's name is a mutation reaching
    # further than intended.
    drill() { # <name> <expected-fragment> <argument>... ; body on stdin
      local name="$1" expect="$2"
      shift 2
      local output status=0
      emit_stub "$work/bin/drill"
      SAFIX_RS="$work/bin/drill"
      output="$(compare "drill/$name" "$@" 2>&1)" || status=$?
      SAFIX_RS="$real_rs"
      [ "$status" != 0 ] || fail "the $name drill was not caught: the comparison passed"
      case "$output" in
        *"$expect"*) note "drill $name caught on: $expect" ;;
        *) fail "the $name drill was caught by something other than '$expect': $output" ;;
      esac
    }

    drill stdout 'stdout differs' list ana <<'SH'
status=0
"$REAL_RS" "$@" || status=$?
printf 'a line the shell runtime does not print\n'
exit "$status"
SH

    # Driven at a refusal, because an invocation that writes nothing to standard
    # error would let a stderr mutation pass unnoticed and prove nothing.
    drill stderr 'stderr differs' list dee <<'SH'
status=0
"$REAL_RS" "$@" || status=$?
printf 'a note the shell runtime does not write\n' >&2
exit "$status"
SH

    drill status 'exit status differs' list ana <<'SH'
"$REAL_RS" "$@" || true
exit 3
SH

    drill effects 'repository effects differ' list ana <<'SH'
status=0
"$REAL_RS" "$@" || status=$?
: >"$SAFIX_REPO_ROOT/an-extra-file"
exit "$status"
SH

    drill residue 'plaintext value in its temporary directory' list ana <<'SH'
status=0
"$REAL_RS" "$@" || status=$?
printf 'value-ana-alone' >"$TMPDIR/leaked"
exit "$status"
SH
    ;;

  *) fail "unknown mode" ;;
esac

if [ "$mode" = drills ]; then
  note "every channel was shown to fail under the mutation it exists to catch."
else
  [ "$COMPARED" -gt 0 ] || fail "no invocation was compared"
  note "$COMPARED invocation(s) compared, all four channels identical."
fi
