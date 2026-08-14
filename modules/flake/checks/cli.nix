# Holds the `safix` command to what it promises an operator who runs it instead
# of reaching for `sops <file>` themselves.
#
# The command's whole value is that the operator stops choosing a file, so the
# claims worth checking are the ones a hand-run `sops` would have made the
# operator responsible for: that a value lands in the file the declarations place
# it in and under the key it is read by, that a file created for it acquires its
# recipients from .sops.yaml's creation rules rather than from anywhere else,
# that setting one key disturbs no other, that a run which changes nothing
# commits nothing, and that a run which aborts leaves neither a partial file nor
# a plaintext value behind.
#
# The checks drive the real sops, the real age and the real git against a
# throwaway repository, keys minted in the sandbox and a fixture .sops.yaml
# shaped like the generated one. Only `nix` is stubbed, because a flake
# evaluation is what a sandbox cannot do; that stub also asserts the attribute
# name the command reads, so renaming flake.safix.lib.placements fails here
# rather than at the operator's terminal. Standing a stub in for sops is what
# lets a check stay green over a command calling something the tree no longer
# contains, and is not done.
#
# ── what these do not cover ──
# The recipient policy itself. ./policy.nix relates the rendered policy to the
# declarations; this command writes through sops and so can neither widen an
# audience nor repair one, and a check here asserting anything about a rendered
# policy would be asserting that other check's claim against a fixture.
# `recipient-drift` below is not an exception: its subject is what the command
# does when handed a drifted file, judged against a fixture.
#
# ── severity: proven by perturbation, one drill per claim ──
# Passing `sops set` without `--idempotent` fails `set-existing` on the
# byte-identity of a re-run. The drill only bites because the check waits out a
# second first: sops stamps `lastmodified` at one-second resolution and reuses an
# unchanged value's IV, so a re-run inside the same second is byte-identical
# either way and the assertion would have held over a command that had dropped
# the flag.
# Dropping the `-- <path>` from the commit fails `staged-bystander`, which finds
# an unrelated staged file in the commit. Dropping the same scoping from the
# emptiness test ahead of it fails the run outright: with another path staged, an
# unscoped `git diff --cached` reports work to do on a run that wrote nothing,
# and the partial commit that follows has nothing to commit.
# Addressing the value by the secret's name rather than by the entry's `sopsKey`
# fails `set-existing`, which reads the aliased entry back under the key the
# declarations name and finds nothing there.
# Writing the created document straight to its final path instead of to the
# scratch file fails `refusals`: sops emits nothing and exits non-zero when no
# creation rule matches, so the redirection leaves an empty unruled file beside
# the others — which is the state the anchoring note in the generated policy
# exists to prevent.
# Removing the EXIT trap fails `abort` on the scratch file and the audience
# directory an interrupted run leaves behind. Removing `trap 'exit 130' INT`
# alone does not move any claim here, and is kept regardless: bash happens to run
# the EXIT trap when a script dies of an unhandled SIGINT, so what the explicit
# routing buys is that the shredder runs by construction rather than by that
# behaviour.
# Dropping the dirty-target guard fails `refusals`, which hand-edits the target
# and then finds the edit accepted; dropping the mid-rebase and mid-merge guard
# fails the same check, which is the state where a partial commit means something
# other than what the message says.
# Dropping the `no matching creation rules found` branch fails `refusals`, whose
# claim is not that the run fails but that its failure names `safix fix`: the
# operator cannot act on sops' own wording, because the rule they are missing is
# generated.
# Pointing `--filename-override` at the writer's own file rather than at the
# target's path fails `set-new`, whose shared file then names ana and not bo.
# The fixture mints two distinct recipients for that drill to have anything to
# catch: with one key under both anchors, a file encrypted to the writer alone
# satisfies every other assertion and hands the other party a file they cannot
# open.
# Dropping the recipient assertion fails `recipient-drift`, which then finds the
# value minted into a file an identity outside the declared audience can open,
# and committed. Moving that same assertion to after the rename fails it as well,
# on HEAD and on the ciphertext digest rather than on the message: a refusal that
# reads correctly after the write has landed is the disclosure it exists to
# prevent, because the value is already in git history. Reading the file in place
# instead of the prepared document passes the drifted case and misses the
# new-file one, where the stale recipients come from a creation rule rather than
# from metadata, so the check drifts a committed file and creates one from a
# narrowed rule.
# Dropping the unknown-name refusal for a bare `die` fails `refusals` on all
# three declaration surfaces. The claim is the naming, not the exit status — a
# name absent from the declarations is most often a name declared in one of
# exactly three places and not yet in any of them. The same check greps the
# refusal for `flake.users` and `flake.homeSecrets`, so a message that still
# names an option path outside safix's namespace fails rather than merely reads
# oddly.
# Driving `fix` from `required` rather than from `managed` fails
# `governed-extras`, whose consumer-named file is then left encrypted to an
# identity the covering rule no longer grants while every other file moved.
# Reporting an extra file's keys as unclaimed fails the same check on its first
# assertion, which is the other direction of the same split: that finding is one
# no declaration could ever resolve.
# Regenerating `.sops.yaml` before staging the scaffold rather than after fails
# `adduser`, whose policy then carries every declared person except the one just
# declared. That order was the defect the check was written against: a flake
# evaluation reads the files git tracks, so an untracked scaffold is invisible to
# it and the result looks freshly generated while describing the declarations as
# they stood a moment earlier. The stub computes the policy from `git ls-files`
# for this drill to have anything to catch — a stub emitting a fixed document
# passes either order.
# Dropping the `-- <path>` from the commit fails `adduser` on the bystander
# staged before the run, the same drill `staged-bystander` runs against `set`.
# Running the onboarding hook before the commit rather than after fails
# `adduser-hook`, which finds the hook's output inside safix's own commit — a
# commit whose message names only what safix did. Accepting `--host` with no hook
# configured fails the same check, which then finds a hostname silently
# discarded.
{ ... }:
{
  perSystem =
    { pkgs, ... }:
    let
      readers = import ../safix/readers.nix { inherit pkgs; };

      # Real sops, age and git; `column` for the `list` table. bash is explicit
      # because the driver and the command are both driven as `bash <script>`
      # rather than executed through their shebangs.
      harness = [
        pkgs.age
        pkgs.bash
        pkgs.coreutils
        pkgs.findutils
        pkgs.git
        pkgs.gnugrep
        pkgs.gnused
        pkgs.jq
        # For `nix-instantiate --parse` alone, which is what holds the nix
        # `adduser` generates to being nix. Parsing needs no store and no
        # daemon, so it is available where an evaluation is not; the `nix` the
        # command itself reaches for stays the stub, because it is named through
        # SAFIX_NIX rather than found on PATH.
        pkgs.nix
        pkgs.sops
        readers.sops-recipients-of
        readers.sops-keys-of
        pkgs.util-linux
      ];

      selftest =
        name: mode:
        pkgs.runCommand name { nativeBuildInputs = harness; } ''
          export HOME="$PWD"
          SAFIX_SH=${../safix/safix.sh} bash ${../safix/safix-selftest.sh} ${mode}
          touch "$out"
        '';
    in
    {
      # A file the declarations place a secret in but that nobody has run sops
      # on yet is created through sops, so it acquires the creation rule's
      # recipients; the value round-trips under the resolved key; and the file
      # is committed on its own under a message naming the secret and never the
      # value.
      checks.safix-set-new = selftest "safix-set-new" "set-new";

      # One key moves and the rest of the file comes through byte-identical,
      # compared by digest rather than by value. A re-run of the same value is
      # byte-identical and commits nothing; a different value rotates that key
      # alone; an entry whose `sopsKey` differs from its name lands under the key.
      checks.safix-set-existing = selftest "safix-set-existing" "set-existing";

      # Every refusal, and each one for its own reason: an undeclared name, a
      # path with no creation rule, an undeclared user, a placement outside
      # `*.yaml`, an empty value, a dirty target file, a repository mid-rebase
      # or mid-merge, and an unrecognised subcommand. None may resolve itself by
      # choosing a destination, and none may name an option path outside safix's
      # namespace.
      checks.safix-refusals = selftest "safix-refusals" "refusals";

      # A file whose recipients have drifted from the audience declared for it
      # is refused before the rename, in both directions — an identity the
      # audience does not name, and an audience member the file cannot be opened
      # by — and the refusal leaves HEAD, the ciphertext and the tree exactly as
      # it found them. Once `sops updatekeys` repairs the drift the same set goes
      # through and commits.
      checks.safix-recipient-drift = selftest "safix-recipient-drift" "recipient-drift";

      # Another path's staged change survives the run staged and uncommitted, and
      # does not make an idempotent re-run commit.
      checks.safix-staged-bystander = selftest "safix-staged-bystander" "staged-bystander";

      # A SIGINT at the prompt and a backend that fails after the value was read.
      # Neither may leave a partial file, a scratch file, a created directory, or
      # the value anywhere on disk including $TMPDIR.
      checks.safix-abort = selftest "safix-abort" "abort";

      # `get` round-trips a value by digest, for a secret of the user's own and
      # for one shared from another owner, and resolves the same file for both
      # parties. A value set and read back is byte-identical, trailing newline
      # included, and nothing but the value reaches standard output. `list`
      # reports each name against the file serving it and the key it is read
      # under, and renders no value.
      checks.safix-get-list = selftest "safix-get-list" "get-list";

      # A generator with no inputs mints and commits; one with a prompt reads it
      # unechoed and derives from it; one with a dependency runs after the
      # generator that writes what it reads and sees that plaintext down a
      # descriptor; one with several outputs writes both, in different files, in
      # one commit. A second bulk run mints nothing, and --regenerate rotates its
      # target while a neighbouring key's ciphertext comes through byte-identical.
      checks.safix-generate = selftest "safix-generate" "generate";

      # Every way a run is refused: a name with no generator, empty output, a
      # script that exits non-zero, a candidate the validation rejects, and a
      # multi-output script printing the wrong keys. None leaves a value, a
      # commit, or a scratch file, and a partial keypair is never written.
      checks.safix-generate-refusals = selftest "safix-generate-refusals" "generate-refusals";

      # What one generator's process may see of another's. A script that reads
      # standard input to end of input does not eat the answer to a later
      # generator's prompt, and a generator running last sees exactly the
      # descriptors one running first sees — every input descriptor carries a
      # decrypted value, so one surviving the generator it was opened for is
      # plaintext in a process that never declared it.
      checks.safix-generate-isolation = selftest "safix-generate-isolation" "generate-isolation";

      # `--regenerate` of a named generator carries everything downstream of it.
      # The set is listed in dependency order and confirmed first; declining
      # writes nothing; accepting leaves every downstream value a function of
      # the value that was just minted rather than of the one it replaced; a
      # generator that reads none of it is not re-run; and --yes answers the
      # confirmation in advance.
      checks.safix-generate-cascade = selftest "safix-generate-cascade" "generate-cascade";

      # The union `fix` acts on, from both sides. A consumer-named file in step
      # with the rule that covers it is not a finding of any kind; the same file
      # drifted from that rule is reported and re-wrapped; and a named path no
      # rule's directory covers is reported as such, because naming a file
      # creates no rule for it.
      checks.safix-governed-extras = selftest "safix-governed-extras" "governed-extras";

      # Declaring a person writes one custody record and commits exactly that
      # and the regenerated policy — not a bystander staged alongside it. The
      # policy carries the person just declared, which is only true if the
      # scaffold was staged before it was regenerated: a flake evaluation reads
      # what git tracks, so regenerating first writes the policy of the
      # declarations as they stood without them. They hold nothing, so their key
      # is an anchor with no rule. Nothing is minted, the output says as much,
      # and redeclaring is refused.
      checks.safix-adduser = selftest "safix-adduser" "adduser";

      # Every refusal, each for its own reason: a name outside the alphabet, a
      # name carrying a path separator, a malformed recipient, an over-long one,
      # and an existing person. Each leaves no scaffold, no commit and no dirt. A
      # recipient that needs a physical interaction is refused separately from
      # the malformed ones, because it is well-formed and still cannot be an
      # activation identity, and its refusal has to name recoveryRecipients —
      # where a card does belong — or the operator is told only that their key is
      # unwelcome.
      checks.safix-adduser-refusals = selftest "safix-adduser-refusals" "adduser-refusals";

      # Host attachment reaches a consumer through the hook or not at all.
      # `--host` with no hook configured is refused naming the hook and saying
      # that onboarding without one succeeds; a configured hook receives the
      # name, the recipient and every host, and runs after safix's commit has
      # landed, so what it writes is left uncommitted and safix's message names
      # only what safix did.
      checks.safix-adduser-hook = selftest "safix-adduser-hook" "adduser-hook";

      # A shared entry is one value: both carriers' placements name one file and
      # one key, one of them mints, the other reads back what was minted, and
      # exactly one file in the repository holds the key.
      checks.safix-shared-placement = selftest "safix-shared-placement" "shared-placement";

      # A carrier dropped from a shared entry is a revocation, and `check` says
      # so: it names the file still holding the value, names the person who can
      # open it, offers a new value as the remedy, and states that `fix` will not
      # revoke. The finding arrives once, not also as an unclaimed value.
      checks.safix-shared-shrink = selftest "safix-shared-shrink" "shared-shrink";

      # Flipping an entry to shared over values already present is reported as a
      # migration rather than a disclosure, because every reader of the copy left
      # behind is still in the audience — and the choice of which per-carrier
      # value survives is left to the operator.
      checks.safix-shared-flip = selftest "safix-shared-flip" "shared-flip";
    };
}
