#!/usr/bin/env bash
# safix — the whole lifecycle of one secret, by name and never by file.
#
#   safix set  [<user>] <name>   # prompt twice, write, stage, commit
#   safix get  [<user>] <name>   # decrypt one key to stdout (plaintext, for piping)
#   safix list [<user>]          # every name this user holds, and what serves it
#
# The declarations already know which file every secret belongs to: a secret's
# audience is its owner plus everyone the owner shares it with, and one file
# serves each distinct audience (modules/flake/safix/resolve.nix). So the
# operator names a secret and this derives the file, the key inside it, and the
# recipients.
#
# Placement is read from `flake.safix.lib.placements` and never guessed. A name
# no declaration covers is refused rather than given a destination: inventing one
# would put a value in a file whose recipients are not the audience the
# declarations compute and for which the recipient policy writes no rule.
#
# ── the value's path through this process ──
# `read -rs` puts it in a shell variable and a pipe carries it into
# `sops set --value-stdin`. It therefore never reaches argv (so never a process
# listing), never an environment variable (so never /proc/<pid>/environ), never
# a shell history file, and never a log. Nothing here has to write a value to
# disk in the clear at any point, so there is no plaintext scratch file to shred.
# Bash herestrings and command substitutions are avoided for the value for the
# same reason: both materialize their operand in $TMPDIR.
#
# The ciphertext scratch file is registered and shredded by the single
# process-wide EXIT trap below, with INT and TERM routed through exit so there is
# exactly one shredder and no path that skips it. A function-scoped
# `trap ... RETURN` is the shape this deliberately avoids: it does not run when
# the process dies between the write and the return, which is the abort a value
# actually leaks through.
#
# ── what a write does and does not touch ──
# `sops set` rewrites one key. sops reuses each unchanged value's original IV, so
# every other key in the file keeps byte-identical ciphertext, and `--idempotent`
# makes re-setting an unchanged value a no-op that does not even churn the MAC or
# `lastmodified`. Without that flag sops rewrites both, so a re-run one second
# later stages a diff that says nothing.
#
# Every write lands through an atomic rename from a scratch file beside the
# target, so an abort leaves either the previous file or no file, never a
# truncated one.
#
# ── what this does not do ──
# Change recipients. It writes through `sops`, which reads them from the file's
# own metadata (existing file) or from .sops.yaml's creation rules (new file), so
# no run of this can grant anyone a key. It does refuse to write when those
# recipients are not the audience the declarations name for the file — see
# `refuse_recipient_drift` below for why that refusal has to happen here rather
# than being left to `safix fix` and the drift check. Repairing drift remains
# their business; declining to mint a fresh value into it is this refusal's.
#
# Multi-line values. `read` takes one line, so a PEM or a TOML block is
# `sops <file>` rather than this. Single-line values are stored exactly as typed,
# with no trailing newline.
#
# A file no declaration names is reachable only as `sops <file>`, because a name
# is the only handle this command takes. Naming it in
# `flake.safix.extraGovernedFiles` is what puts it in the set `fix` re-wraps.
#
# External commands are reached through overridable variables (SAFIX_GIT,
# SAFIX_SOPS, SAFIX_NIX, SAFIX_RECIPIENTS_OF, SAFIX_KEYS_OF) so the hermetic
# flake checks can drive it against fixture state. The checks run the REAL sops,
# age, git and recipient reader; only `nix` is stubbed, because a flake
# evaluation is what a sandbox cannot do and a stub standing in for the backend
# is what lets a check stay green over a backend that has gone missing.
set -euo pipefail

GIT="${SAFIX_GIT:-git}"
SOPS="${SAFIX_SOPS:-sops}"
NIX="${SAFIX_NIX:-nix}"
RECIPIENTS_OF="${SAFIX_RECIPIENTS_OF:-sops-recipients-of}"
KEYS_OF="${SAFIX_KEYS_OF:-sops-keys-of}"

PROG="safix"

log() { printf '%s\n' "$*" >&2; }
note() { printf '  %s\n' "$*" >&2; }
die() {
  printf '%s: %s\n' "$PROG" "$*" >&2
  exit 1
}

# --- Scratch files --------------------------------------------------------------
# Ciphertext rather than plaintext, but registered and shredded all the same: a
# stray scratch file beside a secrets file is one an operator could mistake for
# the real one, and an aborted run must leave the tree as it found it.
SCRATCH_FILES=()
SCRATCH_DIRS=()

cleanup_scratch() {
  local f d
  for f in "${SCRATCH_FILES[@]}"; do
    [ -e "$f" ] || continue
    shred -u "$f" 2>/dev/null || rm -f "$f"
  done
  SCRATCH_FILES=()
  # Only ever a directory this run created and only while still empty, so an
  # aborted first write leaves no evidence and a populated audience directory is
  # never at risk. `rmdir -p` because `mkdir -p` can create more than one level —
  # a first shared audience creates `shared/` as well as `shared/<a>+<b>/` — and
  # it stops at the first ancestor that is not empty, which the repository root
  # always is.
  for d in "${SCRATCH_DIRS[@]}"; do
    rmdir -p "$d" 2>/dev/null || true
  done
  SCRATCH_DIRS=()
}
trap cleanup_scratch EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# --- Repository -----------------------------------------------------------------
REPO_ROOT="${SAFIX_REPO_ROOT:-$("$GIT" rev-parse --show-toplevel)}"
[ -n "$REPO_ROOT" ] || die "not inside a git repository"

# --- Placement (what the declarations resolve to, cached for this run) -----------
PLACEMENTS=""

load_placements() {
  [ -z "$PLACEMENTS" ] || return 0
  PLACEMENTS="$(mktemp)"
  SCRATCH_FILES+=("$PLACEMENTS")
  "$NIX" eval --json "$REPO_ROOT#safix.lib.placements" >"$PLACEMENTS" \
    || die "could not evaluate flake.safix.lib.placements in $REPO_ROOT"
}

declared_users() { jq -r 'keys[]' "$PLACEMENTS"; }

names_of() { jq -r --arg u "$1" '(.[$u] // {}) | keys[]' "$PLACEMENTS"; }

# The user whose custody a bare `safix set <name>` means. $USER when the
# declarations name them, which is the case this exists for: the operator setting
# their own secret on their own workstation. A machine whose login name is not a
# declared user — a builder, a root shell — falls through to the sole declared
# holder when there is exactly one, and is otherwise told to name the user,
# because guessing between two people's custody is the one guess with a
# disclosure at the end of it.
default_user() {
  local me holders count
  me="${USER:-$(id -un)}"
  if declared_users | grep -qxF -- "$me"; then
    printf '%s' "$me"
    return 0
  fi
  holders="$(jq -r 'to_entries | map(select(.value | length > 0)) | .[].key' "$PLACEMENTS")"
  count="$(printf '%s' "$holders" | grep -c . || true)"
  if [ "$count" = 1 ]; then
    printf '%s' "$holders"
    return 0
  fi
  die "no default user: '$me' is not declared in flake.safix.users, and the declarations name $count holders. Name one: $PROG <subcommand> <user> <name>"
}

refuse_unknown_user() { # <user>
  {
    printf '%s: '\''%s'\'' is not a declared user of flake.safix.users.\n\n' "$PROG" "$1"
    printf 'Declared users:\n'
    declared_users | sed 's/^/  - /'
  } >&2
  exit 1
}

refuse_unknown_name() { # <user> <name>
  {
    printf '%s: '\''%s'\'' is not a secret flake.safix.users.%s holds.\n\n' "$PROG" "$2" "$1"
    printf 'A secret is declared in exactly one of three places, and this resolves\n'
    printf 'the owning file from all three:\n\n'
    printf '  1. flake.safix.catalogue.%s — the shared catalogue — selected by\n' "$2"
    printf '     flake.safix.users.%s.carries.%s\n' "$1" "$2"
    printf '  2. flake.safix.users.%s.private.%s — this user'\''s own entry\n' "$1" "$2"
    printf '  3. flake.safix.users.<owner>.sharedWith.%s.%s — granted from outside\n\n' "$1" "$2"
    printf 'Declare it in one of them, then re-run. A name reaching a set only\n'
    printf 'through a perHost.<host> or perTag.<tag> add or force is refused here\n'
    printf 'too: placement is derived from those three sources alone, and the\n'
    printf 'per-host and per-tag scopes sit outside it deliberately, because they\n'
    printf 'adjust a secret for one machine rather than declare who holds it. One\n'
    printf 'file serves every host that resolves the secret, so a value set through\n'
    printf 'a single host'\''s adjustment would apply everywhere that secret\n'
    printf 'resolves rather than only where the adjustment does.\n\n'
    printf 'Names flake.safix.users.%s holds:\n' "$1"
    names_of "$1" | sed 's/^/  - /'
  } >&2
  exit 1
}

# Sets PLACE_FILE, PLACE_KEY, PLACE_ORIGIN, PLACE_OWNER.
PLACE_FILE=""
PLACE_KEY=""
PLACE_ORIGIN=""
PLACE_OWNER=""
resolve_placement() { # <user> <name>
  local user="$1" name="$2" record
  load_placements
  jq -e --arg u "$user" 'has($u)' "$PLACEMENTS" >/dev/null || refuse_unknown_user "$user"
  jq -e --arg u "$user" --arg n "$name" '.[$u] | has($n)' "$PLACEMENTS" >/dev/null \
    || refuse_unknown_name "$user" "$name"

  record="$(jq -r --arg u "$user" --arg n "$name" \
    '.[$u][$n] | [.file, .key, .origin, .owner] | @tsv' "$PLACEMENTS")"
  IFS=$'\t' read -r PLACE_FILE PLACE_KEY PLACE_ORIGIN PLACE_OWNER <<EOF
$record
EOF
  [ -n "$PLACE_FILE" ] || die "the declarations resolved no file for '$name'"

  # Every generated `path_regex` ends in a literal `\.yaml$`, so that a recipient
  # sweep can never reach encrypted material this package did not place — whose
  # original identities are gone and whose recipients are therefore unrecoverable
  # once rewritten. A placement outside that suffix is a path no rule covers, and
  # sops infers a document's format from the extension besides, so it is refused
  # rather than written with a guessed type.
  case "$PLACE_FILE" in
    *.yaml) ;;
    *) die "the declarations place '$name' at $PLACE_FILE, which is not a *.yaml path; every .sops.yaml creation rule ends in \\.yaml\$, so no rule covers it" ;;
  esac
}

# --- Git preflight ---------------------------------------------------------------
# A commit is part of what this command promises, so the states in which a commit
# would mean something other than "this value was set" are refused before the
# operator is asked for anything. Mid-rebase and mid-merge are refused because a
# partial commit is rejected outright during a merge and silently reorders history
# during a rebase; a dirty target file is refused because committing it would
# sweep an edit this command did not make into a message that claims only one.
refuse_bad_repo_state() { # <relpath>
  local rel="$1" gitdir state
  gitdir="$("$GIT" -C "$REPO_ROOT" rev-parse --absolute-git-dir)"
  for state in rebase-merge rebase-apply MERGE_HEAD CHERRY_PICK_HEAD REVERT_HEAD; do
    [ -e "$gitdir/$state" ] || continue
    die "the repository is mid-$state ($gitdir/$state). Finish or abort it before setting a secret."
  done

  if [ -n "$("$GIT" -C "$REPO_ROOT" ls-files -u -- "$rel")" ]; then
    die "$rel has unmerged conflict entries. Resolve them before setting a secret."
  fi

  # stderr discarded: the porcelain output is the whole signal here, and a
  # warning on git's stderr would otherwise print in the middle of this
  # command's own messages — immediately before a refusal, where it reads as
  # part of the explanation the operator is meant to act on.
  local status
  status="$("$GIT" -C "$REPO_ROOT" status --porcelain --untracked-files=all -- "$rel" 2>/dev/null)"
  [ -n "$status" ] || return 0
  {
    printf '%s: %s already has uncommitted changes:\n\n' "$PROG" "$rel"
    printf '%s\n\n' "$status"
    printf 'This commits the file it writes, so committing it now would carry that\n'
    printf 'change under a message naming only one secret. Commit or discard it\n'
    printf 'first, then re-run.\n'
  } >&2
  exit 1
}

# --- Recipients -------------------------------------------------------------------
# The audience the declarations name for every file they place a secret in,
# cached for this run. A second `nix eval` rather than one call fetching both
# attributes: the selftest's `nix` stub dispatches on the attribute each call
# names, so a rename of either fails a check rather than the operator's terminal,
# and the flake's eval cache makes the second call cheap.
AUDIENCES=""

load_audiences() {
  [ -z "$AUDIENCES" ] || return 0
  AUDIENCES="$(mktemp)"
  SCRATCH_FILES+=("$AUDIENCES")
  "$NIX" eval --json "$REPO_ROOT#safix.lib.audiences" >"$AUDIENCES" \
    || die "could not evaluate flake.safix.lib.audiences in $REPO_ROOT"
}

# `sops set` on an existing file takes that file's recipients from the file's own
# metadata. So a file whose recipients have drifted from the audience the
# declarations name — someone dropped from sharedWith, the policy regenerated,
# the ciphertext not yet re-wrapped — would take the new value and wrap it for the
# audience that used to be. This command commits what it writes, so that is a
# disclosure with a window in it: the removed reader gets a value minted after
# their removal, out of git history, and gets it before anyone runs the drift
# check that would have reported it.
#
# Judged on the candidate document rather than on the file in place, and before
# the rename, so a refusal is a run that never wrote: the EXIT trap shreds the
# scratch file and removes any directory this run created, leaving the tree as it
# was found. Judging the candidate also covers the new-file case, where the
# recipients came from a `.sops.yaml` creation rule that may itself be stale.
#
# Both sides are the ones `safix check` already uses, so the two cannot disagree:
# the declared side is `flake.safix.lib.audiences` and the actual side is read by
# ./sops_recipients.py. Recipients are age public keys, so naming them in a
# refusal discloses nothing.
refuse_recipient_drift() { # <relpath> <candidate>
  local rel="$1" candidate="$2" declared result extra missing
  load_audiences

  jq -e --arg f "$rel" 'has($f)' "$AUDIENCES" >/dev/null \
    || die "flake.safix.lib.audiences declares no audience for $rel, so there is nothing to hold its recipients to"

  declared="$(mktemp)"
  SCRATCH_FILES+=("$declared")
  jq --arg f "$rel" '.[$f].recipients' "$AUDIENCES" >"$declared"

  result="$("$RECIPIENTS_OF" "$candidate" "$declared")" \
    || die "could not read the recipients of the document prepared for $rel"
  extra="$(printf '%s' "$result" | jq -r '.extra[]')"
  missing="$(printf '%s' "$result" | jq -r '.missing[]')"
  [ -n "$extra$missing" ] || return 0

  {
    printf '%s: %s is not encrypted to the audience declared for it.\n\n' "$PROG" "$rel"
    if [ -n "$extra" ]; then
      printf 'Can open it and is not in its audience:\n'
      printf '%s\n' "$extra" | sed 's/^/  - /'
      printf '\n'
    fi
    if [ -n "$missing" ]; then
      printf 'Is in its audience and cannot open it:\n'
      printf '%s\n' "$missing" | sed 's/^/  - /'
      printf '\n'
    fi
    printf 'Nothing was written. A value set now would be wrapped for the recipients\n'
    printf 'above rather than for the declared audience, and this command commits what\n'
    printf 'it writes, so a reader the audience no longer names would read a value\n'
    printf 'minted after their removal straight out of git history.\n\n'
    printf 'Re-wrap the file to its declared audience, review the diff, then re-run:\n\n'
    printf '    %s fix\n' "$PROG"
    printf '    git diff -- %s\n' "$rel"
  } >&2
  exit 1
}

# --- The write ------------------------------------------------------------------
# A new file is created THROUGH sops so .sops.yaml's creation rules choose its
# recipients. `--filename-override` is what applies the rule for the path the file
# will occupy while the bytes are produced somewhere else, so a failure — most
# often a rule that has not been regenerated — leaves no half-made file beside the
# others. The document created holds the target key with an empty value and no
# secret at all; the value arrives in the separate `sops set` below.
create_through_sops() { # <relpath> <key> <out>
  local rel="$1" key="$2" out="$3" err
  err="$(mktemp)"
  SCRATCH_FILES+=("$err")
  if jq -nr --arg k "$key" '{ ($k): "" }' \
    | (cd "$REPO_ROOT" && "$SOPS" encrypt --filename-override "$rel" \
      --input-type json --output-type yaml /dev/stdin) >"$out" 2>"$err"; then
    return 0
  fi
  if grep -qF 'no matching creation rules found' "$err"; then
    {
      printf '%s: .sops.yaml has no creation rule for %s\n\n' "$PROG" "$rel"
      printf 'The recipient policy is generated from the declarations, and a file with\n'
      printf 'no rule must fail closed rather than acquire a default recipient set:\n'
      printf 'there is deliberately no catch-all rule to fall back on.\n\n'
      printf 'Regenerate it, review the diff, then re-run:\n\n'
      printf '    %s fix\n' "$PROG"
      printf '    git diff .sops.yaml\n'
    } >&2
    exit 1
  fi
  log "$PROG: sops could not create $rel:"
  cat "$err" >&2
  exit 1
}

# Reads the value twice from the terminal and leaves it in SECRET_VALUE. A
# terminal is preferred over stdin so that a redirected stdin does not silently
# swallow the prompt; when there is none — the hermetic checks, a pipeline — the
# same reads run against stdin and say so, rather than this failing on a machine
# with no controlling terminal.
SECRET_VALUE=""
read_secret_value() { # <user> <name>
  local src="/dev/tty" again=""
  if ! { : >/dev/tty; } 2>/dev/null; then
    src="/dev/stdin"
    log "$PROG: no terminal; reading the value from stdin (it will not be echoed anyway)."
  fi
  printf '%s: setting %s for %s. The value is not echoed.\n' "$PROG" "$2" "$1" >&2
  IFS= read -rs -p "  value: " SECRET_VALUE <"$src" || die "no value read"
  printf '\n' >&2
  IFS= read -rs -p "  again: " again <"$src" || die "no confirmation read"
  printf '\n' >&2
  [ -n "$SECRET_VALUE" ] || die "the value is empty; refusing to store it"
  [ "$SECRET_VALUE" = "$again" ] || die "the two entries differ; nothing was written"
}

# --- Generators -------------------------------------------------------------------
# The run plan, cached for this run: which generators a user has, in which order
# they may run, what each writes, and the name space its script addresses its
# inputs by. A third `nix eval` beside placements and audiences, for the reason
# the second one is separate: the selftest's `nix` stub dispatches on the
# attribute each call names, so a rename fails a check rather than a rotation.
GENPLAN=""

load_generator_plan() {
  [ -z "$GENPLAN" ] || return 0
  GENPLAN="$(mktemp)"
  SCRATCH_FILES+=("$GENPLAN")
  "$NIX" eval --json "$REPO_ROOT#safix.lib.generatorPlan" >"$GENPLAN" \
    || die "could not evaluate flake.safix.lib.generatorPlan in $REPO_ROOT"
}

# Everything on stdin, into CAPTURED, without a command substitution and without
# a herestring. `read -d ''` stops at a NUL the stream does not contain, so it
# consumes to end of input and reports failure with the data already in the
# variable, which is why the `|| true` is not swallowing an error. The two
# shorter spellings are avoided for the same reason the value path avoids them
# everywhere else in this command: a herestring materializes its operand in
# $TMPDIR, where a shredder cannot reach what it never registered.
CAPTURED=""
capture_stdin() {
  CAPTURED=""
  IFS= read -r -d '' CAPTURED || true
}

# One trailing newline comes off a single-line value and nothing comes off a
# multi-line one. `openssl rand -base64 32` and every other echo-shaped one-liner
# ends in a newline it did not mean, and storing it would put a stray byte in
# every consumer; an OpenSSH private key ends in a newline it did mean, and
# taking it off produces a file `ssh` refuses to load. The two cases are
# distinguishable — after removing the final newline, a single-line value has no
# newline left — so they are distinguished rather than settled one way.
strip_generated_newline() {
  case "$CAPTURED" in
    *$'\n') ;;
    *) return 0 ;;
  esac
  case "${CAPTURED%$'\n'}" in
    *$'\n'*) ;;
    *) CAPTURED="${CAPTURED%$'\n'}" ;;
  esac
}

# Whether a name already holds a value, answered off the ciphertext. `check` asks
# this about people whose files it cannot decrypt, so it may not decrypt to find
# out; `generate` asks it to decide whether a run is a mint or a rotation, and
# asking the same way keeps the two from disagreeing about the same file.
has_value() { # <relpath> <key>
  local abs="$REPO_ROOT/$1" key="$2"
  [ -e "$abs" ] || return 1
  "$KEYS_OF" "$abs" | jq -e --arg k "$key" 'has($k) and (.[$k].empty | not)' >/dev/null
}

# ── how a generator receives its inputs ──
# Each prompt and each dependency reaches the script as `$in_<name>`, holding the
# path of a read-only file descriptor this process opened and the script
# inherits. `-` becomes `_` so the name is a spellable shell identifier; two
# inputs colliding under that mapping are refused at evaluation
# (modules/flake/safix/resolve.nix), so the script's name space is injective.
#
# A descriptor rather than a directory of files. The directory shape needs
# $TMPDIR to be memory-backed to be equivalent, and on a machine where it is not,
# plaintext written there is plaintext on a disk, surviving in free blocks after
# any unlink. So the value goes down a pipe and is never a file at all.
#
# The consequence to know when writing a script: a pipe is read once. `cat
# "$in_x"` twice gives the value and then nothing. Read it into a variable if the
# script needs it twice.
GEN_ENV=()
GEN_FDS=()

open_dependency_input() { # <shell-name> <relpath> <key>
  local shellname="$1" abs="$REPO_ROOT/$2" key="$3" index fd
  [ -e "$abs" ] || die "the dependency behind \$in_$shellname has no value yet: $2 does not exist"
  index="$(jq -nc --arg k "$key" '[$k]')"
  exec {fd}< <("$SOPS" decrypt --extract "$index" "$abs" </dev/null)
  GEN_ENV+=("in_$shellname=/dev/fd/$fd")
  GEN_FDS+=("$fd")
}

# The value is in PROMPT_VALUE, and `printf` is a shell builtin, so the forked
# subshell that feeds the pipe execs nothing and the value never becomes an
# argument vector anyone can read.
PROMPT_VALUE=""
open_prompt_input() { # <shell-name>
  local shellname="$1" fd
  exec {fd}< <(printf '%s' "$PROMPT_VALUE")
  GEN_ENV+=("in_$shellname=/dev/fd/$fd")
  GEN_FDS+=("$fd")
}

# Every descriptor this generator was given, closed before the next one starts.
# Each carried a decrypted value, so one surviving into a later generator's
# process is that generator holding plaintext it never declared and the command
# cannot account for. The `generate-isolation` check is what holds this: it
# compares the descriptors a generator running last sees against those one
# running first sees, and a close that stops happening makes the two differ.
#
# The number is closed through `eval` rather than as `exec {fd}<&-` only so the
# guard above it can be written at all; bash 5.3 returns 0 from either spelling
# when the descriptor is already gone, so the guard is defensive rather than
# load-bearing. Note the direction of its risk: a guard whose test stops
# matching skips the close entirely, which is the leak, and is why the check
# asserts the descriptors rather than that the loop ran.
close_generator_inputs() {
  local fd
  for fd in "${GEN_FDS[@]}"; do
    [ -e "/dev/fd/$fd" ] || continue
    eval "exec ${fd}<&-"
  done
  GEN_FDS=()
  GEN_ENV=()
}

# Reads one prompt into PROMPT_VALUE. The terminal is preferred over stdin for
# the reason `read_secret_value` prefers it, and the stdin fallback is what makes
# a generator with prompts drivable from a pipe — which is how the hermetic
# checks drive one. There is deliberately no environment variable carrying a
# prompt's answer: a value in the environment is a value in /proc/<pid>/environ,
# and an affordance that puts one there for the tests' convenience is an
# affordance production can reach.
read_prompt() { # <kind> <name> <description>
  local kind="$1" name="$2" description="$3" src="/dev/tty" line
  if ! { : >/dev/tty; } 2>/dev/null; then
    src="/dev/stdin"
    log "$PROG: no terminal; reading '$name' from stdin (it will not be echoed anyway)."
  fi
  PROMPT_VALUE=""
  case "$kind" in
    hidden)
      IFS= read -rs -p "  $name ($description): " PROMPT_VALUE <"$src" || die "no value read for prompt '$name'"
      printf '\n' >&2
      ;;
    line)
      IFS= read -r -p "  $name ($description): " PROMPT_VALUE <"$src" || die "no value read for prompt '$name'"
      ;;
    multiline)
      printf '  %s (%s), ending with a line reading EOF:\n' "$name" "$description" >&2
      while IFS= read -r line <"$src"; do
        [ "$line" = "EOF" ] && break
        PROMPT_VALUE="$PROMPT_VALUE$line"$'\n'
      done
      ;;
    *) die "unknown prompt type '$kind' for '$name'" ;;
  esac
  [ -n "$PROMPT_VALUE" ] || die "prompt '$name' was answered with nothing; refusing to generate from an empty input"
}

# `nix shell` with the entry's `runtimeInputs`, resolved against the flake's own
# locked nixpkgs through `--inputs-from` so a generator mints the same value from
# the same declaration on every machine. The inputs are PREPENDED to the caller's
# PATH rather than replacing it, so a script that reaches a tool it did not
# declare works for whoever wrote it and fails for everyone else: name every tool
# the script runs in `runtimeInputs`.
#
# The script's exit status travels through a file rather than through the pipe,
# because a process substitution's status is not reportable. The file holds an
# exit code and never a value.
GEN_STATUS=0
run_in_generator_shell() { # <script> <runtime-inputs-json> ; extra env in GEN_ENV
  local script="$1" inputs="$2" status_file pkg
  local -a specs=()
  while IFS= read -r pkg; do
    [ -n "$pkg" ] || continue
    specs+=("nixpkgs#$pkg")
  done < <(printf '%s' "$inputs" | jq -r '.[]')
  status_file="$(mktemp)"
  SCRATCH_FILES+=("$status_file")
  # Standard input is /dev/null, and that is part of the interface rather than
  # tidiness. A generator's inputs are its descriptors; the command's own stdin
  # is where an operator's prompt answers arrive, and a script that read stdin
  # would eat the answers to every prompt after it — silently, since a prompt
  # that reads end-of-input looks exactly like one nobody answered.
  # The status is recorded through an `if` rather than after the fact. A process
  # substitution inherits `errexit`, so a bare `$?` on the next line is a line
  # the subshell never reaches when the generator fails — which is exactly the
  # run whose status matters, and it would be reported as a generator that
  # printed nothing rather than one that failed.
  capture_stdin < <(
    if env "${GEN_ENV[@]}" "$NIX" shell --inputs-from "$REPO_ROOT" "${specs[@]}" \
      -c bash -euo pipefail -c "$script" </dev/null; then
      printf '0' >"$status_file"
    else
      printf '%s' "$?" >"$status_file"
    fi
  )
  GEN_STATUS="$(cat "$status_file")"
}

# The entry's `validation` fragment, judging one candidate value handed to it on
# standard input. `$out_name` names which output is being judged, so one fragment
# can cover a generator that writes several. A non-zero exit refuses the whole
# run: the values are still only in this process's memory at that point, so
# nothing has to be undone.
#
# Same shell and the same `runtimeInputs` as the script, because a validation
# that could not run the tool that produced the value would be able to check
# almost nothing about it.
run_validation() { # <script> <runtime-inputs-json> <out-name> ; value on stdin
  local script="$1" inputs="$2" outname="$3" pkg
  local -a specs=()
  while IFS= read -r pkg; do
    [ -n "$pkg" ] || continue
    specs+=("nixpkgs#$pkg")
  done < <(printf '%s' "$inputs" | jq -r '.[]')
  env "out_name=$outname" "$NIX" shell --inputs-from "$REPO_ROOT" "${specs[@]}" \
    -c bash -euo pipefail -c "$script"
}

# --- Subcommands -----------------------------------------------------------------
cmd_set() { # <user> <name>
  local user="$1" name="$2" abs dir work index
  resolve_placement "$user" "$name"
  abs="$REPO_ROOT/$PLACE_FILE"
  dir="$(dirname "$abs")"
  refuse_bad_repo_state "$PLACE_FILE"

  # Beside the target, so the move into place is an atomic rename rather than a
  # cross-filesystem copy that can be interrupted half-written, and keeping the
  # `.yaml` suffix, because sops reads a document's format off the extension and
  # would parse a `*.tmp.1234` YAML file as JSON.
  work="$abs.$PROG-tmp.$$.yaml"
  SCRATCH_FILES+=("$work")
  index="$(jq -nc --arg k "$PLACE_KEY" '[$k]')"

  log "$PROG: $name ($PLACE_ORIGIN, owner $PLACE_OWNER) -> $PLACE_FILE [$PLACE_KEY]"
  if [ -e "$abs" ]; then
    cp -p "$abs" "$work"
  else
    if [ ! -d "$dir" ]; then
      mkdir -p "$dir"
      SCRATCH_DIRS+=("$dir")
    fi
    note "$PLACE_FILE does not exist yet; creating it through sops so the creation rules apply."
    create_through_sops "$PLACE_FILE" "$PLACE_KEY" "$work"
  fi

  read_secret_value "$user" "$name"
  printf '%s' "$SECRET_VALUE" | jq -Rs . \
    | "$SOPS" set --value-stdin --idempotent --input-type yaml --output-type yaml "$work" "$index"

  refuse_recipient_drift "$PLACE_FILE" "$work"

  mv "$work" "$abs"
  SCRATCH_DIRS=()
  commit_written_files "chore(safix): set $name for $user" "$PLACE_FILE"
}

# One decision point, and it is git's rather than a byte comparison of our own:
# `sops set --idempotent` leaves an unchanged value's file untouched, so a re-run
# moves a byte-identical file into place and git has nothing staged.
#
# Scoped to the files written on both halves. An unscoped `git diff --cached`
# would read another path's staged change as this command's work and commit on a
# run that wrote nothing, and an unscoped `git commit` would carry that path into
# a commit whose message names one secret; `git commit -- <path>...` commits
# those paths alone and leaves the rest of the index staged.
#
# More than one path only ever arrives from one generator writing more than one
# output. A keypair split across two commits is a state in which the tree holds a
# private half and a public half that do not match, so the outputs of one run go
# in together or not at all.
commit_written_files() { # <message> <relpath>...
  local message="$1"
  shift
  "$GIT" -C "$REPO_ROOT" add -- "$@"
  if "$GIT" -C "$REPO_ROOT" diff --cached --quiet -- "$@"; then
    note "unchanged — the file already holds this value, so nothing was committed."
    return 0
  fi
  "$GIT" -C "$REPO_ROOT" commit -q -m "$message" -- "$@"
  note "committed $("$GIT" -C "$REPO_ROOT" rev-parse --short HEAD) — the value is not in the message."
}

cmd_get() { # <user> <name>
  local user="$1" name="$2" abs index
  resolve_placement "$user" "$name"
  abs="$REPO_ROOT/$PLACE_FILE"
  [ -e "$abs" ] || die "$PLACE_FILE does not exist yet, so '$name' has no value. Set one with: $PROG set $user $name"
  index="$(jq -nc --arg k "$PLACE_KEY" '[$k]')"
  "$SOPS" decrypt --extract "$index" "$abs"
}

# --- The governed files, and the drift report over them ----------------------------
# Which files the recipient policy governs, in three sets. `required` is computed
# from the audiences the declarations imply. `extra` is what the consumer named
# through `flake.safix.extraGovernedFiles`, for files that ride an existing rule
# but that no declaration places a secret in. `managed` is their union, and is
# what `fix` re-wraps: a file judged out of policy that no sanctioned command can
# name is a file that can only drift further.
GOVERNED=""

load_governed_files() {
  [ -z "$GOVERNED" ] || return 0
  GOVERNED="$(mktemp)"
  SCRATCH_FILES+=("$GOVERNED")
  "$NIX" eval --json "$REPO_ROOT#safix.lib.governedFiles" >"$GOVERNED" \
    || die "could not evaluate flake.safix.lib.governedFiles in $REPO_ROOT"
}

RECIPIENTS=""

load_recipients() {
  [ -z "$RECIPIENTS" ] || return 0
  RECIPIENTS="$(mktemp)"
  SCRATCH_FILES+=("$RECIPIENTS")
  "$NIX" eval --json "$REPO_ROOT#safix.lib.recipients" >"$RECIPIENTS" \
    || die "could not evaluate flake.safix.lib.recipients in $REPO_ROOT"
}

# Which declared users hold any of these age keys, and which keys belong to no
# declared user at all. A key on a file that no longer answers to a name is the
# more alarming of the two and must not be swallowed by reporting only the names
# that matched, so both halves come back.
holders_of() { # <keys-json> -> "<name>..." on stdout, unmatched keys on the second line
  jq -r --argjson keys "$1" '
      (to_entries | map(select(any(.value[]; . as $k | $keys | index($k) != null))) | map(.key)) as $named
    | ([.[][]]) as $known
    | ($keys | map(select(. as $k | $known | index($k) == null))) as $orphan
    | ($named | join(" ")), ($orphan | join(" "))
  ' "$RECIPIENTS"
}

# The (file, key) pairs `check_shared` has already reported. A stray copy of a
# shared name is an unclaimed value too, and reporting it twice under two
# remedies — one of which is "delete it", the other "declare it" — would invite
# the wrong one.
SHARED_STRAYS=""

CHECK_FINDINGS=0

finding() { # <headline>
  CHECK_FINDINGS=$((CHECK_FINDINGS + 1))
  printf '\n%s\n' "$*" >&2
}

remedy() { printf '    %s\n' "$*" >&2; }

# The committed `.sops.yaml` against the one the declarations imply. The sops CLI
# reads the file off disk rather than out of an evaluation, so the file is an
# artifact that has to be regenerated and committed and can therefore be stale in
# a way nothing else here can.
check_policy() {
  local generated
  generated="$(mktemp)"
  SCRATCH_FILES+=("$generated")
  "$NIX" eval --raw "$REPO_ROOT#safix.lib.policyText" >"$generated" \
    || die "could not evaluate flake.safix.lib.policyText in $REPO_ROOT"
  if [ ! -e "$REPO_ROOT/.sops.yaml" ]; then
    finding ".sops.yaml does not exist, so no creation rule covers any file."
    remedy "$PROG fix"
    return 0
  fi
  cmp -s "$generated" "$REPO_ROOT/.sops.yaml" && return 0
  finding ".sops.yaml differs from the policy flake.safix.users implies."
  remedy "$PROG fix"
  remedy "git diff .sops.yaml"
}

# Each governed file's actual recipients against the audience declared for it,
# read the same way the pre-write assertion reads them, so the two cannot reach
# different answers about one file's stanzas.
#
# The two halves of the governed set are judged differently because they are
# different claims. A `required` file has an audience of its own, computed from
# the declarations, and drift is that file's stanzas disagreeing with it. An
# `extra` file has no audience — no declaration places a secret in it — so what
# holds it is the rule whose directory covers it, which is also exactly what
# `sops updatekeys` will re-wrap it to. A path no rule's directory covers is
# reported as such: naming a file in `extraGovernedFiles` does not create a rule
# for it, and encryption into it fails closed.
check_recipients() {
  local f declared result extra missing dir
  load_audiences
  load_governed_files
  while IFS= read -r f; do
    [ -e "$REPO_ROOT/$f" ] || continue
    declared="$(mktemp)"
    SCRATCH_FILES+=("$declared")
    if jq -e --arg f "$f" 'has($f)' "$AUDIENCES" >/dev/null; then
      jq --arg f "$f" '.[$f].recipients' "$AUDIENCES" >"$declared"
    else
      dir="$(dirname "$f")"
      jq --arg d "$dir" '[to_entries[] | select(.value.dir == $d)] | (.[0].value.recipients // null)' \
        "$AUDIENCES" >"$declared"
      if jq -e 'type == "null"' "$declared" >/dev/null; then
        finding "$f is named in flake.safix.extraGovernedFiles and no creation rule's directory covers it, so nothing declares who should be able to open it and \`$PROG fix\` cannot re-wrap it."
        remedy "move it beside the secrets of the audience it belongs to, or drop it from flake.safix.extraGovernedFiles"
        continue
      fi
    fi
    result="$("$RECIPIENTS_OF" "$REPO_ROOT/$f" "$declared")" \
      || die "could not read the recipients of $f"
    extra="$(printf '%s' "$result" | jq -r '.extra[]')"
    missing="$(printf '%s' "$result" | jq -r '.missing[]')"
    [ -n "$extra$missing" ] || continue
    finding "$f is not encrypted to the audience declared for it."
    if [ -n "$extra" ]; then
      printf '  can open it and is not in its audience:\n' >&2
      printf '%s\n' "$extra" | sed 's/^/    - /' >&2
    fi
    if [ -n "$missing" ]; then
      printf '  is in its audience and cannot open it:\n' >&2
      printf '%s\n' "$missing" | sed 's/^/    - /' >&2
    fi
    remedy "$PROG fix"
    remedy "git diff -- $f"
  done < <(jq -r '.managed[]' "$GOVERNED")
}

# A shared entry is one value, so a copy of its key anywhere but the file its
# audience picks is a second value the audience does not hold.
#
# Every way an audience can change produces one, because a file is named for its
# members: adding a carrier or dropping one moves the entry to a different file
# rather than re-wrapping the file it was in, so `fix` alone never resolves this
# and the value has to be re-minted where the audience now reads. Flipping an
# entry to shared over per-carrier values that are already there leaves the same
# shape.
#
# Which of the two it is comes off the stray's own stanzas rather than out of any
# record of what the audience used to be. A stray a non-member can open is a
# revocation: that reader has held the data key, so re-wrapping does not unread
# what they read, and only a new value revokes. A stray every one of whose
# readers is still in the audience is a migration. Neither is `fix`'s to do — one
# needs a value minted, the other a choice between values that can disagree, and
# a tool that picked the winner would be discarding a secret someone is using.
check_shared() {
  local name file key gen carrier f declared result extra people orphans
  local -a rows
  load_placements
  load_audiences
  load_governed_files
  load_recipients

  SHARED_STRAYS="$(mktemp)"
  SCRATCH_FILES+=("$SHARED_STRAYS")
  printf '[]\n' >"$SHARED_STRAYS"

  while IFS=$'\t' read -r name file key gen carrier; do
    declared="$(mktemp)"
    SCRATCH_FILES+=("$declared")
    jq --arg f "$file" '.[$f].recipients // []' "$AUDIENCES" >"$declared"

    while IFS= read -r f; do
      [ "$f" != "$file" ] || continue
      [ -e "$REPO_ROOT/$f" ] || continue
      has_value "$f" "$key" || continue

      jq --arg f "$f" --arg k "$key" '. + [{ file: $f, key: $k }]' "$SHARED_STRAYS" >"$SHARED_STRAYS.tmp"
      mv "$SHARED_STRAYS.tmp" "$SHARED_STRAYS"

      result="$("$RECIPIENTS_OF" "$REPO_ROOT/$f" "$declared")" \
        || die "could not read the recipients of $f"
      extra="$(printf '%s' "$result" | jq -c '.extra')"

      if [ "$extra" = "[]" ]; then
        finding "flake.safix.catalogue.$name is shared, so one value in $file serves every carrier, but $f holds a value under '$key' of its own."
        printf '  Everyone who can open that copy is still in the audience, so this is a\n' >&2
        printf '  migration rather than a disclosure: the value the audience holds in common\n' >&2
        printf '  has not been minted into %s yet, and the copies left behind can disagree\n' "$file" >&2
        printf '  with each other. Which one should win is yours to say, not this tool'"'"'s.\n' >&2
        remedy "mint the value the audience is to share:"
        if [ "$gen" = "true" ]; then
          remedy "    $PROG generate --regenerate $carrier $name"
        else
          remedy "    $PROG set $carrier $name"
        fi
        remedy "then delete the superseded key:  sops $f"
        remedy "then converge the policy:        $PROG fix"
        continue
      fi

      mapfile -t rows < <(holders_of "$extra")
      people="${rows[0]:-}"
      orphans="${rows[1]:-}"
      finding "flake.safix.catalogue.$name is shared and its audience reads $file, but $f still holds a value under '$key' that someone outside that audience can open. This is a revocation."
      if [ -n "$people" ]; then
        printf '  can open the copy in %s and is no longer a carrier:\n' "$f" >&2
        printf '%s\n' "$people" | tr ' ' '\n' | sed 's/^/    - /' >&2
      fi
      if [ -n "$orphans" ]; then
        printf '  can open it and answers to no declared user:\n' >&2
        printf '%s\n' "$orphans" | tr ' ' '\n' | sed 's/^/    - /' >&2
      fi
      printf '  They have held the data key that copy is wrapped under, so re-wrapping it\n' >&2
      printf '  does not unread what they have already read. %s fix is not the remedy\n' "$PROG" >&2
      printf '  here and will not be: revoking means a value they never saw.\n' >&2
      remedy "mint a new value for the audience that remains:"
      if [ "$gen" = "true" ]; then
        remedy "    $PROG generate --regenerate $carrier $name"
      else
        remedy "    $PROG set $carrier $name"
      fi
      remedy "then delete the revoked copy:    sops $f"
      remedy "then converge the policy:        $PROG fix"
    done < <(jq -r '.managed[]' "$GOVERNED")
  done < <(jq -r '
      [ to_entries[] as $u
        | $u.value
        | to_entries[]
        | select(.value.shared)
        | { name: .key, file: .value.file, key: .value.key,
            gen: (.value.generator != null), user: $u.key } ]
    | group_by(.name)
    | map(.[0])
    | .[]
    | [.name, .file, .key, (.gen | tostring), .user]
    | @tsv
  ' "$PLACEMENTS")
}

# Names the declarations make that hold no value, and values in a file the
# declarations do place secrets in that no name claims. The two are opposite
# directions of the same question and are reported apart because their remedies
# are different: a valueless name is minted or typed, an unclaimed value is
# declared or deleted.
#
# Which remedy a valueless name gets depends on whether it has a generator, so
# the report says which, and answering that is why the reader works off the
# ciphertext: this walks every declared user, and the machine running it holds an
# identity for at most one of them.
#
# The unclaimed half walks `required` rather than `managed`. Every key in a file
# named through `extraGovernedFiles` is unclaimed by construction — that is what
# naming it there means — so reporting those would be a finding no declaration
# can ever resolve.
check_values() { # [<user>]
  local only="${1:-}" user name file key gen present
  load_placements
  while IFS= read -r user; do
    [ -z "$only" ] || [ "$only" = "$user" ] || continue
    while IFS=$'\t' read -r name file key gen; do
      if has_value "$file" "$key"; then continue; fi
      if [ "$gen" = "true" ]; then
        finding "flake.safix.users.$user declares '$name' and $file holds no value for it. It has a generator."
        remedy "$PROG generate $user $name"
      else
        finding "flake.safix.users.$user declares '$name' and $file holds no value for it. It has no generator."
        remedy "$PROG set $user $name"
      fi
    done < <(jq -r --arg u "$user" \
      '.[$u] | to_entries[] | [.key, .value.file, .value.key, (.value.generator != null | tostring)] | @tsv' \
      "$PLACEMENTS")
  done < <(declared_users)

  load_governed_files
  local claimed
  claimed="$(mktemp)"
  SCRATCH_FILES+=("$claimed")
  jq '[.[] | to_entries[] | { file: .value.file, key: .value.key }]' "$PLACEMENTS" >"$claimed"
  while IFS= read -r file; do
    [ -e "$REPO_ROOT/$file" ] || continue
    while IFS= read -r key; do
      present="$(jq -r --arg f "$file" --arg k "$key" \
        'map(select(.file == $f and .key == $k)) | length' "$claimed")"
      [ "$present" = 0 ] || continue
      if [ -n "$SHARED_STRAYS" ] && jq -e --arg f "$file" --arg k "$key" \
        'any(.[]; .file == $f and .key == $k)' "$SHARED_STRAYS" >/dev/null; then
        continue
      fi
      finding "$file holds a value under '$key' and no declaration claims it."
      remedy "declare it in flake.safix.users, or remove it with: sops $file"
    done < <("$KEYS_OF" "$REPO_ROOT/$file" | jq -r 'keys[]')
  done < <(jq -r '.required[]' "$GOVERNED")
}

cmd_check() { # [<user>]
  local only="${1:-}"
  if [ -n "$only" ]; then
    load_placements
    jq -e --arg u "$only" 'has($u)' "$PLACEMENTS" >/dev/null || refuse_unknown_user "$only"
  fi
  check_policy
  check_recipients
  check_shared
  check_values "$only"
  if [ "$CHECK_FINDINGS" = 0 ]; then
    log "$PROG: no drift. The policy, the recipients and the values all agree with the declarations."
    return 0
  fi
  printf '\n%s: %s finding(s).\n' "$PROG" "$CHECK_FINDINGS" >&2
  exit 1
}

# `fix` is the write half of `check`, and it is deliberately not all of it. It
# regenerates the policy and re-wraps each governed file to the audience that
# policy declares; it does not mint a value, delete one, or declare a name,
# because each of those is a decision rather than a convergence.
#
# The two halves run in this order and not the other. Re-wrapping first re-wraps
# to a policy that is about to change.
#
# It does not commit. Re-wrapping every governed file is a diff worth reading
# before it becomes history.
cmd_fix() { # [--yes]
  local assume_yes="${1:-}" f
  "$NIX" eval --raw "$REPO_ROOT#safix.lib.policyText" >"$REPO_ROOT/.sops.yaml.new" \
    || die "could not evaluate flake.safix.lib.policyText in $REPO_ROOT"
  mv "$REPO_ROOT/.sops.yaml.new" "$REPO_ROOT/.sops.yaml"
  log "$PROG: wrote $REPO_ROOT/.sops.yaml"

  {
    printf '\n'
    printf 'Re-wrapping aligns ciphertext with policy. It does not revoke: a person\n'
    printf 'removed from an audience has already read every value in the file, and\n'
    printf 're-wrapping the data key does not unread it. Revoking means minting a new\n'
    printf 'value — %s generate --regenerate <user> <name>, or sops <file>.\n\n' "$PROG"
  } >&2

  load_governed_files
  # Named one by one from the declarations. `sops updatekeys` over a glob is what
  # the .sops.yaml header warns about: a rule matching more than intended
  # rewrites recipients on files whose original identities are gone.
  #
  # `managed` rather than `required`, because a file the consumer named through
  # `extraGovernedFiles` rides a rule and so moves when that rule's audience
  # moves. Driving this from the narrower set would leave such a file encrypted
  # to whoever it was encrypted to when it was written, with no sanctioned
  # command able to name it.
  while IFS= read -r f; do
    if [ ! -e "$REPO_ROOT/$f" ]; then
      log "==> $f does not exist yet; create it with: sops $f"
      continue
    fi
    log "==> sops updatekeys $f"
    if [ "$assume_yes" = --yes ]; then
      (cd "$REPO_ROOT" && "$SOPS" updatekeys --yes "$f")
    else
      (cd "$REPO_ROOT" && "$SOPS" updatekeys "$f")
    fi
  done < <(jq -r '.managed[]' "$GOVERNED")

  log "$PROG: review the diff before committing it: git diff"
}

refuse_no_generator() { # <user> <name>
  {
    printf '%s: '\''%s'\'' has no generator, so there is nothing to run.\n\n' "$PROG" "$2"
    printf 'A generator is declared on the entry, beside its mode and its path:\n\n'
    printf '    generator.script = "openssl rand -base64 32";\n'
    printf '    generator.runtimeInputs = [ "openssl" ];\n\n'
    printf 'Only a value you are free to choose can have one. A credential some\n'
    printf 'server already knows is set by hand:\n\n'
    printf '    %s set %s %s\n' "$PROG" "$1" "$2"
  } >&2
  exit 1
}

# One generator: its inputs, its run, its outputs, and one commit. Returns 1
# without running when every output already holds a value and no rotation was
# asked for, so the bulk form can walk the whole order and mint only what is
# missing.
GEN_REGENERATE=0

run_one_generator() { # <user> <gen>
  local user="$1" gen="$2" gen_json script inputs validation names doc index
  local -a outs=() out_files=() out_keys=() values=() uniq_files=() work_files=()
  local missing=0 i o f

  while IFS= read -r o; do
    outs+=("$o")
  done < <(jq -r --arg u "$user" --arg g "$gen" '.[$u].outputs[$g][]' "$GENPLAN")

  for o in "${outs[@]}"; do
    resolve_placement "$user" "$o"
    out_files+=("$PLACE_FILE")
    out_keys+=("$PLACE_KEY")
    if ! has_value "$PLACE_FILE" "$PLACE_KEY"; then
      missing=$((missing + 1))
    fi
  done

  if [ "$missing" = 0 ] && [ "$GEN_REGENERATE" = 0 ]; then
    note "$gen already holds a value for every output; --regenerate rotates it."
    return 1
  fi

  # Distinct files first, because the preflight and the write both work per file
  # and two outputs of one generator share a file whenever they share an
  # audience.
  for f in "${out_files[@]}"; do
    local seen=0 u
    for u in "${uniq_files[@]}"; do
      if [ "$u" = "$f" ]; then seen=1; fi
    done
    if [ "$seen" = 0 ]; then uniq_files+=("$f"); fi
  done
  for f in "${uniq_files[@]}"; do
    refuse_bad_repo_state "$f"
  done

  gen_json="$(jq -c --arg u "$user" --arg g "$gen" '.[$u][$g].generator' "$PLACEMENTS")"
  script="$(printf '%s' "$gen_json" | jq -r '.script')"
  inputs="$(printf '%s' "$gen_json" | jq -c '.runtimeInputs')"
  validation="$(printf '%s' "$gen_json" | jq -r '.validation // ""')"

  names="$(printf '%s, ' "${outs[@]}")"
  names="${names%, }"
  log "$PROG: generating $names for $user"

  # The loop body runs in this shell — a process substitution rather than a pipe
  # — because `open_*_input` allocates a descriptor that has to outlive it and
  # be inherited by the generator.
  #
  # The plan is read through a descriptor of its own rather than through the
  # loop's standard input, which is not a style choice: `done < <(...)` makes the
  # process substitution the body's stdin too, and a prompt read inside the body
  # would then read the exhausted plan instead of the operator. It looks exactly
  # like a prompt nobody answered.
  local shellname kind iname ptype pdesc plan_fd
  while IFS=$'\t' read -r shellname kind iname <&"$plan_fd"; do
    case "$kind" in
      prompt)
        ptype="$(printf '%s' "$gen_json" | jq -r --arg n "$iname" '.prompts[$n].type')"
        pdesc="$(printf '%s' "$gen_json" | jq -r --arg n "$iname" '.prompts[$n].description')"
        read_prompt "$ptype" "$iname" "$pdesc"
        open_prompt_input "$shellname"
        PROMPT_VALUE=""
        ;;
      dependency)
        resolve_placement "$user" "$iname"
        open_dependency_input "$shellname" "$PLACE_FILE" "$PLACE_KEY"
        ;;
      *) die "unknown input kind '$kind' for '$iname'" ;;
    esac
  done {plan_fd}< <(jq -r --arg u "$user" --arg g "$gen" \
    '.[$u].inputs[$g] | to_entries[] | [.key, .value.kind, .value.name] | @tsv' "$GENPLAN")
  eval "exec ${plan_fd}<&-"

  run_in_generator_shell "$script" "$inputs"
  close_generator_inputs
  [ "$GEN_STATUS" = 0 ] \
    || die "the generator for '$gen' exited $GEN_STATUS; nothing was written. Its diagnostics are above, on stderr."

  # One output takes the script's standard output as the value; several take a
  # JSON object keyed by output name. The fork is not a convenience: a shell's
  # standard output is a byte stream with no way to say "these two values", and
  # a stream that had to carry a separator would make every value that could
  # contain the separator unstorable.
  #
  # No newline comes off a value read out of the JSON form, either. JSON states
  # a string exactly, so there is no echo-shaped artifact to remove and removing
  # one would corrupt a value that meant it.
  if [ "${#outs[@]}" = 1 ]; then
    strip_generated_newline
    values+=("$CAPTURED")
  else
    doc="$CAPTURED"
    printf '%s' "$doc" | jq -e 'type == "object"' >/dev/null 2>&1 \
      || die "'$gen' writes ${#outs[@]} outputs, so its script must print a JSON object keyed by output name; it printed something else"
    local declared actual
    declared="$(printf '%s\n' "${outs[@]}" | jq -Rnc '[inputs] | sort')"
    actual="$(printf '%s' "$doc" | jq -c 'keys')"
    [ "$declared" = "$actual" ] \
      || die "'$gen' printed keys $actual but declares outputs $declared; nothing was written"
    for o in "${outs[@]}"; do
      capture_stdin < <(printf '%s' "$doc" | jq -j --arg k "$o" '.[$k]')
      values+=("$CAPTURED")
    done
    doc=""
  fi
  CAPTURED=""

  for i in "${!outs[@]}"; do
    [ -n "${values[$i]}" ] \
      || die "'$gen' produced nothing for '${outs[$i]}'; an empty value is the state a truncated write leaves behind, so it is refused"
    if [ -n "$validation" ]; then
      printf '%s' "${values[$i]}" | run_validation "$validation" "$inputs" "${outs[$i]}" \
        || die "the validation for '$gen' rejected the candidate value for '${outs[$i]}'; nothing was written"
    fi
  done

  for f in "${uniq_files[@]}"; do
    local abs dir work first_key
    abs="$REPO_ROOT/$f"
    dir="$(dirname "$abs")"
    work="$abs.$PROG-tmp.$$.yaml"
    SCRATCH_FILES+=("$work")
    if [ -e "$abs" ]; then
      cp -p "$abs" "$work"
    else
      if [ ! -d "$dir" ]; then
        mkdir -p "$dir"
        SCRATCH_DIRS+=("$dir")
      fi
      first_key=""
      for i in "${!outs[@]}"; do
        if [ "${out_files[$i]}" = "$f" ] && [ -z "$first_key" ]; then first_key="${out_keys[$i]}"; fi
      done
      note "$f does not exist yet; creating it through sops so the creation rules apply."
      create_through_sops "$f" "$first_key" "$work"
    fi
    for i in "${!outs[@]}"; do
      if [ "${out_files[$i]}" != "$f" ]; then continue; fi
      index="$(jq -nc --arg k "${out_keys[$i]}" '[$k]')"
      printf '%s' "${values[$i]}" | jq -Rs . \
        | "$SOPS" set --value-stdin --idempotent --input-type yaml --output-type yaml "$work" "$index"
    done
    # Once per file rather than once per key: recipients are a property of the
    # file, and the document judged is the one holding every key this run writes,
    # so the assertion covers the bytes that are about to land.
    refuse_recipient_drift "$f" "$work"
    work_files+=("$work")
  done

  for i in "${!uniq_files[@]}"; do
    mv "${work_files[$i]}" "$REPO_ROOT/${uniq_files[$i]}"
  done
  SCRATCH_DIRS=()
  commit_written_files "chore(safix): generate $names for $user" "${uniq_files[@]}"
  return 0
}

# Every generator that would derive from <gen>'s output, <gen> first, in the
# run plan's own order.
#
# The graph is read off the plan the resolver computed rather than restated: a
# generator's `inputs` name each dependency, and `outputs` says which generator
# writes each name, so the producer of a dependency is exactly the edge
# `generatorEdges` draws. What makes one forward pass over `order` sufficient is
# that `order` is topological — a generator appears after everything it reads —
# which is the resolver's claim, guarded by the cycle refusal that makes an
# order exist at all. A dependency nobody generates resolves to no producer and
# contributes no edge, the same way it contributes none at evaluation.
generator_cascade() { # <user> <gen> -> one name per line
  jq -r --arg u "$1" --arg g "$2" '
    .[$u] as $p
    | ($p.outputs | to_entries | map(.key as $g | .value[] | { key: ., value: $g }) | from_entries) as $producer
    | (reduce $p.order[] as $n ([$g];
        . as $marked
        | if ($marked | index($n)) then $marked
          else
            ( ($p.inputs[$n] // {})
              | to_entries
              | map(select(.value.kind == "dependency") | $producer[.value.name] // empty)
              | any(. as $q | $marked | index($q) != null) ) as $derives
            | if $derives then $marked + [$n] else $marked end
          end)) as $marked
    | $p.order | map(select(. as $n | $marked | index($n) != null))[]
  ' "$GENPLAN"
}

# A rotation that stops at the value it was asked to rotate leaves every value
# derived from it standing, and a derived value outlives its input silently:
# nothing in the tree records which run it came from, so a hash of a retired
# password reads exactly like a hash of the current one. So `--regenerate` of a
# named generator carries the whole downstream set, and says so before it starts
# rather than after — the re-runs commit as they go, and a run the operator did
# not want cannot be taken back out of history by declining it afterwards.
confirm_cascade() { # <user> <gen> <assume-yes> ; cascade in CASCADE
  local gen="$2" assume_yes="$3" src="/dev/tty" answer n
  [ "${#CASCADE[@]}" -gt 1 ] || return 0
  {
    printf '\n%s: %s outputs are read by %s other generator(s), which this\n' \
      "$PROG" "$gen" "$((${#CASCADE[@]} - 1))"
    printf 'rotation retires the input of. All of them re-run, in this order:\n\n'
    for n in "${CASCADE[@]}"; do printf '    %s\n' "$n"; done
    printf '\nEach commits as it goes. Leaving them alone would leave values derived\n'
    printf 'from the value being replaced, which nothing afterwards can tell apart\n'
    printf 'from values derived from the new one.\n\n'
  } >&2
  if [ "$assume_yes" = --yes ]; then
    log "$PROG: --yes given; re-running all ${#CASCADE[@]}."
    return 0
  fi
  if ! { : >/dev/tty; } 2>/dev/null; then
    src="/dev/stdin"
    log "$PROG: no terminal; reading the confirmation from stdin."
  fi
  IFS= read -r -p "  re-run all ${#CASCADE[@]}? [y/N] " answer <"$src" || answer=""
  case "$answer" in
    y | Y | yes | YES) ;;
    *) die "declined; nothing was written. Pass --yes to answer this in advance." ;;
  esac
}

CASCADE=()

cmd_generate() { # <user> [<name>] [--yes]
  local user="$1" want="${2:-}" assume_yes="${3:-}" producer g ran=0
  local -a order=()
  load_placements
  load_generator_plan
  jq -e --arg u "$user" 'has($u)' "$PLACEMENTS" >/dev/null || refuse_unknown_user "$user"

  if [ -n "$want" ]; then
    jq -e --arg u "$user" --arg n "$want" '.[$u] | has($n)' "$PLACEMENTS" >/dev/null \
      || refuse_unknown_name "$user" "$want"
    # An output of a multi-output generator is named by its own name, not by the
    # entry the generator hangs off, so naming either half of a keypair runs the
    # one generator that mints both.
    producer="$(jq -r --arg u "$user" --arg n "$want" \
      '.[$u].outputs | to_entries | map(select(.value | index($n))) | (.[0].key // "")' "$GENPLAN")"
    [ -n "$producer" ] || refuse_no_generator "$user" "$want"
    order=("$producer")
    # Only a rotation cascades. A first mint of one name leaves nothing derived
    # from a value that no longer exists, and the bulk form already walks every
    # generator in this same order.
    if [ "$GEN_REGENERATE" = 1 ]; then
      CASCADE=()
      while IFS= read -r g; do
        CASCADE+=("$g")
      done < <(generator_cascade "$user" "$producer")
      confirm_cascade "$user" "$producer" "$assume_yes"
      order=("${CASCADE[@]}")
    fi
  else
    while IFS= read -r g; do
      order+=("$g")
    done < <(jq -r --arg u "$user" '.[$u].order[]' "$GENPLAN")
  fi

  if [ "${#order[@]}" = 0 ]; then
    log "$PROG: flake.safix.users.$user declares no generator."
    return 0
  fi

  # Dependency order, decided at evaluation and walked here, so a generator that
  # reads another's output finds one.
  for g in "${order[@]}"; do
    if run_one_generator "$user" "$g"; then ran=$((ran + 1)); fi
  done
  note "$ran generator(s) ran."
}

cmd_list() { # <user>
  local user="$1"
  load_placements
  jq -e --arg u "$user" 'has($u)' "$PLACEMENTS" >/dev/null || refuse_unknown_user "$user"
  jq -r --arg u "$user" '
    .[$u]
    | to_entries
    | if length == 0 then "flake.safix.users.\($u) holds no secret." else
        (["NAME", "ORIGIN", "SHARED", "GENERATOR", "KEY", "FILE"],
         (.[] | [
            .key,
            .value.origin,
            (if .value.shared then "yes" else "-" end),
            (if .value.generator == null then "-" else (.value.generator.description // "yes") end),
            .value.key,
            .value.file
          ]))
        | @tsv
      end
  ' "$PLACEMENTS" | column -t -s$'\t'
}

# An age identity for a person who has none, written where sops looks for one.
#
# Custody is the whole of what makes this delicate. The identity this mints is
# the private half of what will become that person's `recipient`, and everything
# they own is encrypted to it. It is therefore meant to run on their own machine,
# under their own account, and what leaves this command is the public half alone.
#
# It appends. `age-keygen -o <file>` refuses an existing file outright, which is
# the right refusal for the wrong shape here: sops reads every identity in
# keys.txt and tries each, so a second identity beside a first is a working
# state, and truncating the file is how someone loses the key to everything they
# hold. Appending never rewrites a line that is already there.
cmd_keygen() { # <user> <for-someone-else>
  local user="$1" others="$2" keyfile dir pub errfile
  load_placements
  jq -e --arg u "$user" 'has($u)' "$PLACEMENTS" >/dev/null || refuse_unknown_user "$user"

  if [ "$user" != "${USER:-$(id -un)}" ] && [ "$others" != --for-someone-else ]; then
    {
      printf '%s: '\''%s'\'' is not you, and this writes a private key into your own\n' "$PROG" "$user"
      printf 'identity file.\n\n'
      printf 'An age identity is custody: everything encrypted to its public half is\n'
      printf 'readable by whoever holds this private half. Minting one for another\n'
      printf 'person on your machine means you hold their key, which is the opposite of\n'
      printf 'the independent custody this package is built on. They should run this\n'
      printf 'themselves and hand you the public half.\n\n'
      printf 'If you have decided otherwise anyway, say so:\n\n'
      printf '    %s keygen --for-someone-else %s\n' "$PROG" "$user"
    } >&2
    exit 1
  fi
  if [ "$others" = --for-someone-else ]; then
    log "$PROG: minting an identity for '$user' in YOUR identity file. You will hold their private key and be able to read everything they own."
  fi

  keyfile="${SAFIX_AGE_KEY_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/sops/age/keys.txt}"
  dir="$(dirname "$keyfile")"
  mkdir -p "$dir"
  chmod 700 "$dir"
  if [ -e "$keyfile" ]; then
    note "$keyfile already holds an identity; appending. Nothing already in it is rewritten, and sops tries every identity in the file."
  fi

  errfile="$(mktemp)"
  SCRATCH_FILES+=("$errfile")
  # stdout is the identity and goes straight into the file; the public half is
  # the only thing age-keygen puts on stderr, and the only thing this reads.
  (
    umask 077
    age-keygen >>"$keyfile" 2>"$errfile"
  ) || die "age-keygen failed; nothing was appended"
  chmod 600 "$keyfile"
  pub="$(sed -n 's/^Public key: //p' "$errfile")"
  [ -n "$pub" ] || die "age-keygen wrote no public key; check $keyfile before re-running"

  {
    printf '\n%s: appended an identity for %s to %s\n\n' "$PROG" "$user" "$keyfile"
    printf 'The private half stays in that file and is not printed. Hand over the\n'
    printf 'public half, which is public data:\n\n'
    printf '    %s\n\n' "$pub"
    printf 'It becomes their recipient:\n\n'
    printf '    flake.safix.users.%s.recipient = "%s";\n\n' "$user" "$pub"
    printf 'Then re-wrap the files their audience now names, and review the diff:\n\n'
    printf '    %s fix\n' "$PROG"
    printf '    git diff\n\n'
    printf 'An existing ssh key can be a recipient instead of a fresh identity:\n'
    printf 'ssh-to-age reads an ed25519 public key and prints the age recipient for\n'
    printf 'it, and sops.age.sshKeyPaths names the private half.\n'
  } >&2
}

# --- adduser ----------------------------------------------------------------------
# Declaring a person is a declaration edit rather than a secret operation. This
# writes the one file that says who they are, regenerates the recipient policy
# that declaration implies, and commits the two together. Nothing is encrypted,
# nothing is decrypted, and no key material is minted or read — which is what
# lets it run before its subject holds anything at all.
#
# The recipient is an argument and never generated here, for the reason `keygen`
# refuses at length: minting it on this machine would mean this operator held
# their private half, which is the custody inversion the package is built to
# avoid. They run `safix keygen` on their own machine and hand over the public
# string.
#
# Everything beyond that is a consumer's business and reaches it through the
# hook. Attaching an account on a host, allocating an identifier, editing a
# host's module imports: each is a property of one consumer's module tree, so
# safix passes the name and the recipient to `flake.safix.onboardingHook` and
# makes no assumption about what it does. No hook configured is a supported
# configuration; onboarding simply does less.

ADDUSER_HOSTS=()
ADDUSER_YES=""

# The file this scaffolds into. safix imposes no layout on declarations — an
# attrset option merges from anywhere, so this file resolves the same wherever
# it sits — but a scaffold has to choose a path, and it chooses one under a
# directory of safix's own rather than guessing at the consumer's. Moving it
# afterwards is safe and the epilogue says so.
scaffold_path() { # <name>
  printf 'safix/users/%s.nix' "$1"
}

# `builtins.match` anchors the whole string; bash's `=~` does not, so the anchors
# are added to the exported pattern rather than assumed of it.
refuse_bad_user_name() { # <name>
  local name="$1" pattern
  pattern="$("$NIX" eval --raw "$REPO_ROOT#safix.lib.nameRegex")" \
    || die "could not evaluate flake.safix.lib.nameRegex in $REPO_ROOT"
  [[ "$name" =~ ^${pattern}$ ]] && return 0
  {
    printf '%s: %s is not a well-formed user name.\n\n' "$PROG" "$name"
    printf 'A user name is interpolated into the path of the file their secrets are\n'
    printf 'placed in and into the path_regex of a .sops.yaml creation rule, so the\n'
    printf 'alphabet excludes everything that could act as a path separator or as a\n'
    printf 'regex metacharacter. A widened rule is how a sops updatekeys sweep\n'
    printf 'reaches a file it was never meant to touch.\n\n'
    printf 'Names match %s, anchored: lowercase letters and digits, then any of\n' "$pattern"
    printf 'those plus underscore and hyphen.\n'
  } >&2
  exit 1
}

# Shape only. Nothing here can tell whether anyone holds the private half, and a
# recipient no one can decrypt with is the one error this command cannot catch —
# which is why the epilogue says so rather than implying the check was made.
refuse_bad_recipient() { # <recipient>
  local key="$1"
  case "$key" in
    age1yubikey1*)
      {
        printf '%s: a recipient that needs a physical interaction cannot be the\n' "$PROG"
        printf 'primary one.\n\n'
        printf 'Decrypting to %s...\n' "${key:0:20}"
        printf 'requires the card present, a PIN and a touch, once per file. Activation\n'
        printf 'decrypts non-interactively, with the identity sops.age.sshKeyPaths\n'
        printf 'names, so a profile whose only recipient is a hardware key cannot\n'
        printf 'activate at all — and the failure lands at switch time on their\n'
        printf 'machine, not here.\n\n'
        printf 'A card belongs in the same person'"'"'s recoveryRecipients, which is\n'
        printf 'additive: every file their audience names is encrypted to it as well as\n'
        printf 'to recipient, so the card opens their files after the activation key is\n'
        printf 'lost and is needed at no other time.\n\n'
        printf 'Pass their software recipient here, then add the card by hand.\n'
      } >&2
      exit 1
      ;;
  esac
  # bech32: the alphabet excludes 1, b, i and o, and an age X25519 recipient is
  # the prefix plus 58 of those characters.
  [[ "$key" =~ ^age1[02-9ac-hj-np-z]{58}$ ]] && return 0
  {
    printf '%s: %s is not a well-formed age recipient.\n\n' "$PROG" "$key"
    printf 'An age X25519 recipient is age1 followed by 58 bech32 characters\n'
    printf '(no 1, b, i or o). This checks the shape and nothing else: whether anyone\n'
    printf 'holds the private half is not knowable from here.\n\n'
    printf 'They mint one with %s keygen on their own machine, or convert an\n' "$PROG"
    printf 'ed25519 ssh key they already hold:\n\n'
    printf '    ssh-to-age -i ~/.ssh/id_ed25519.pub\n'
  } >&2
  exit 1
}

refuse_existing_user() { # <name>
  local name="$1" rel
  rel="$(scaffold_path "$name")"
  load_placements
  if declared_users | grep -qxF -- "$name"; then
    {
      printf '%s: %s is already a declared user.\n\n' "$PROG" "$name"
      printf 'Editing an existing person is not what this does.\n\n'
      printf 'What they hold is %s list %s; changing their recipient is an edit to\n' "$PROG" "$name"
      printf 'flake.safix.users.%s.recipient followed by %s fix, which re-wraps every\n' "$name" "$PROG"
      printf 'file their audience names and is explicitly not revocation.\n'
    } >&2
    exit 1
  fi
  [ ! -e "$REPO_ROOT/$rel" ] || die "$rel already exists but declares no user; resolve that by hand before scaffolding over it"
}

# The consumer-supplied invocation, or the empty string when none is configured.
# Read through the same namespace every other attribute comes from, so a consumer
# who sets nothing is not distinguishable from one whose flake safix cannot see.
ONBOARDING_HOOK=""

load_onboarding_hook() {
  local raw
  raw="$("$NIX" eval --json "$REPO_ROOT#safix.onboardingHook")" \
    || die "could not evaluate flake.safix.onboardingHook in $REPO_ROOT"
  ONBOARDING_HOOK="$(printf '%s' "$raw" | jq -r 'if . == null then "" else . end')"
}

refuse_host_without_hook() {
  cat >&2 <<EOF
$PROG: --host was given and flake.safix.onboardingHook is unset.

safix scaffolds a person's custody declaration and regenerates the recipient
policy. Attaching an account on a host is not one of those: allocating an
identifier, writing a per-host account module and editing that host's imports
are all properties of one consumer's module tree, and safix has no way to know
its shape.

Set the hook, which receives the name, the recipient and every --host given,
and runs after the scaffolding is committed:

    flake.safix.onboardingHook = ''
      name="\$1"
      recipient="\$2"
      shift 2
      for host in "\$@"; do ... ; done
    '';

Or drop --host: onboarding without it succeeds, having done less.
EOF
  exit 1
}

write_user_nix() { # <name> <recipient> <out>
  cat >"$3" <<EOF
# $(scaffold_path "$1") — $1's custody record.
#
# Scaffolded by \`$PROG adduser\`. This file holds who can read what and nothing
# else: no account, no identifier, no profile. Move it anywhere this flake
# imports and it resolves the same — declarations merge, so where one is written
# is not something safix knows or cares about.
{
  flake.safix.users.$1 = {
    # The age public key this person's secrets are encrypted to, handed over by
    # them. A recipient, never an identity: the private half stays on their
    # machine, nothing here can decrypt anything, and this file names no private
    # key.
    #
    # recoveryRecipients is deliberately absent. With this key alone their
    # custody is independent — no one else can open what they own — and the cost
    # is that losing it makes those files unopenable by everyone, because adding
    # a recipient to a file requires decrypting it first. The mitigation that
    # keeps independence is a second recipient THEY hold, listed here before
    # their first secret is committed.
    recipient = "$2";

    # Both empty, which is what a person who holds nothing yet looks like. The
    # first name added to \`private\` is declared and selected in one stroke;
    # regenerating the policy is what writes the creation rule their file is
    # made through, so \`$PROG fix\` comes before \`$PROG set\` for the first one.
    #
    # Catalogue selection is by explicit name rather than every entry in
    # flake.safix.catalogue, so an entry added for someone else does not
    # silently join this user.
    carries = { };
    private = { };
  };
}
EOF
}

# Every generated file is parsed before anything is staged. A scaffold that does
# not parse would be committed alongside a regenerated .sops.yaml and found at
# the next evaluation, with the recipient policy already moved.
refuse_unparsable() { # <abs-path>...
  local f
  for f in "$@"; do
    case "$f" in
      *.nix) ;;
      *) continue ;;
    esac
    nix-instantiate --parse "$f" >/dev/null 2>&1 \
      || die "generated $f does not parse; nothing was staged"
  done
}

confirm_scaffold() {
  local reply
  [ "$ADDUSER_YES" = --yes ] && return 0
  printf '  scaffold this? [y/N] ' >&2
  read -r reply || reply=""
  case "$reply" in
    y | Y | yes | YES) return 0 ;;
    *) die "aborted; nothing was written" ;;
  esac
}

cmd_adduser() { # <name> <recipient>
  local name="$1" recipient="$2" rel dir host
  local written=()

  refuse_bad_user_name "$name"
  refuse_bad_recipient "$recipient"
  refuse_existing_user "$name"
  load_onboarding_hook
  if [ ${#ADDUSER_HOSTS[@]} -gt 0 ] && [ -z "$ONBOARDING_HOOK" ]; then
    refuse_host_without_hook
  fi

  rel="$(scaffold_path "$name")"
  dir="$(dirname "$REPO_ROOT/$rel")"

  {
    printf '\n%s: declare %s\n\n' "$PROG" "$name"
    printf '  %s   custody record, holds nothing yet\n' "$rel"
    printf '  .sops.yaml                regenerated from the above\n\n'
    printf '  recipient %s\n' "$recipient"
    printf '  no value is written, no key is minted.\n'
    if [ ${#ADDUSER_HOSTS[@]} -gt 0 ]; then
      printf '\n  then flake.safix.onboardingHook, with:\n'
      for host in "${ADDUSER_HOSTS[@]}"; do
        printf '    --host %s\n' "$host"
      done
    fi
    printf '\n'
  } >&2
  confirm_scaffold

  if [ ! -d "$dir" ]; then
    mkdir -p "$dir"
    SCRATCH_DIRS+=("$dir")
  fi
  write_user_nix "$name" "$recipient" "$REPO_ROOT/$rel"
  written+=("$rel")

  # Cleared before the policy is regenerated: from here the files are the
  # command's output rather than scratch, and the EXIT trap must not reclaim the
  # directory it just filled.
  SCRATCH_DIRS=()

  refuse_unparsable "$REPO_ROOT/$rel"

  # Staged before the policy is regenerated rather than after, because a flake
  # evaluation reads the files git knows about and nothing else. An untracked
  # scaffold is invisible to flake.safix.lib.policyText, so regenerating first
  # writes the policy of the declarations as they stood WITHOUT this person — a
  # .sops.yaml that looks freshly generated, carries no anchor for them, and
  # disagrees with the tree it was committed beside.
  "$GIT" -C "$REPO_ROOT" add -- "${written[@]}"

  "$NIX" eval --raw "$REPO_ROOT#safix.lib.policyText" >"$REPO_ROOT/.sops.yaml.new" \
    || die "could not evaluate flake.safix.lib.policyText in $REPO_ROOT; the scaffold is written but .sops.yaml is untouched and nothing is committed"
  mv "$REPO_ROOT/.sops.yaml.new" "$REPO_ROOT/.sops.yaml"
  written+=(".sops.yaml")

  commit_written_files "feat(safix): declare $name and regenerate the recipient policy" "${written[@]}"

  {
    printf '\n%s: %s is declared.\n\n' "$PROG" "$name"
    printf 'What was done:\n'
    printf '  - %s, holding their recipient and nothing else\n' "$rel"
    printf '  - .sops.yaml regenerated, carrying their key as an anchor\n'
    printf '  - both committed together\n\n'
    printf 'What was NOT done, because it is not safix'\''s:\n'
    printf '  - no key was minted. They run %s keygen on THEIR machine.\n' "$PROG"
    printf '  - no account, identifier, group or password hash anywhere.\n'
    printf '  - no creation rule for them yet: they hold nothing, so no audience\n'
    printf '    includes them and no rule is emitted.\n\n'
  } >&2

  run_onboarding_hook "$name" "$recipient"

  {
    printf 'What remains, and none of it is something this command may do for you:\n\n'
    printf '  the recipient — it has to be a key %s themselves holds the private\n' "$name"
    printf '    half of. Nothing here checked that, because nothing here can: only\n'
    printf '    the shape was verified. If that string did not come from them,\n'
    printf '    every file it is added to is one they cannot open.\n\n'
    printf '  their first secret — add a name to %s under private or carries,\n' "$rel"
    printf '    then %s fix to write the rule, then %s set %s <name>.\n' "$PROG" "$PROG" "$name"
  } >&2
}

# After the safix-owned scaffolding is written and committed, so that whatever
# the hook does is its own to stage and commit and this command's single-intent
# commit stays single-intent. safix makes no assumption about what it does and
# reports its exit status without interpreting it.
run_onboarding_hook() { # <name> <recipient>
  local name="$1" recipient="$2" status=0
  if [ -z "$ONBOARDING_HOOK" ]; then
    if [ ${#ADDUSER_HOSTS[@]} -gt 0 ]; then
      die "internal: --host reached the hook with none configured"
    fi
    note "flake.safix.onboardingHook is unset, so nothing further ran."
    return 0
  fi
  log "$PROG: running flake.safix.onboardingHook"
  (cd "$REPO_ROOT" && bash -euo pipefail -c "$ONBOARDING_HOOK" "$PROG-onboarding-hook" \
    "$name" "$recipient" ${ADDUSER_HOSTS[@]+"${ADDUSER_HOSTS[@]}"}) || status=$?
  if [ "$status" != 0 ]; then
    die "the onboarding hook exited $status. The scaffold and the policy are committed; whatever the hook left behind is yours to review."
  fi
  note "the hook ran; anything it wrote is uncommitted and yours to review."
}

usage() {
  cat >&2 <<EOF
$PROG — the whole lifecycle of one secret, by name and never by file.

  $PROG set      [<user>] <name>                    write a value you type
  $PROG get      [<user>] <name>                    decrypt one key to stdout
  $PROG list     [<user>]                           every name a user holds
  $PROG generate [--regenerate] [--yes] [<user>] [<name>]
                                                    mint values from generators
  $PROG check    [<user>]                           report drift, change nothing
  $PROG fix      [--yes]                            converge policy and ciphertext
  $PROG keygen   [--for-someone-else] [<user>]      an age identity for a person
  $PROG adduser  <name> <age-recipient> [...]       declare a person who holds none

<user> defaults to \$USER when flake.safix.users declares them, and otherwise to
the sole declared holder when there is exactly one.

The file, the key inside it and the recipients all come from
flake.safix.lib.placements. A name no declaration covers is refused rather than
given a destination.

\`$PROG <subcommand> -h\` explains one of them.

── narrowing an audience is not revocation ──
Removing someone from an audience stops future encryptions reaching them. It
takes nothing back: they have already read every value in every file they could
open, and only minting a new value revokes it. \`$PROG fix\` re-wraps each
governed file's data key to the audience now declared, which aligns ciphertext
with policy and is explicitly not revocation.

── verbs that do not exist here, and why ──
  upload   a tool that pushes generated values to a machine over ssh exists
           because the machine does not evaluate the flake holding them. A
           profile served from this repository does: activation is what delivers
           a value, through sops-nix reading the committed file. There is
           nothing for an upload to do that a rebuild does not already do.

  export   writing every value out as a plaintext tree serves migrating between
  import   backends. Both directions exist here as \`get\` and \`set\`, one name at
           a time and never as a tree — there is one backend and the migration
           those two serve is the one this does not have. A plaintext tree is
           also a thing that outlives the migration that made it, on a disk,
           which is the shape this command exists to avoid.
EOF
  exit "${1:-1}"
}

usage_set() {
  cat >&2 <<EOF
$PROG set [<user>] <name>

Prompt for a value twice without echoing it, write it into the file the
declarations place <name> in, then stage and commit that file alone.

Values are single-line and stored exactly as typed, with no trailing newline. A
multi-line value is \`sops <file>\`, or a generator — \`generate\` stores what a
script produces, newlines and all.

This is the hand-typed case, and it stays separate from \`generate\` on purpose: a
credential some server already knows cannot be minted, only transcribed.
EOF
  exit "${1:-1}"
}

usage_get() {
  cat >&2 <<EOF
$PROG get [<user>] <name>

Decrypt that one key to stdout. The output is plaintext by design and is meant
for piping. It needs an identity that opens the file, which is the owner's or a
recovery identity theirs names.
EOF
  exit "${1:-1}"
}

usage_list() {
  cat >&2 <<EOF
$PROG list [<user>]

Every name <user> holds, where it came from, whether it has a generator, the key
it is read under, and the file serving it.

The GENERATOR column shows a generator's own description when it has one, \`yes\`
when it has a generator and no description, and \`-\` when the value can only be
typed or transcribed.
EOF
  exit "${1:-1}"
}

usage_generate() {
  cat >&2 <<EOF
$PROG generate [--regenerate] [--yes] [<user>] [<name>]

Run <user>'s generators, in the dependency order the declarations compute, for
every declared secret with no value yet. --regenerate re-runs over values that
already exist, which is the rotation affordance.

With no <name>, every generator that has something to mint runs. Naming a secret
runs the one generator that writes it; naming either half of a multi-output
generator runs the generator that mints both, and both land in one commit.

A single argument that names a declared user selects that user and runs all of
their generators; anything else is read as a secret's name.

── --regenerate cascades ──
Rotating a named generator also re-runs every generator that reads what it
writes, transitively, in the same dependency order. Otherwise a rotation would
leave values derived from the value it replaced, and nothing afterwards can tell
a hash of a retired password from a hash of the current one — the tree records
no run that a value came from.

The set is listed before anything runs and confirmed, because each re-run
commits as it goes and declining afterwards takes nothing back out of history.
--yes answers that confirmation in advance. A generator nothing reads is not a
cascade and asks nothing.

── what a generator script sees ──
Each prompt and each dependency is \`\$in_<name>\`, holding the path of a
read-only file descriptor carrying that value. A hyphen in the name becomes an
underscore. Nothing reaches argv, the environment or a file, and a descriptor is
read once — read it into a variable if the script needs it twice.

That describes how the value arrives, not a sandbox it stays inside. The script
runs with the caller's filesystem and network: one that redirects \`\$in_<name>\`
into a file, or echoes it to standard error, has put plaintext somewhere this
command does not know about and cannot shred. What the script does with a value
is the script author's to get right.

\`runtimeInputs\` is prepended to PATH. Name every tool the script runs, or it
works for whoever wrote it and fails for everyone else.

One output: the script's standard output is the value, and one trailing newline
comes off a single-line one. Several outputs: the script prints a JSON object
keyed by output name, and nothing is stripped from a value read out of it.

Standard error reaches you, so diagnostics go there and never into the value.
EOF
  exit "${1:-1}"
}

usage_check() {
  cat >&2 <<EOF
$PROG check [<user>]

Report drift and change nothing. Exits non-zero when there is any, and each
finding prints the command that resolves it. Four classes:

  - the committed .sops.yaml against the policy the declarations imply
  - each governed file's recipients against the audience declared for it
  - declared names with no value, saying which have a generator
  - values in a governed file that no declaration claims

\`fix\` handles the first two. The last two are decisions rather than
convergences — a value is minted or typed, an unclaimed one is declared or
deleted — so nothing here does them for you.

It needs no identity for any file it examines: every question above is answered
from the document's structure, and nothing on this path decrypts.
EOF
  exit "${1:-1}"
}

usage_fix() {
  cat >&2 <<EOF
$PROG fix [--yes]

Regenerate .sops.yaml from the declarations, then re-wrap each governed file's
data key to the audience that policy declares. --yes answers sops' confirmation.

The order is not interchangeable: re-wrapping first re-wraps to a policy that is
about to change.

The governed set is the union of the files the declarations imply and the ones
named in flake.safix.extraGovernedFiles. A file left out of it is a file a change
of audience reaches for every other file and not for that one.

It does not commit: re-wrapping every governed file is a diff worth reading
first. It does not revoke either. A person removed from an audience has already
read every value in the file, and re-wrapping the data key does not unread it;
revoking means a new value, which is \`generate --regenerate\` or \`sops <file>\`.
EOF
  exit "${1:-1}"
}

usage_keygen() {
  cat >&2 <<EOF
$PROG keygen [--for-someone-else] [<user>]

Mint an age identity and append it to \${XDG_CONFIG_HOME:-\$HOME/.config}/sops/age/keys.txt,
then print the public half and what to do with it. The private half is never
printed.

It appends and never truncates: sops tries every identity in that file, so a
second identity beside a first is a working state, and overwriting is how
someone loses the key to everything they hold.

Run it on your own machine, as yourself. Minting another person's identity here
means you hold their private key, which is the opposite of the independent
custody this package rests on, so it takes an explicit --for-someone-else.

An existing ssh key works instead: \`ssh-to-age < ~/.ssh/id_ed25519.pub\` prints
the age recipient for it, and sops.age.sshKeyPaths names the private half.
EOF
  exit "${1:-1}"
}

usage_adduser() {
  cat >&2 <<EOF
$PROG adduser <name> <age-recipient> [--host <hostname>]... [--yes]

Declare a person who holds nothing yet: write $(scaffold_path '<name>'),
regenerate .sops.yaml from the policy that declaration implies, commit the two,
and then hand the name and the recipient to flake.safix.onboardingHook.

  --host H    passed through to the hook, repeatable. Refused when no hook is
              configured, because attaching an account on a host is a property
              of a consumer's module tree and safix has none.
  --yes       skip the confirmation.

<age-recipient> is theirs, minted by them, and only its SHAPE is checked here —
whether anyone holds the private half is not knowable from this machine. A
recipient that needs a physical interaction to decrypt is refused for this field:
activation decrypts non-interactively and a card needs a touch, so it belongs in
that person's recoveryRecipients instead, where it is additive.

── what this does not do ──
Mint anything. No age key (that is \`keygen\`, run by them on their machine), no
password material, and no secret value.

Give them anything to hold. The scaffold declares no secret, so no audience is
computed for them and the regenerated .sops.yaml carries their key as an anchor
with no creation rule yet. Their first secret is a name under \`private\` or
\`carries\`, then \`$PROG fix\` to write the rule, then \`$PROG set\`.

Anything about hosts, accounts, identifiers or groups. Those are one consumer's
module tree, reached through the hook and nowhere else. A hook receives:

    \$1  the new person's name
    \$2  their recipient
    \$3… every --host given, in order

and runs after the scaffold and the policy are committed, so whatever it writes
is its own to stage. Its absence is a supported configuration: onboarding
without a hook succeeds, having done less.
EOF
  exit "${1:-1}"
}

# `-h` on a subcommand shows that subcommand's page. Checked before the argument
# arity, so asking for help is never an arity error.
help_for() { # <sub> <args...>
  local sub="$1" arg
  shift
  for arg in "$@"; do
    case "$arg" in
      -h | --help)
        case "$sub" in
          set) usage_set 0 ;;
          get) usage_get 0 ;;
          list) usage_list 0 ;;
          generate) usage_generate 0 ;;
          check) usage_check 0 ;;
          fix) usage_fix 0 ;;
          keygen) usage_keygen 0 ;;
          adduser) usage_adduser 0 ;;
          *) usage 0 ;;
        esac
        ;;
    esac
  done
}

main() {
  local sub user name
  sub="${1:-}"
  if [ $# -gt 0 ]; then shift; fi
  help_for "$sub" "$@"
  case "$sub" in
    -h | --help | help) usage 0 ;;
    set | get)
      case $# in
        1)
          load_placements
          user="$(default_user)"
          name="$1"
          ;;
        2)
          user="$1"
          name="$2"
          ;;
        *) die "usage: $PROG $sub [<user>] <name>" ;;
      esac
      "cmd_$sub" "$user" "$name"
      ;;
    list)
      case $# in
        0)
          load_placements
          user="$(default_user)"
          ;;
        1) user="$1" ;;
        *) die "usage: $PROG list [<user>]" ;;
      esac
      cmd_list "$user"
      ;;
    keygen)
      local others=""
      if [ "${1:-}" = --for-someone-else ]; then
        others=--for-someone-else
        shift
      fi
      case $# in
        0)
          load_placements
          user="$(default_user)"
          ;;
        1) user="$1" ;;
        *) die "usage: $PROG keygen [--for-someone-else] [<user>]" ;;
      esac
      cmd_keygen "$user" "$others"
      ;;
    check)
      case $# in
        0) cmd_check "" ;;
        1) cmd_check "$1" ;;
        *) die "usage: $PROG check [<user>]" ;;
      esac
      ;;
    fix)
      case $# in
        0) cmd_fix "" ;;
        1)
          [ "$1" = --yes ] || die "usage: $PROG fix [--yes]"
          cmd_fix --yes
          ;;
        *) die "usage: $PROG fix [--yes]" ;;
      esac
      ;;
    generate)
      # Both flags are read before the positional arguments and in either order,
      # because `--yes` answers a question `--regenerate` is what raises.
      local assume_yes=""
      while :; do
        case "${1:-}" in
          --regenerate)
            GEN_REGENERATE=1
            shift
            ;;
          --yes)
            assume_yes=--yes
            shift
            ;;
          *) break ;;
        esac
      done
      case $# in
        0)
          load_placements
          user="$(default_user)"
          name=""
          ;;
        1)
          # The one argument is a user when it names one, and a secret otherwise.
          # A secret whose name is also a person's is reachable by naming both:
          # this is the only subcommand whose single optional argument could be
          # either, because it is the only one that means something with no
          # secret named at all.
          load_placements
          if declared_users | grep -qxF -- "$1"; then
            user="$1"
            name=""
          else
            user="$(default_user)"
            name="$1"
          fi
          ;;
        2)
          user="$1"
          name="$2"
          ;;
        *) die "usage: $PROG generate [--regenerate] [--yes] [<user>] [<name>]" ;;
      esac
      cmd_generate "$user" "$name" "$assume_yes"
      ;;
    adduser)
      # Flags are read in any order and around the two positionals, because
      # `--host` is repeatable and a caller adding a second one should not have
      # to know where the name and the recipient sit.
      local positional=()
      while [ $# -gt 0 ]; do
        case "$1" in
          --host)
            [ $# -ge 2 ] || die "--host takes a hostname"
            ADDUSER_HOSTS+=("$2")
            shift 2
            ;;
          --yes)
            ADDUSER_YES=--yes
            shift
            ;;
          --)
            shift
            while [ $# -gt 0 ]; do
              positional+=("$1")
              shift
            done
            ;;
          -*) die "unknown option '$1' (expected --host or --yes)" ;;
          *)
            positional+=("$1")
            shift
            ;;
        esac
      done
      [ ${#positional[@]} -eq 2 ] \
        || die "usage: $PROG adduser <name> <age-recipient> [--host <hostname>]... [--yes]"
      cmd_adduser "${positional[0]}" "${positional[1]}"
      ;;
    "") usage 1 ;;
    *) die "unknown subcommand '$sub' (expected set, get, list, generate, check, fix, keygen or adduser)" ;;
  esac
}

main "$@"
