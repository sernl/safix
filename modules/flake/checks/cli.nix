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
# Each drill below names a perturbation and the check whose claim it breaks.
# They were established against the runtime that was under test when each mode
# was written, and they survive the port because what they perturb is the claim
# rather than the language: the assertion that catches a dropped `--idempotent`
# is the same assertion whichever runtime dropped it. Where a drill named a
# construct only the shell runtime had, it is recast below in terms of the claim
# it broke.
#
# The drills observed red during this change are the four single-runtime checks'
# own, recorded beside the tests that carry them: the interrupt inside sops in
# `abort_residue.rs`, the plaintext write to a regular file in
# `syscall_proof.rs`, and the five channel mutations in `channel_drills.rs`,
# which is the severity evidence for every check on this page — an assertion
# nobody has watched fail is not evidence, and those five are what show that
# each channel these checks read can fail.
#
# Handing the refusing run of `identity-recipiency` an identity the file is
# encrypted to fails it on the assertion that exists to catch it, which is what
# separates the claim from one that would hold over any unusable key file.
# Observed during this change.
#
# Judging the run order after walking it rather than before fails
# `generate-cycle`, whose fixture puts a generator ahead of the cycle in the
# order: the walk mints and commits that one, then reports the first missing
# input as an empty output. Observed during this change, which is why the check's
# assertions are about when the refusal arrives rather than that it does.
#
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
# candidate file fails `refusals`: sops emits nothing and exits non-zero when no
# creation rule matches, so the write leaves an empty unruled file beside the
# others — which is the state the anchoring note in the generated policy exists
# to prevent.
# Leaking the sweep that runs on the way out fails `abort` on the scratch file
# and the audience directory an interrupted run leaves behind. In the shell
# runtime that sweep was an EXIT trap and the drill removed it; here it is the
# scratch registry's guard, and the claim is the same one. What the explicit
# signal handling buys is unchanged too: the sweep runs by construction rather
# than by a property of the interpreter.
# Dropping the dirty-target guard fails `refusals`, which hand-edits the target
# and then finds the edit accepted; dropping the mid-rebase and mid-merge guard
# fails the same check, which is the state where a partial commit means something
# other than what the message says.
# Dropping the `no matching creation rules found` branch fails `refusals`, whose
# claim is not that the run fails but that its failure names `safix fix`: the
# operator cannot act on sops' own wording, because the rule they are missing is
# generated.
# Pointing `--filename-override` at the writer's own file rather than at the
# target's path fails `set-new`, whose shared file then names alice and not bob.
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
#
# ── the enrollment drills, all observed red during this change ──
# Sampling the pseudo-terminal's echo state only when the master has nothing to
# read fails `enroll-one-attempt`: the child restores the echo the instant it has
# the line and turns it off again for a second prompt, both inside one polling
# interval, so a watcher looking for the transition sees none and the run stalls
# where it should have refused. The check reads the attempt count for that reason
# rather than only the refusal.
# Inferring the child's exit from the master reporting `EIO` rather than asking it
# fails every enrollment check: `Command::spawn` borrows the command rather than
# consuming it, so the parent holds a description of the slave for as long as the
# command is alive and the master never reports it.
# Dispatching the card stub's plugin role on `--generate` appearing anywhere
# rather than on the word the vector opens with fails `enroll` and
# `enroll-one-attempt`: `piv access change-management-key --protect --generate`
# carries it too, and the stub then prompts for a PIN in the middle of
# provisioning.
# Staging every governed file rather than the ones that exist fails `enroll` on
# the commit: a governed file is a path a declaration implies rather than a file
# anybody has written, and `git add` on an absent path refuses the whole ceremony.
# Taking the first identity file the spy recorded rather than the first that is
# not the ambient one fails `enroll-proof-isolation`, which is the assertion that
# exists to catch a proof reading the software key: every invocation before the
# proof was handed that key, and the proof's own is the one after them.
# Writing a newline after the last value fed to a store rather than only between
# values fails `enroll-custody` on the round trip, which finds a byte in the
# stored credential that nobody put there.
# Passing a generated credential to ykman as an option rather than answering its
# prompt fails `enroll` on the unconditional reading: every invocation's argument
# vector and environment are recorded, and neither may contain the PIN or the PUK
# on any path. Observed red by adding a `--pin` carrying the PIN back to the
# management-key drive. The earlier shape of this check read only the store's own
# invocations and would have passed over exactly the channel that mattered.
# Sampling the echo state to find where one prompt ends and the next begins fails
# `enroll` and `enroll-one-attempt` intermittently rather than reliably, which is
# why the wrapper does not do it: a hidden read restores the terminal in
# microseconds and both tools flush pending input when they set it, so a wrapper
# that sequenced answers would put the wrong value in the wrong prompt under load.
# Answering an echo-off state immediately rather than waiting for the terminal to
# fall quiet fails `enroll` by queueing a second copy of the value behind the
# first, because a prompt's text is written after the echo goes off.
# Answering whatever a drive asks rather than a bounded number of prompts fails
# `enroll-one-attempt-ykman`, whose stub asks once more than the drive needs.
# ── the sync drills, all observed red during this change ──
# Making the two-way tiebreak answer "neither side matches the agreement" fails
# `safix-sync-two-way` on the one-side-changed cases, and making it pick safix's
# value instead of reporting a conflict fails the same check on the both-changed
# case. The pair is what holds the tiebreak to the recorded state rather than to a
# constant: either alone would pass over a runtime that always did the other.
# Dropping the newline refusal fails `safix-sync-refusals`, which then finds the
# entry written with the bytes before the newline — a mirror that lies about what
# it holds, and a mapping that would rewrite the whole database on every run
# afterwards because the comparison is byte-exact.
# Writing the recorded agreement into the repository as well as into the database
# fails `safix-sync`, `safix-sync-two-way` and `safix-sync-converges`: the first
# two on the no-oracle search, which looks for the value, for its digest taken by
# `sha256sum`, and for the record's own format tag, and the third on the tree
# being dirty. The first attempt at this drill wrote into a directory that does
# not exist and swallowed the failure, so nothing turned red and the drill proved
# nothing — the second wrote where the run's working directory is, which is what
# made the claim severe rather than vacuous.
# Reading one entry back after writing it fails `safix-sync-burst`, which finds a
# read between two writes. Placing that read *before* the write does not fail it
# and must not: the entry is absent then, so the read is answered from the listing
# without spawning anything, and there is no save for it to sit inside.
# Dropping the store override from a run's environment, or declaring a database
# outside the fixture's own scratch directory, fails every sync check before a
# process is spawned, on the harness's own guard. That drill was run deliberately
# and its failure mode is not a red check: the machines this is developed on have
# the operator's own database, which is the fleet's root of trust, so a run that
# reached it would edit entries in it. The guard is structural for that reason
# rather than a convention to be remembered — see `refuse_a_real_database`.
# `store_cli.rs` needed no drill for its own subject and produced a finding
# instead: `ls -R -f` over a database holding nothing prints `[empty]` rather than
# nothing, which the runtime has to skip and which no model would have revealed.
#
# ── the bridge-sync drills, all observed red during this change ──
# Treating two absent sides as a conflict rather than as agreement fails
# `safix-bridge-sync-unchanged` on the report and on the exit code, which a
# run that converged nothing must leave at zero.
# Picking a side by fiat in the "no agreement yet, sides disagree" branch,
# rather than refusing, fails `safix-bridge-sync-conflict` — the run is
# accepted where it must be refused, and safix's own side is overwritten.
# Skipping the companion write in `push` fails `safix-bridge-sync-push`'s
# "no commit landed" assertion, because the companion is the only commit a
# push makes in this repository; the same drill, applied to `pull` instead,
# fails `safix-bridge-sync-pull` on which commit `HEAD` names, since without
# it the value's own write becomes the run's last commit.
# Making `remembered_agreement` always answer `None` fails
# `safix-bridge-sync-remembered`: a later divergence that should converge
# using the recorded agreement is reported as a conflict instead, because
# there is no agreement left for it to consult.
# Skipping the stale-generator check in `push` fails
# `safix-bridge-sync-stale-generator`, whose run is accepted and its value
# written into clan where the refusal exists to prevent exactly that.
# Having `Addressing::resolve` search no candidate at all — never calling
# `clan machines list` — fails `safix-bridge-sync-shared-address` on
# `ClanAddressUnresolved`, since the fixture's mapping declares no machine
# for a fixed one to have come from instead.
#
# Dropping one of the four card-surface overrides from a run's environment fails
# every enrollment check before a process is spawned, on the harness's own guard.
# That drill was run deliberately and is the one whose failure mode is not a red
# check: the machines this is developed on have the real ykman and a hardware key
# in a reader holding master identities, so a run that reached it would provision
# a live card irreversibly. The guard is structural for that reason rather than a
# convention to be remembered.
#
# ── the upload drills ──
# `safix-upload-remote-match`'s claim is the one this whole change exists for
# (task 3.6): that a matching probe opens no write-capable session, asserted
# against the transport stub's own recorded role list rather than against file
# state alone — a bug that opened a session and happened to write nothing would
# still pass a check that only looked at the filesystem. Recording `ssh` in
# `transport_invocations()` when `Host::write` runs and never on the match
# branch of `write_remote` is what a regression there turns red on.
# `safix-upload-remote-flip-drill` (task 3.7) is the same claim's other half:
# flipping one byte of the declared recipient in the fixture, with the same
# presented key `safix-upload-remote-match` used, turns the honest no-op into a
# refusal — proving the branch follows the comparison rather than a
# fixture-specific shortcut a stub could satisfy by coincidence.
# `safix-upload-directory-drift-drill` (task 2.6, first) derives an identity one
# byte off the declared recipient under this suite's own conversion and asserts
# the write still refuses; comparing with anything looser than exact string
# equality — a prefix match, a truncated comparison — passes it and fails this.
# `safix-upload-directory-null-recipient-drill` (task 2.6, second) is ordering
# rather than comparison: a null-recipient machine refuses before an identity
# with an otherwise-matching derivation is ever read, asserted by the transport
# stub recording no invocation at all — reordering `resolve_machine` after the
# identity read would leave a spurious `ssh-keygen`/`ssh-to-age` pair in that
# list even though the run still refuses.
# `safix-upload-destination`'s own depth-safety constant is drilled at the unit
# level, in `crates/safix-core/src/upload.rs`'s
# `upload::tests::a_shallow_destination_fails_the_depth_safety`: pointing
# `destination_is_safe` at a two-component path turns it red, which is what
# shows the check this integration test relies on is live rather than
# trivially satisfied by the fixed constant.
# `safix-upload-unknown-machine` (task 1.4, first) holds the undeclared-name
# refusal: no subprocess runs and no output directory is created before
# `resolve_machine` returns. Reading `--identity` before resolving the machine
# would leave a spurious `ssh-keygen` invocation in the recorded list even
# though the run still refuses.
# `safix-upload-no-recipient` (task 1.4, second) holds the null-recipient
# refusal, distinct from the undeclared-name one, and — like the directory-mode
# drill above — asserts it fires before any identity is read.
# `safix-upload-not-a-machine` (task 1.4, third) holds design decision D6: a
# declared person's name is refused with the identical undeclared-machine
# message rather than a distinct one, so a message that grew a
# person-specific branch would fail this check's exact-string assertion.
# `safix-upload-directory` (tasks 2.1-2.3) holds the write itself: a matching
# identity derives its recipient, confirms it against the declared one, and
# lands at exactly the two paths 2.3 names, at 0600 and 0644, with no other
# path under DIR — asserted by `std::fs::metadata` and a full directory
# listing rather than by the two paths' presence alone.
# `safix-upload-directory-needs-identity` (task 2.4) holds `--directory`
# without `--identity` refusing before the filesystem is touched.
# `safix-upload-directory-mismatch` (task 2.2) holds the ordinary mismatch
# refusal, naming both recipients, before DIR is created.
# `safix-upload-remote-match-force` (task 3.4) holds `--force` staying inert on
# the match branch: given alongside a matching probe, the run still reports the
# honest no-op and still opens no write-capable session, distinguishing an
# override that is inert on a match from one that would have short-circuited
# the comparison.
# `safix-upload-remote-write` (task 3.3, second branch) holds the write path
# when the probe presents no key: the recorded invocation order
# (`ssh-keyscan`, `ssh-keygen`, `ssh-to-age`, `ssh`) and the `ssh` argv itself
# — the destination, `BatchMode=yes`, and the wipe-then-extract sequence — are
# asserted directly rather than inferred from success.
# `safix-upload-remote-needs-identity` (task 3.3, second branch without
# `--identity`) holds the same branch's refusal, with the probe
# (`ssh-keyscan`) the only invocation recorded.
# `safix-upload-remote-mismatch` (task 3.3, third branch without `--force`)
# holds the default refusal on an unrelated presented key, naming both
# recipients, with no write-capable session opened.
# `safix-upload-remote-force` (task 3.3, third branch with `--force` and
# `--identity`) holds the override reaching the transport.
# `safix-upload-tarball-modes` (tasks 4.1-4.2, 4.7) holds the tarball's own
# contents: both staged files at file mode 0400 and root ownership, read back
# from the real archive with `tar -tvzf` rather than trusted from the writer
# that built it.
# `safix-upload-staging-cleanup` (task 4.5) holds the `Staging` root's
# lifecycle on both the success path and a simulated transport failure
# (`SAFIX_TRANSPORT_STUB_SSH_REFUSES=1`): created before the tarball is
# written, gone after the run either way.
# `safix-upload-help-scaffold` (tasks 1.3, 6.3) holds `safix -h` listing
# `upload` in the scaffold's operator-facing order, against an insta snapshot.
# `safix-upload-help-text` (tasks 6.1, 6.3) holds `safix upload -h` stating the
# two write modes and the three named absences, against an insta snapshot.
{ ... }:
{
  perSystem =
    { config, pkgs, ... }:
    let
      integration = import ./integration.nix { inherit pkgs; };

      # One mode, one test of the compiled suite. The attribute names are the
      # ones this file has always had, so a consumer's CI keeps running the check
      # it configured; what changed is the subject, which is now the shipped
      # binary held to a literal rather than a shell script held to a fixture.
      mode =
        name: target: test:
        integration.runOne config.checks.safix-integration name target test;

      # The one mode that needs a tool the rest of the page has no use for: the
      # real store command, whose closure is a Qt application. See
      # ./integration.nix for why it is not in `backends`.
      withStore =
        name: target: test:
        integration.runOneWith [ integration.keepassxc ] config.checks.safix-integration name target test;
    in
    {
      # A file the declarations place a secret in but that nobody has run sops
      # on yet is created through sops, so it acquires the creation rule's
      # recipients; the value round-trips under the resolved key; and the file
      # is committed on its own under a message naming the secret and never the
      # value.
      checks.safix-set-new =
        mode "safix-set-new" "write_path"
          "set_new_creates_the_file_through_the_creation_rules";

      # One key moves and the rest of the file comes through byte-identical,
      # compared by digest rather than by value. A re-run of the same value is
      # byte-identical and commits nothing; a different value rotates that key
      # alone; an entry whose `sopsKey` differs from its name lands under the key.
      checks.safix-set-existing =
        mode "safix-set-existing" "write_path"
          "set_existing_moves_one_key_and_leaves_the_others_byte_identical";

      # Every refusal, and each one for its own reason: an undeclared name, a
      # path with no creation rule, an undeclared user, a placement outside
      # `*.yaml`, an empty value, a dirty target file, a repository mid-rebase
      # or mid-merge, and an unrecognised subcommand. None may resolve itself by
      # choosing a destination, and none may name an option path outside safix's
      # namespace.
      checks.safix-refusals =
        mode "safix-refusals" "write_path"
          "refusals_each_have_their_own_code_and_leave_the_tree_alone";

      # A file whose recipients have drifted from the audience declared for it
      # is refused before the rename, in both directions — an identity the
      # audience does not name, and an audience member the file cannot be opened
      # by — and the refusal leaves HEAD, the ciphertext and the tree exactly as
      # it found them. Once `sops updatekeys` repairs the drift the same set goes
      # through and commits.
      checks.safix-recipient-drift =
        mode "safix-recipient-drift" "write_path"
          "recipient_drift_is_refused_before_anything_is_written";

      # Another path's staged change survives the run staged and uncommitted, and
      # does not make an idempotent re-run commit.
      checks.safix-staged-bystander =
        mode "safix-staged-bystander" "write_path"
          "a_staged_bystander_survives_the_run_and_does_not_make_it_commit";

      # A SIGINT at the prompt and a backend that fails after the value was read.
      # Neither may leave a partial file, a scratch file, a created directory, or
      # the value anywhere on disk including $TMPDIR.
      checks.safix-abort =
        mode "safix-abort" "write_path"
          "an_aborted_run_leaves_no_file_no_scratch_and_no_value";

      # Where `set` reads its value from. A pipe is stored as its own bytes,
      # trailing newline included, with nothing prompted and nothing confirmed; an
      # empty pipe takes the empty-value refusal; and a real pseudoterminal on
      # standard input still gets the hidden double prompt, with the second read
      # shown to happen by a run given one line and refused for want of a
      # confirmation. Three tests rather than one, because the fork is what is
      # under test and each side has to be reachable on its own.
      checks.safix-value-source = mode "safix-value-source" "value_source" "";

      # `get` round-trips a value by digest, for a secret of the user's own and
      # for one shared from another owner, and resolves the same file for both
      # parties. A value set and read back is byte-identical, trailing newline
      # included, and nothing but the value reaches standard output. `list`
      # reports each name against the file serving it and the key it is read
      # under, and renders no value.
      checks.safix-get-list =
        mode "safix-get-list" "read_path"
          "get_round_trips_a_value_and_list_reports_where_it_lives";

      # The sentence the consumption module's identity preflight makes about what
      # it did not check: an identity present, readable and not a recipient of
      # these files does not open them. `safix-consumption-ordering` holds the
      # ordering that guard rests on against a real profile evaluation, and
      # everything else on that path was an evaluation too, so this is the half of
      # the guard's own message that needed ciphertext to be held at all. The
      # stranger's identity is shown to open a document it is a recipient of
      # first, or the claim would hold over a key file that was simply broken.
      checks.safix-identity-recipiency =
        mode "safix-identity-recipiency" "read_path"
          "an_identity_present_and_readable_and_not_a_recipient_does_not_decrypt";

      # A generator with no inputs mints and commits; one with a prompt reads it
      # from $prompts twice, which the descriptor interface this replaced could
      # not do; one with a dependency runs after the generator that writes what
      # it reads and finds it at $in/<producer>/<name>; one with several outputs
      # writes both, in different files, in one commit. A second bulk run mints
      # nothing, and --regenerate rotates its target while a neighbouring key's
      # ciphertext comes through byte-identical.
      checks.safix-generate =
        mode "safix-generate" "generators"
          "generate_mints_in_dependency_order_and_commits_each_generator";

      # Every way a run is refused: a name with no generator, empty output, a
      # script that exits non-zero, a candidate the validation rejects, a
      # multi-output script that wrote only one of its outputs, and a staging
      # location that is not memory-backed. None leaves a value, a commit, or a
      # scratch file, and a partial keypair is never written.
      checks.safix-generate-refusals =
        mode "safix-generate-refusals" "generators"
          "generate_refusals_each_have_their_own_code_and_write_nothing";

      # A run order carrying a cycle, refused before the first generator rather
      # than at the one whose input never arrives. The plan is one the resolver
      # does not emit — it refuses a cycle at evaluation and leaves the
      # generators inside one out of the order — so the subject here is the
      # runtime's own reading of a plan it did not get from that refusal, which
      # is what a stand-in for nix and an embedder of the library both hand it.
      checks.safix-generate-cycle =
        mode "safix-generate-cycle" "generators"
          "a_run_order_carrying_a_cycle_is_refused_before_anything_runs";

      # What one generator's process may see of another's. A script that reads
      # standard input to end of input does not eat the answer to a later
      # generator's prompt — now true because answers are files rather than a
      # shared stream, which is a different reason and so is re-asserted rather
      # than assumed to carry over — and a generator running last sees exactly
      # the descriptors one running first sees.
      checks.safix-generate-isolation =
        mode "safix-generate-isolation" "generators"
          "one_generator_sees_neither_the_stdin_nor_the_descriptors_of_another";

      # `--regenerate` of a named generator carries everything downstream of it.
      # The set is listed in dependency order and confirmed first; declining
      # writes nothing; accepting leaves every downstream value a function of
      # the value that was just minted rather than of the one it replaced; a
      # generator that reads none of it is not re-run; and --yes answers the
      # confirmation in advance.
      checks.safix-generate-cascade =
        mode "safix-generate-cascade" "generators"
          "a_rotation_carries_its_downstream_set_and_nothing_else";

      # clan's wireguard keypair, ported. One generator, an encrypted private
      # half and a public half stored in the clear and readable with no
      # identity, both from one execution, both in one commit. A re-run mints
      # nothing, a rotation moves both halves together, and editing the public
      # half is refused.
      checks.safix-generate-public =
        mode "safix-generate-public" "generators"
          "a_wireguard_keypair_lands_encrypted_and_in_the_clear_in_one_commit";

      # The definition record, and the drift `check` reports over it. A mint leaves
      # one line under state/safix/definitions/ carrying a digest and none of the
      # value; an edit to the declaration afterwards is reported naming the entry
      # and both remedies and no value; regenerating clears it, with the refreshed
      # record riding that commit; and a hand-set entry, a record in a format this
      # version does not write, and an absent record each produce nothing.
      checks.safix-generate-definition-drift =
        mode "safix-generate-definition-drift" "generators"
          "a_definition_edited_after_a_mint_is_reported_and_a_regeneration_clears_it";

      # The one field of that record whose coverage is a claim about the envelope
      # rather than about the script: a generator that gains `network` describes a
      # mint that may do something the recorded one could not, so the flip alone —
      # same script, same tools, same outputs — has to read as drift. A digest that
      # left the grant out would report nothing here, which is what makes this a
      # drill on the coverage rather than on the report.
      checks.safix-generate-network-drift =
        mode "safix-generate-network-drift" "generators"
          "a_generator_that_gains_the_network_reads_as_definition_drift";

      # `edit`'s four outcomes: a non-zero exit and an emptied buffer write
      # nothing, an unchanged buffer commits nothing, and a changed one goes
      # through the same path `set` writes through. No staging root outlives any
      # of them.
      checks.safix-edit =
        mode "safix-edit" "editor"
          "the_four_outcomes_of_an_edit_write_what_each_is_supposed_to";

      # Neither editor variable set is a refusal naming both, the visual one
      # wins over the other, and an entry holding nothing opens on an empty
      # buffer so that editing is an authoring verb too.
      checks.safix-edit-selection =
        mode "safix-edit-selection" "editor"
          "the_editor_is_the_one_the_operator_named_or_the_run_refuses";

      # The staged path reaches the editor's argument vector and the value does
      # not, read out of the editor's own /proc entry rather than out of a
      # process listing.
      checks.safix-edit-argv =
        mode "safix-edit-argv" "editor"
          "the_editor_receives_the_path_and_never_the_value";

      # The union `fix` acts on, from both sides. A consumer-named file in step
      # with the rule that covers it is not a finding of any kind; the same file
      # drifted from that rule is reported and re-wrapped; and a named path no
      # rule's directory covers is reported as such, because naming a file
      # creates no rule for it.
      checks.safix-governed-extras =
        mode "safix-governed-extras" "read_path"
          "a_governed_extra_is_held_to_its_rule_and_not_to_the_declarations";

      # Declaring a person writes one custody record and commits exactly that
      # and the regenerated policy — not a bystander staged alongside it. The
      # policy carries the person just declared, which is only true if the
      # scaffold was staged before it was regenerated: a flake evaluation reads
      # what git tracks, so regenerating first writes the policy of the
      # declarations as they stood without them. They hold nothing, so their key
      # is an anchor with no rule. Nothing is minted, the output says as much,
      # and redeclaring is refused.
      checks.safix-adduser =
        mode "safix-adduser" "custody"
          "adduser_commits_the_scaffold_and_the_policy_that_saw_it";

      # Every refusal, each for its own reason: a name outside the alphabet, a
      # name carrying a path separator, a malformed recipient, an over-long one,
      # and an existing person. Each leaves no scaffold, no commit and no dirt. A
      # recipient that needs a physical interaction is refused separately from
      # the malformed ones, because it is well-formed and still cannot be an
      # activation identity, and its refusal has to name recoveryRecipients —
      # where a card does belong — or the operator is told only that their key is
      # unwelcome.
      checks.safix-adduser-refusals =
        mode "safix-adduser-refusals" "custody"
          "adduser_refusals_leave_the_tree_as_they_found_it";

      # Host attachment reaches a consumer through the hook or not at all.
      # `--host` with no hook configured is refused naming the hook and saying
      # that onboarding without one succeeds; a configured hook receives the
      # name, the recipient and every host, and runs after safix's commit has
      # landed, so what it writes is left uncommitted and safix's message names
      # only what safix did.
      checks.safix-adduser-hook =
        mode "safix-adduser-hook" "custody"
          "host_attachment_is_refused_without_a_hook_and_handed_to_one_after_the_commit";

      # The enrollment ceremony over one factory-fresh card: a generated PIN and
      # a distinct generated PUK reach ykman as flags, a random management key is
      # put on the card and named nowhere, the generator is answered once on a
      # terminal, the identity block lands where keygen appends, the recipient
      # lands in recoveryRecipients, the credentials are stored through the
      # ordinary write path, and no argument vector anywhere names the OTP
      # applet. The proof does not pass, because no card is present and the
      # isolation is what decides that — see the head of
      # `crates/safix/tests/enrollment.rs`.
      checks.safix-enroll =
        mode "safix-enroll" "enrollment"
          "enrollment_provisions_generates_wires_and_commits_once";

      # A card already provisioned keeps its access: the three drives that would
      # change a PIN, a PUK or a management key record nothing at all, and the PIN
      # comes from the operator instead — asked once, unechoed, and used to answer
      # the generator. The state probe is what decides between the two paths, and
      # it costs no PIN retry.
      checks.safix-enroll-provisioned =
        mode "safix-enroll-provisioned" "enrollment"
          "a_provisioned_card_keeps_its_access_and_the_pin_is_asked_for_once";

      # A backup card is the same verb run again: its own identity, its own
      # recipient beside the first, and neither run knowing about the other.
      checks.safix-enroll-backup =
        mode "safix-enroll-backup" "enrollment"
          "a_backup_card_sits_beside_the_first_and_changes_nothing_about_it";

      # Every refusal the card surface produces, each for its own reason: two
      # cards with no serial named, no smartcard service, no card, a touch policy
      # of never, and an OTP slot asked for under any of the spellings somebody
      # would reach for. The OTP one is refused with the database-lockout hazard
      # named rather than as an unknown option, which is the whole point of it
      # being a refusal.
      checks.safix-enroll-refusals =
        mode "safix-enroll-refusals" "enrollment"
          "the_card_refusals_each_have_their_own_code_and_leave_the_tree_alone";

      # A PIN the card refuses costs one retry and not three. The claim is the
      # count: a run that answered every prompt would walk a card's counter to
      # zero and block it, which is a card nobody can use again without the PUK.
      checks.safix-enroll-one-attempt =
        mode "safix-enroll-one-attempt" "enrollment"
          "a_rejected_pin_aborts_after_one_attempt";

      # The same bounded-answer discipline at the card's own boundary, where the
      # credentials travel a prompt rather than an argument vector: a drive that
      # asks past its bound is not answered further, and the run stops with
      # nothing wired and nothing committed.
      checks.safix-enroll-one-attempt-ykman =
        mode "safix-enroll-one-attempt-ykman" "enrollment"
          "a_ykman_drive_that_asks_past_its_bound_stops_the_run";

      # The proof's isolation, which is what makes the proof about the card. An
      # ambient software identity opens the file the proof names, and the run is
      # handed an identity source holding one line — the card's stub — so a proof
      # that passed with no card would mean the isolation had failed.
      checks.safix-enroll-proof-isolation =
        mode "safix-enroll-proof-isolation" "enrollment"
          "the_proof_is_isolated_from_every_ambient_identity";

      # The proof machinery's passing path, hardware-free: the isolated source
      # opens a file it is a recipient of, and one that is not a recipient does
      # not. A separate check from the isolation because wrapping a data key to a
      # card's recipient runs the plugin, and the plugin runs the card.
      checks.safix-enroll-proof =
        mode "safix-enroll-proof" "enrollment"
          "the_proof_opens_a_file_with_the_isolated_source_alone";

      # Registration reaches clan through clan's own command and the consumer
      # through flake.safix.enrollHook, which receives the person, the serial and
      # the recipient, and runs after safix's commit has landed — so what it
      # writes is left uncommitted and safix's message names only what safix did.
      checks.safix-enroll-hook =
        mode "safix-enroll-hook" "enrollment"
          "clan_and_the_hook_receive_the_enrollment_after_it_is_committed";

      # The credentials' second home: they reach the store on standard input,
      # round-trip through the same transport, and neither store's argument vector
      # ever carries one.
      checks.safix-enroll-custody =
        mode "safix-enroll-custody" "enrollment"
          "the_mirrored_credentials_travel_standard_input_and_round_trip";

      # A manager scaffolding for somebody who consented to that: the run proceeds
      # and its commit records the organization it was performed for, in the same
      # words the run announced before it edited anything.
      checks.safix-delegation =
        mode "safix-delegation" "delegation"
          "a_manager_scaffolds_for_a_managed_person_and_the_commit_records_the_organization";

      # The two refusals, before the card is selected and before any file is
      # written: a declared person outside the delegation, and an identity no
      # declaration corresponds to. Neither reaches the card, neither commits, and
      # neither leaves the record it was refused over changed.
      checks.safix-delegation-refusals =
        mode "safix-delegation-refusals" "delegation"
          "an_out_of_scope_actor_is_refused_before_the_card_and_before_any_file";

      # A person no delegation covers, scaffolded by an identity the declarations do
      # not name at all. It proceeds, which is the sharpest form the compatibility
      # promise takes: a verb that consulted delegation there would refuse.
      checks.safix-delegation-unmanaged =
        mode "safix-delegation-unmanaged" "delegation"
          "an_unmanaged_person_never_consults_delegation";

      # A group gains a member: one inserted line, every name and comment that was
      # there kept, the recipient policy regenerated from the declarations that
      # edit implies and committed beside it, and the delegation recorded in the
      # commit. A second run writes nothing and commits nothing.
      checks.safix-group-add =
        mode "safix-group-add" "group"
          "an_addition_is_one_line_and_the_policy_is_re_derived_beside_it";

      # A group loses one: one removed line, the not-retroactive disclosure made,
      # and the next `check` reporting the shrink as the revocation the verb said it
      # was.
      checks.safix-group-remove =
        mode "safix-group-remove" "group"
          "a_removal_says_what_it_does_not_undo_and_the_next_check_reports_the_shrink";

      # The delegation over groups, which is silo coverage: a covered group is its
      # organization's managers' to edit, and a group no silo set names is editable
      # by whoever can commit with nothing consulted and nothing mentioned.
      checks.safix-group-delegation =
        mode "safix-group-delegation" "group"
          "a_covered_group_is_its_organizations_and_an_uncovered_one_is_anybodys";

      # Every refusal the verb has, each for its own reason: an undeclared group, an
      # undeclared subject, a declaration this cannot edit, a `members` value it
      # cannot read, and an act that is neither add nor remove. None leaves the
      # declaration or HEAD moved.
      checks.safix-group-refusals =
        mode "safix-group-refusals" "group"
          "refusals_each_have_their_own_code_and_leave_the_declaration_alone";

      # One mapping of each mode over one run: the database converges to safix,
      # safix converges to the database through the ordinary write path, a two-way
      # mapping with an empty database side bootstraps and records its agreement
      # beside the entry, and a backup mapping writes into absence. The username a
      # mapping declares reaches the entry, no value reaches standard output, and
      # no digest of one reaches the repository.
      checks.safix-sync = mode "safix-sync" "sync_path" "each_mode_converges_exactly_as_its_name_says";

      # Convergence, which is load-bearing rather than an optimisation here: a kdbx
      # save rewrites the whole file. A second run over the same tree reports every
      # mapping unchanged, commits nothing, moves no ciphertext, and issues no
      # write of any kind against the database — asserted from the store's own
      # invocation log rather than from the report.
      checks.safix-sync-converges =
        mode "safix-sync-converges" "sync_path"
          "a_second_run_writes_nothing_anywhere";

      # A pulled value lands as a commit indistinguishable in shape from a
      # hand-set write — the same paths, a subject naming the mapping, and no value
      # in the message.
      checks.safix-sync-pull =
        mode "safix-sync-pull" "sync_path"
          "a_pulled_value_lands_as_a_commit_shaped_like_a_hand_set_write";

      # The three-way decision, over the agreement the companion entry remembers:
      # one side moved converges toward it in each direction, and both sides moved
      # writes nothing and names the two one-way modes that each resolve it.
      checks.safix-sync-two-way =
        mode "safix-sync-two-way" "sync_path"
          "two_way_converges_toward_the_side_that_moved_and_will_not_guess_when_both_did";

      # backup's whole content: a database value that differs is reported and never
      # overwritten.
      checks.safix-sync-backup =
        mode "safix-sync-backup" "sync_path"
          "a_backup_mapping_never_overwrites_and_reports_the_divergence";

      # Every refusal, each for its own reason: a mapping nothing declares, a safix
      # side holding nothing, a database side holding no entry, a value carrying a
      # newline the store's command cannot carry, a database that will not open, a
      # run with no terminal to ask the password on, and mappings declared with no
      # database. None leaves a commit, a dirty tree, or a partial write.
      checks.safix-sync-refusals =
        mode "safix-sync-refusals" "sync_path"
          "the_refusals_each_have_their_own_code_and_leave_both_sides_alone";

      # A mapping whose safix side does not decrypt for whoever is running is
      # reported as one that could not be judged rather than skipped, and the
      # mappings beside it are still judged.
      checks.safix-sync-unjudgeable =
        mode "safix-sync-unjudgeable" "sync_path"
          "a_mapping_that_cannot_be_judged_is_reported_rather_than_skipped";

      # The burst discipline the 292 MB rewrite is bounded by: every database write
      # of a run is issued consecutively, with no read between two of them.
      checks.safix-sync-burst =
        mode "safix-sync-burst" "sync_path"
          "the_database_writes_of_a_run_are_one_burst";

      # Entries no mapping declares, including the companion of a mapping that is
      # gone, are reported as information and left where they are. No mode deletes.
      checks.safix-sync-leftovers =
        mode "safix-sync-leftovers" "sync_path"
          "an_entry_no_mapping_declares_is_reported_and_never_removed";

      # Neither side holding a value writes nothing anywhere: no clan write,
      # no companion write, no commit.
      checks.safix-bridge-sync-unchanged =
        mode "safix-bridge-sync-unchanged" "bridge_sync"
          "neither_side_holding_anything_is_unchanged_and_writes_nothing";

      # A bootstrap push into clan lands the value through clan's own command
      # and the companion afterward as this repository's own, single new
      # commit — the companion holding a digest tagged `safix-bridge-sync-v1`
      # rather than the plaintext value.
      checks.safix-bridge-sync-push =
        mode "safix-bridge-sync-push" "bridge_sync"
          "safix_only_bootstraps_toward_clan_and_records_the_agreement_as_a_second_commit";

      # A bootstrap pull lands the value as its own commit, and the agreement
      # as a second, separate one afterward — the load-bearing order D8
      # states, held against the repository's own commit history rather than
      # against a reading of the code.
      checks.safix-bridge-sync-pull =
        mode "safix-bridge-sync-pull" "bridge_sync"
          "clan_only_bootstraps_toward_safix_and_records_the_agreement_as_a_second_commit";

      # Both sides moved with no agreement recorded is a conflict rather than
      # a guess: nothing written on either side, and the finding names the
      # mapping and the two one-way remedies.
      checks.safix-bridge-sync-conflict =
        mode "safix-bridge-sync-conflict" "bridge_sync"
          "both_sides_holding_different_values_with_no_agreement_is_a_conflict";

      # A later divergence converges using the agreement a prior bootstrap
      # recorded, proving the companion's own write is read back by a later
      # run rather than only ever written.
      checks.safix-bridge-sync-remembered =
        mode "safix-bridge-sync-remembered" "bridge_sync"
          "a_later_divergence_converges_using_the_recorded_agreement";

      # A two-way push into clan carries the identical stale-generator refusal
      # a safix-to-clan write already has, under the identical condition.
      checks.safix-bridge-sync-stale-generator =
        mode "safix-bridge-sync-stale-generator" "bridge_sync"
          "a_stale_generator_refuses_a_two_way_push_toward_clan";

      # A shared placement's clan side is reached by a machine discovered
      # from clan's own `machines list`, never one the mapping declares — its
      # fixture carries no machine for a declared one to have come from.
      checks.safix-bridge-sync-shared-address =
        mode "safix-bridge-sync-shared-address" "bridge_sync"
          "a_shared_placements_machine_is_discovered_from_clan";

      # The store's own command, driven for real against a database the check
      # creates. Every other sync check drives the model, which answers the vectors
      # safix sends because it was written to; this one establishes that those
      # vectors mean to keepassxc-cli what the runtime thinks they mean. It found
      # one thing no model would have: `ls` prints `[empty]` rather than nothing for
      # a database holding no entry, which the runtime has to skip.
      checks.safix-store-cli = withStore "safix-store-cli" "store_cli" "";

      # A shared entry is one value: both carriers' placements name one file and
      # one key, one of them mints, the other reads back what was minted, and
      # exactly one file in the repository holds the key.
      checks.safix-shared-placement =
        mode "safix-shared-placement" "shared_entries"
          "both_carriers_resolve_one_file_and_read_one_value";

      # A carrier dropped from a shared entry is a revocation, and `check` says
      # so: it names the file still holding the value, names the person who can
      # open it, offers a new value as the remedy, and states that `fix` will not
      # revoke. The finding arrives once, not also as an unclaimed value.
      checks.safix-shared-shrink =
        mode "safix-shared-shrink" "shared_entries"
          "a_dropped_carrier_is_reported_as_a_revocation_naming_the_file_and_the_person";

      # Flipping an entry to shared over values already present is reported as a
      # migration rather than a disclosure, because every reader of the copy left
      # behind is still in the audience — and the choice of which per-carrier
      # value survives is left to the operator.
      checks.safix-shared-flip =
        mode "safix-shared-flip" "shared_entries"
          "a_flip_to_shared_over_existing_values_is_reported_as_a_migration";

      # A narrowed audience — a member left a group, a grant was dropped, a
      # machine changed hands, all one state by the time a report reads it — is
      # reported as the revocation it is: the key's holder named rather than
      # printed, `fix` offered as the alignment, and a new value as the only thing
      # that revokes.
      checks.safix-audience-narrowed =
        mode "safix-audience-narrowed" "subjects"
          "a_narrowed_audience_is_reported_as_the_revocation_it_is";

      # A key on the narrowed file answering to no declared subject is the more
      # alarming half of the same finding, and is reported apart from the subjects
      # that did match rather than swallowed by them.
      checks.safix-audience-orphan =
        mode "safix-audience-orphan" "subjects"
          "a_key_answering_to_nobody_is_reported_apart_from_the_named_subjects";

      # The other direction of the same fact: a widened audience — a member joining
      # a group — leaves the file where it is, so `fix` converges it with a real
      # `sops updatekeys` that adds the recipient and leaves the value readable.
      checks.safix-audience-widened =
        mode "safix-audience-widened" "subjects"
          "a_widened_audience_is_re_wrapped_by_fix";

      # An undeclared machine name is refused before any subprocess runs, with
      # no output directory created (tasks 1.1, 1.4).
      checks.safix-upload-unknown-machine =
        mode "safix-upload-unknown-machine" "upload"
          "an_undeclared_machine_is_refused_before_anything_else_runs";

      # A declared machine with no recipient is refused distinctly, before any
      # identity is read (task 1.4).
      checks.safix-upload-no-recipient =
        mode "safix-upload-no-recipient" "upload"
          "a_declared_machine_with_no_recipient_is_refused_distinctly";

      # A person's declared name is refused with the same message an
      # undeclared machine gets, per D6 (task 1.4).
      checks.safix-upload-not-a-machine =
        mode "safix-upload-not-a-machine" "upload"
          "a_persons_name_is_refused_the_same_way_as_an_undeclared_machine";

      # `--directory` writes exactly the two host-identity files, at the
      # declared paths and modes, and touches no network tool (tasks 2.1-2.3,
      # 2.5).
      checks.safix-upload-directory =
        mode "safix-upload-directory" "upload"
          "directory_mode_writes_the_matching_identity_at_the_declared_paths_and_modes";

      # `--directory` without `--identity` refuses before touching the
      # filesystem (task 2.4).
      checks.safix-upload-directory-needs-identity =
        mode "safix-upload-directory-needs-identity" "upload"
          "directory_without_identity_is_refused_before_touching_the_filesystem";

      # A supplied identity that derives to the wrong recipient is refused
      # before DIR is created, naming both recipients (task 2.2).
      checks.safix-upload-directory-mismatch =
        mode "safix-upload-directory-mismatch" "upload"
          "a_mismatched_identity_is_refused_before_directory_is_created_naming_both_recipients";

      # 2.6, first drill: a recipient one character off the declared one still
      # refuses.
      checks.safix-upload-directory-drift-drill =
        mode "safix-upload-directory-drift-drill" "upload"
          "a_recipient_one_character_different_still_refuses";

      # 2.6, second drill: a null-recipient machine refuses before any identity
      # is read, even one that would otherwise derive to something plausible.
      checks.safix-upload-directory-null-recipient-drill =
        mode "safix-upload-directory-null-recipient-drill" "upload"
          "a_null_recipient_machine_refuses_before_reading_any_identity";

      # 3.6: a matching probe is an honest no-op that opens no write-capable
      # session, asserted against the recorded invocation list rather than
      # against file state alone — the claim this whole change exists for.
      checks.safix-upload-remote-match =
        mode "safix-upload-remote-match" "upload"
          "a_matching_presented_key_is_an_honest_no_op_and_opens_no_session";

      # 3.4: `--force` is inert on the match branch.
      checks.safix-upload-remote-match-force =
        mode "safix-upload-remote-match-force" "upload" "force_is_inert_on_a_match";

      # 3.3, second branch: no key presented writes the given identity, with
      # the recorded invocation order and the `ssh` argv both asserted.
      checks.safix-upload-remote-write =
        mode "safix-upload-remote-write" "upload" "no_key_presented_writes_given_identity";

      # 3.3, second branch without `--identity`: refuses before opening a
      # write-capable session.
      checks.safix-upload-remote-needs-identity =
        mode "safix-upload-remote-needs-identity" "upload"
          "no_key_presented_without_identity_refuses_before_opening_a_session";

      # 3.3, third branch without `--force`: refused by default, naming both
      # recipients.
      checks.safix-upload-remote-mismatch =
        mode "safix-upload-remote-mismatch" "upload"
          "a_different_presented_key_is_refused_by_default";

      # 3.3, third branch with `--force` and `--identity`: the override reaches
      # the transport.
      checks.safix-upload-remote-force =
        mode "safix-upload-remote-force" "upload"
          "a_mismatched_presented_key_is_overridden_with_force_and_identity";

      # 3.7: flipping one byte of the declared recipient turns 3.6's match into
      # a mismatch, proving the branch follows the comparison.
      checks.safix-upload-remote-flip-drill =
        mode "safix-upload-remote-flip-drill" "upload"
          "flipping_the_declared_recipient_turns_a_match_into_a_mismatch";

      # 4.1-4.2, 4.7: the tarball's own contents — both files at mode 0400,
      # root-owned — read back from the real archive.
      checks.safix-upload-tarball-modes =
        mode "safix-upload-tarball-modes" "upload"
          "the_tarball_carries_the_declared_modes_and_root_ownership";

      # 4.5: the staging root is created before the tarball is written and gone
      # after both a success and a simulated transport failure.
      checks.safix-upload-staging-cleanup =
        mode "safix-upload-staging-cleanup" "upload"
          "the_staging_root_is_gone_after_a_success_and_after_a_simulated_failure";

      # 4.3-4.4, 4.6: the wipe-then-extract sequence names the fixed
      # destination; the depth-safety constant it depends on is drilled at the
      # unit level in `upload.rs` itself.
      checks.safix-upload-destination =
        mode "safix-upload-destination" "upload"
          "the_wipe_then_extract_sequence_names_the_fixed_destination";

      # 1.3, 6.3: `safix -h` lists `upload` in the scaffold's operator-facing
      # order.
      checks.safix-upload-help-scaffold =
        mode "safix-upload-help-scaffold" "upload"
          "safix_help_lists_upload_in_table_order_after_group";

      # 6.1, 6.3: `safix upload -h` states the two write modes and the three
      # named absences.
      checks.safix-upload-help-text =
        mode "safix-upload-help-text" "upload"
          "safix_upload_help_states_the_two_modes_and_the_three_absences";

      # Holds the `--entry`/`SAFIX_ENTRY` evaluation path (safix-cli spec) and
      # `generate`'s flakeless refusal, over one fixture fleet declared once as
      # nix source text and evaluated two ways: `nix eval --file <entry>`
      # against a plain expression outside any repository, the same mechanism
      # D1's `mkVault` wraps, and `nix eval <flakeref>#<attr>` against a
      # from-scratch zero-input flake — no flake-parts, no network either way,
      # because neither evaluation resolves a flake input.
      #
      # ── what this holds ──
      # All thirteen `Attribute` spellings evaluate under `--file` exactly as
      # they do under a flake target — the same strings, only how the target
      # is built differs (D4). `generatorPlan`, `bridge` and `keepassxc`
      # deserialize against the real `Generator`, `GeneratorFile`, `Mapping`,
      # `SyncMapping` and `PlanInput` structs with no `deny_unknown_fields`
      # rejection — reached through the real `safix` binary's own `generate`,
      # `sync clan` and `sync`, which is the residual measurement gap the
      # proposal names. The same three attributes are byte-identical between
      # the two evaluation paths. `--entry` overrides a conflicting
      # `SAFIX_ENTRY`. The workspace root a write stages and commits into is
      # still the one git discovers, even with `--entry` pointed at a file
      # outside that repository. `generate` refuses under `--entry` with no
      # `--nixpkgs`/`SAFIX_NIXPKGS` declared and a non-empty generator order,
      # names both remedies, is unaffected for a user with an empty order —
      # the ordering drill — and is unaffected in flake mode regardless of
      # `--nixpkgs`.
      #
      # ── what this cannot check ──
      # That a generator actually runs to completion under `--nixpkgs`: the
      # declared reference resolves inside a nested, network-disabled build
      # sandbox only as far as `nix shell` itself gets, which is enough to
      # prove the refusal lifted and not enough to prove a tool resolves. The
      # assertions below hold the refusal's presence and absence, not the
      # generator's own run — `safix-generate*` already holds that under a
      # flake.
      checks.safix-cli =
        pkgs.runCommand "safix-cli"
          {
            nativeBuildInputs = [
              pkgs.git
              pkgs.nix
            ];
            # The real `safix` binary's own `nix` subprocess calls carry no
            # `--extra-experimental-features` of their own — the same as at an
            # operator's terminal, where the ambient nix.conf already enables
            # them for a flake-based project. The sandbox's nix.conf does not,
            # so this is what the sandbox stands in for that ambient config.
            env.NIX_CONFIG = "experimental-features = nix-command flakes";
          }
          ''
                    set -eu
                    export HOME="$PWD"
                    export GIT_AUTHOR_NAME="safix-cli fixture"
                    export GIT_AUTHOR_EMAIL="fixture@example.invalid"
                    export GIT_COMMITTER_NAME="$GIT_AUTHOR_NAME"
                    export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"

                    nix_eval() {
                      nix --extra-experimental-features "nix-command flakes" eval "$@"
                    }

                    safix="${config.packages.safix}/bin/safix"

                    # The one fleet both evaluation paths declare: alice holds a hand-set
                    # entry and a generated one, and bob holds nothing generated — bob is
                    # the empty-order user the ordering drill (4.7) needs. The bridge and
                    # keepassxc mappings are synthetic: no machine, database or clan named
                    # here exists, matching every other fixture in this directory.
                    fleetText=$(cat <<'FLEET'
                    {
                      users.alice = {
                        recipient = "age1fixtureaaa00000000000000000000000000000000000000000000000";
                        private.tok = { };
                        private.api-token.generator = {
                          script = "printf '%s' fixture > \"$out/api-token\"";
                          runtimeInputs = [ "coreutils" ];
                        };
                      };
                      users.bob.recipient = "age1fixturebbb00000000000000000000000000000000000000000000000";
                      bridge = {
                        clanFlake = null;
                        mappings.a = {
                          direction = "clan-to-safix";
                          clan = {
                            machine = "nonexistent";
                            generator = "ntfy";
                            file = "token";
                          };
                          safix = {
                            user = "alice";
                            name = "tok";
                          };
                        };
                      };
                      keepassxc = {
                        database = "/nonexistent/master.kdbx";
                        group = "safix";
                        mappings.a = {
                          mode = "safix-to-keepassxc";
                          safix = {
                            user = "alice";
                            name = "tok";
                          };
                          kdbx = {
                            path = "alice/grafana";
                          };
                        };
                      };
                      groups.oncall.members = [ ];
                    }
            FLEET
                    )

                    # The flake-mode side: a from-scratch flake declaring only `nixpkgs`
                    # as an input, resolved against this system's own already-fetched
                    # copy so nothing here fetches. Pure evaluation forbids a flake from
                    # reading any absolute path outside its own inputs and `self`, so the
                    # safix module is copied into the fixture tree rather than referenced
                    # by the store path outside it; the `--entry` side below has no such
                    # restriction; and `self` here is the flake's own store path, which is
                    # `mkVault`'s `root` under flake-parts's own mechanism (D2, D3) — the
                    # same `lib.evalModules` call the `--entry` side makes, differing only
                    # in where `self` comes from, which the three compared attributes
                    # never read (model.rs: none of Generator, GeneratorFile, Mapping,
                    # SyncMapping or PlanInput carry a path).
                    repo="$PWD/repo"
                    mkdir -p "$repo/safix/groups"
                    cd "$repo"
                    git init -q
                    cp -r ${../safix} ./safix-module

                    cat > safix/groups/oncall.nix <<'GRP'
                    {
                      flake.safix.groups.oncall.members = [ ];
                    }
            GRP

                    cat > flake.nix <<FLAKE
                    {
                      inputs.nixpkgs.url = "path:${pkgs.path}";
                      outputs = { self, nixpkgs, ... }: {
                        safix =
                          let
                            lib = nixpkgs.lib;
                            projection = (lib.evalModules {
                              modules = [
                                ./safix-module
                                { _module.args.self = self; }
                                { flake.safix = $fleetText; }
                              ];
                            }).config.flake.safix.lib;
                          in {
                            lib = projection;
                            onboardingHook = null;
                            enrollHook = null;
                          };
                      };
                    }
            FLAKE

                    git add -A
                    git commit -q -m fixture

                    # The --entry side: a plain expression outside the repository nix
                    # never sees as a flake at all. `self` is a literal string, which the
                    # design's own scenario admits: "any path value that supports
                    # `+ \"/…\"` concatenation is sufficient."
                    mkdir -p "$PWD/../outside"
                    entry="$PWD/../outside/entry.nix"
                    cat > "$entry" <<ENTRY
                    let
                      lib = import ${pkgs.path}/lib;
                      projection = (lib.evalModules {
                        modules = [
                          ${../safix}
                          { _module.args.self = "/entry-fixture-root"; }
                          { flake.safix = $fleetText; }
                        ];
                      }).config.flake.safix.lib;
                    in {
                      safix = {
                        lib = projection;
                        onboardingHook = null;
                        enrollHook = null;
                      };
                    }
            ENTRY

                    # 3.5: all thirteen attribute spellings evaluate under --file exactly as
                    # they do against the flake target.
                    attrs=(
                      safix.lib.placements safix.lib.audiences safix.lib.governedFiles
                      safix.lib.recipients safix.lib.delegation safix.lib.policyText
                      safix.lib.generatorPlan safix.lib.nameRegex safix.lib.bridge
                      safix.lib.keepassxc safix.lib.subjects safix.onboardingHook safix.enrollHook
                    )
                    formats=(
                      --json --json --json --json --json --raw --json --raw --json --json --json --json --json
                    )
                    for i in "''${!attrs[@]}"; do
                      attr="''${attrs[$i]}"
                      fmt="''${formats[$i]}"
                      nix_eval --file "$entry" "$attr" "$fmt" >/dev/null \
                        || { echo "entry-mode evaluation of $attr failed" >&2; exit 1; }
                      nix_eval "path:$repo#$attr" "$fmt" --no-write-lock-file >/dev/null \
                        || { echo "flake-mode evaluation of $attr failed" >&2; exit 1; }
                    done

                    # 3.7: generatorPlan, bridge and keepassxc are byte-identical between
                    # the two paths.
                    for attr in safix.lib.generatorPlan safix.lib.bridge safix.lib.keepassxc; do
                      entry_json="$(nix_eval --file "$entry" "$attr" --json)"
                      flake_json="$(nix_eval "path:$repo#$attr" --json --no-write-lock-file)"
                      if [ "$entry_json" != "$flake_json" ]; then
                        echo "entry-mode and flake-mode $attr diverge:" >&2
                        echo "  entry: $entry_json" >&2
                        echo "  flake: $flake_json" >&2
                        exit 1
                      fi
                    done

                    # Every CLI invocation below runs from inside $repo, so
                    # Workspace::discover finds $repo as root — with --entry pointed
                    # outside it, which is 3.8's claim.

                    # 3.9 drill: --entry overrides a conflicting SAFIX_ENTRY. Only the
                    # value --entry names is a valid nix expression, so a run that used
                    # SAFIX_ENTRY's instead fails with a broken evaluation rather than
                    # succeeding on bob's empty order.
                    output="$(SAFIX_ENTRY="$PWD/../outside/does-not-exist.nix" "$safix" --entry "$entry" generate bob 2>&1)" \
                      && status=0 || status=$?
                    if [ "$status" != 0 ]; then
                      echo "--entry did not override a conflicting SAFIX_ENTRY:" >&2
                      echo "$output" >&2
                      exit 1
                    fi

                    # 3.6 / 4.5: alice's generatorPlan deserializes (Generator,
                    # GeneratorFile, PlanInput) and the refusal fires before the sandbox
                    # is probed, naming both remedies.
                    output="$("$safix" --entry "$entry" generate alice 2>&1)" && status=0 || status=$?
                    if [ "$status" = 0 ]; then
                      echo "generate alice under --entry with no --nixpkgs did not refuse" >&2
                      exit 1
                    fi
                    case "$output" in
                      *"generate needs a flake or a declared nixpkgs reference"*) ;;
                      *)
                        echo "generate alice refused for the wrong reason:" >&2
                        echo "$output" >&2
                        exit 1
                        ;;
                    esac
                    case "$output" in
                      *"evaluated to a shape this runtime does not read"*)
                        echo "generatorPlan failed to deserialize against the real structs:" >&2
                        echo "$output" >&2
                        exit 1
                        ;;
                    esac

                    # 3.6 continued: bridge (Mapping) and keepassxc (SyncMapping)
                    # deserialize too, reached through sync clan and sync — each refuses
                    # afterwards for an unrelated reason (no clan, no terminal), which is
                    # not what this asserts.
                    output="$("$safix" --entry "$entry" sync clan 2>&1)" || true
                    case "$output" in
                      *"evaluated to a shape this runtime does not read"*)
                        echo "bridge failed to deserialize against the real structs:" >&2
                        echo "$output" >&2
                        exit 1
                        ;;
                    esac
                    output="$("$safix" --entry "$entry" sync 2>&1)" || true
                    case "$output" in
                      *"evaluated to a shape this runtime does not read"*)
                        echo "keepassxc failed to deserialize against the real structs:" >&2
                        echo "$output" >&2
                        exit 1
                        ;;
                    esac

                    # 4.6 / 4.7: the ordering drill. bob's generatorPlan order is empty,
                    # so the empty-order return above the refusal has to fire first: this
                    # asserts directly that it does, rather than only that the refusal
                    # itself exists.
                    output="$("$safix" --entry "$entry" generate bob 2>&1)" && status=0 || status=$?
                    if [ "$status" != 0 ]; then
                      echo "generate bob (no generator) refused under --entry:" >&2
                      echo "$output" >&2
                      exit 1
                    fi
                    case "$output" in
                      *"generate needs a flake or a declared nixpkgs reference"*)
                        echo "the empty-order user was refused -- the ordering drill fired red" >&2
                        exit 1
                        ;;
                    esac

                    # 4.5 continued: a declared --nixpkgs lifts the refusal. The declared
                    # reference is this system's own already-fetched nixpkgs, so nothing
                    # here needs the network; whether the sandbox can go on to build a
                    # tool from it is `safix-generate*`'s claim, not this one.
                    output="$("$safix" --entry "$entry" --nixpkgs "path:${pkgs.path}" generate alice 2>&1)" || true
                    case "$output" in
                      *"generate needs a flake or a declared nixpkgs reference"*)
                        echo "--nixpkgs did not lift the refusal:" >&2
                        echo "$output" >&2
                        exit 1
                        ;;
                    esac

                    # Flake mode is unaffected: an empty-order user succeeds exactly as
                    # under --entry, with no --entry, no --nixpkgs, and no flake to
                    # resolve --nixpkgs against even if it had been given.
                    output="$("$safix" generate bob 2>&1)" && status=0 || status=$?
                    if [ "$status" != 0 ]; then
                      echo "flake-mode generate bob (no generator) refused:" >&2
                      echo "$output" >&2
                      exit 1
                    fi
                    case "$output" in
                      *"generate needs a flake or a declared nixpkgs reference"*)
                        echo "flake mode raised the flake-only refusal, which it must never do" >&2
                        exit 1
                        ;;
                    esac

                    # 3.8: the root a write stages and commits into is still the one git
                    # discovers, with --entry pointed outside that repository.
                    before_head="$(git rev-parse HEAD)"
                    output="$("$safix" --entry "$entry" group add oncall alice 2>&1)" && status=0 || status=$?
                    if [ "$status" != 0 ]; then
                      echo "group add under --entry (pointed outside the repository) refused:" >&2
                      echo "$output" >&2
                      exit 1
                    fi
                    after_head="$(git rev-parse HEAD)"
                    if [ "$before_head" = "$after_head" ]; then
                      echo "group add under --entry committed nothing into the discovered root" >&2
                      exit 1
                    fi
                    if ! grep -q '"alice"' safix/groups/oncall.nix; then
                      echo "group add under --entry did not edit the discovered root's declaration file" >&2
                      exit 1
                    fi

                    touch "$out"
          '';
    };
}
