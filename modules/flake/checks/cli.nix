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
    };
}
