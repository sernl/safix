//! What `set` does to a repository, and what it refuses to do to one.
//!
//! These are the claims a hand-run `sops` would have made the operator
//! responsible for: that a value lands in the file the declarations place it in
//! and under the key it is read by, that a file created for it acquires its
//! recipients from the creation rules rather than from anywhere else, that
//! setting one key disturbs no other, that a run which changes nothing commits
//! nothing, and that a run which aborts leaves neither a partial file nor a
//! plaintext value behind.
//!
//! Every expectation here is a literal written in the test. Nothing is obtained
//! by running a second implementation, and nothing is re-derived by calling the
//! path that produced it.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
// `too_many_lines` is allowed because one test per retired mode is the unit the
// parity table names: splitting a mode in half to satisfy a line count would
// leave a row of that table naming two tests and neither of them the mode.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

mod harness;

use std::collections::BTreeSet;
use std::time::Duration;

use harness::{ANA_FILE, Fixture, SHARED_FILE};

/// A file the declarations place a secret in, that nobody has run sops on yet,
/// is created through sops so the creation rules choose its recipients.
#[test]
fn set_new_creates_the_file_through_the_creation_rules() {
    let fixture = Fixture::new();

    fixture
        .set("ana", "wifi-psk", "CANARY-shared-value")
        .expect_success("set on a new file");

    assert!(fixture.exists(SHARED_FILE), "the audience file was created");

    // Created THROUGH sops: both halves of the audience are recipients. A file
    // encrypted to the writer alone would satisfy "it is encrypted" and hand the
    // other party a file they cannot open.
    let document = fixture.read(SHARED_FILE);
    assert!(document.contains(&fixture.ana), "encrypted to ana");
    assert!(document.contains(&fixture.bo), "encrypted to bo");
    assert!(document.contains("ENC[AES256_GCM"), "holds sops ciphertext");

    assert_eq!(
        fixture.value(SHARED_FILE, "wifi-psk"),
        "CANARY-shared-value",
        "the value round-trips under the resolved key"
    );

    assert_eq!(
        fixture.subject("HEAD"),
        "chore(safix): set wifi-psk for ana",
        "the commit names the secret"
    );
    assert_eq!(
        fixture.paths_in("HEAD"),
        vec![SHARED_FILE.to_owned()],
        "the commit is the one file"
    );
    assert!(
        !fixture.message("HEAD").contains("CANARY-shared-value"),
        "the commit message carries the value"
    );
    assert_eq!(fixture.status(), "", "the tree is clean after the write");

    // One rule per audience, from the other side: a file created for ana alone
    // must not name bo. A single rule covering both would hand a grant's
    // recipient everything its owner holds and pass every assertion above.
    fixture.make_sops_file(ANA_FILE, &["api-token"]);
    assert!(
        !fixture.read(ANA_FILE).contains(&fixture.bo),
        "ana's own file names bo as a recipient"
    );
}

/// One key moves and the rest of the file comes through byte-identical; the same
/// value twice leaves the file untouched and commits nothing.
#[test]
fn set_existing_moves_one_key_and_leaves_the_others_byte_identical() {
    let fixture = Fixture::new();
    fixture.make_sops_file(
        ANA_FILE,
        &[
            "api-token",
            "mail-password",
            "bystander-one",
            "bystander-two",
        ],
    );

    let bystanders = ["mail-password", "bystander-one", "bystander-two"];
    let before = fixture.ciphertext_lines(ANA_FILE);
    let head_before = fixture.head();

    fixture
        .set("ana", "api-token", "CANARY-api-v1")
        .expect_success("set on an existing file");

    assert_eq!(fixture.value(ANA_FILE, "api-token"), "CANARY-api-v1");
    let after = fixture.ciphertext_lines(ANA_FILE);
    for key in bystanders {
        assert_eq!(
            after.get(key),
            before.get(key),
            "bystander key '{key}' was disturbed by the write"
        );
    }
    assert_ne!(fixture.head(), head_before, "the write produced no commit");

    // Idempotent, and severe only because the second boundary is waited out:
    // sops stamps `lastmodified` at one-second resolution and reuses an
    // unchanged value's IV, so a re-run inside the same second is byte-identical
    // whether or not `--idempotent` was passed. Without the wait the assertion
    // would hold over a command that had dropped the flag.
    let snapshot = fixture.read(ANA_FILE);
    let head_after = fixture.head();
    std::thread::sleep(Duration::from_millis(1_100));
    let rerun = fixture
        .set("ana", "api-token", "CANARY-api-v1")
        .expect_success("the idempotent re-run");
    assert_eq!(
        fixture.read(ANA_FILE),
        snapshot,
        "re-setting the same value rewrote the file"
    );
    assert_eq!(fixture.head(), head_after, "the re-run made a commit");
    rerun.says("unchanged");

    // A different value moves the target key and nothing else.
    let before = fixture.ciphertext_lines(ANA_FILE);
    fixture
        .set("ana", "api-token", "CANARY-api-v2")
        .expect_success("the rotation");
    assert_eq!(fixture.value(ANA_FILE, "api-token"), "CANARY-api-v2");
    let after = fixture.ciphertext_lines(ANA_FILE);
    for key in bystanders {
        assert_eq!(
            after.get(key),
            before.get(key),
            "bystander key '{key}' was disturbed by the rotation"
        );
    }

    // An entry may name a key that differs from the secret's name, and the value
    // follows the key. Writing under the name would leave a profile reading an
    // absent key while this reported success.
    fixture
        .set("ana", "aliased-secret", "CANARY-aliased")
        .expect_success("the aliased set");
    assert_eq!(fixture.value(ANA_FILE, "custom-key"), "CANARY-aliased");
    assert!(
        !fixture
            .ciphertext_lines(ANA_FILE)
            .contains_key("aliased-secret"),
        "an entry with a sopsKey also wrote a key named after the secret"
    );

    // A mistyped confirmation writes nothing at all.
    let snapshot = fixture.read(ANA_FILE);
    fixture
        .set_confirming("ana", "api-token", "CANARY-typo-a", "CANARY-typo-b")
        .expect_refusal("a mismatched confirmation");
    assert_eq!(
        fixture.read(ANA_FILE),
        snapshot,
        "a mismatched confirmation still wrote the file"
    );
}

/// Every refusal, each for its own reason, none of them writing anything.
///
/// The claim is not that the run fails: it is that each condition produces its
/// own code and its own prose, so an operator's next move differs by why the
/// value could not be stored.
#[test]
fn refusals_each_have_their_own_code_and_leave_the_tree_alone() {
    let fixture = Fixture::new();
    let mut codes = Vec::new();

    // A name no declaration covers, named against all three declaration
    // surfaces — and against no option path outside safix's namespace.
    let refused = fixture
        .set("ana", "not-declared-anywhere", "CANARY-unknown")
        .expect_refusal("an undeclared name");
    refused.says("flake.safix.catalogue.not-declared-anywhere");
    refused.says("flake.safix.users.ana.private.not-declared-anywhere");
    refused.says("sharedWith.ana.not-declared-anywhere");
    refused.silent_about("flake.users.");
    refused.silent_about("flake.homeSecrets.");
    assert_eq!(fixture.status(), "", "the refused name touched the tree");
    codes.push(
        fixture
            .run_graphical_with(
                &["set", "ana", "not-declared-anywhere"],
                "CANARY-unknown\nCANARY-unknown\n",
            )
            .refusal_code(),
    );

    // A declared name whose file the policy writes no rule for. The refusal
    // names the regenerator rather than writing an unruled file: sops emits
    // nothing and exits non-zero when no creation rule matches, so a command
    // redirecting straight to the final path would leave an empty unruled file
    // beside the others.
    let refused = fixture
        .set("ana", "no-rule-secret", "CANARY-norule")
        .expect_refusal("a path with no creation rule");
    refused.says("no creation rule");
    refused.says("safix fix");
    assert!(
        !fixture.exists("secrets/safix/users/cy/secrets.yaml"),
        "an unruled file was created"
    );
    assert_eq!(fixture.status(), "", "the no-rule refusal left something");
    codes.push(
        fixture
            .run_graphical_with(
                &["set", "ana", "no-rule-secret"],
                "CANARY-norule\nCANARY-norule\n",
            )
            .refusal_code(),
    );

    // A placement outside `*.yaml`. Every generated rule ends in `\.yaml$` so a
    // sweep can never reach encrypted material safix did not place.
    let refused = fixture
        .set("ana", "not-yaml", "CANARY-notyaml")
        .expect_refusal("a non-yaml placement");
    refused.says("not a *.yaml path");
    assert!(
        !fixture.exists("secrets/safix/users/ana/secret.age"),
        "a non-yaml file was written"
    );
    codes.push(
        fixture
            .run_graphical_with(&["set", "ana", "not-yaml"], "x\nx\n")
            .refusal_code(),
    );

    // An empty value is the written-but-empty state a truncated write leaves,
    // and a probe matching the key name alone would call it converged.
    fixture
        .run_with(&["set", "ana", "api-token"], "\n\n")
        .expect_refusal("an empty value");
    codes.push(
        fixture
            .run_graphical_with(&["set", "ana", "api-token"], "\n\n")
            .refusal_code(),
    );

    // An unknown user is a distinct refusal from an unknown name.
    let refused = fixture
        .set("cy", "api-token", "CANARY-nouser")
        .expect_refusal("an undeclared user");
    refused.says("not a declared user");
    codes.push(
        fixture
            .run_graphical_with(&["set", "cy", "api-token"], "x\nx\n")
            .refusal_code(),
    );

    // A dirty target file: committing it would carry an edit this command did
    // not make under a message naming one secret.
    fixture.make_sops_file(ANA_FILE, &["api-token"]);
    let edited = format!("{}hand edit\n", fixture.read(ANA_FILE));
    fixture.write(ANA_FILE, &edited);
    let refused = fixture
        .set("ana", "api-token", "CANARY-dirty")
        .expect_refusal("a dirty target file");
    refused.says("uncommitted changes");
    codes.push(
        fixture
            .run_graphical_with(&["set", "ana", "api-token"], "x\nx\n")
            .refusal_code(),
    );
    fixture.git(&["checkout", "--", ANA_FILE]);

    // Mid-merge and mid-rebase: a partial commit means something else there.
    let git_dir = fixture.git(&["rev-parse", "--absolute-git-dir"]);
    std::fs::write(format!("{git_dir}/MERGE_HEAD"), "").unwrap();
    let refused = fixture
        .set("ana", "api-token", "CANARY-merge")
        .expect_refusal("a run mid-merge");
    refused.says("mid-MERGE_HEAD");
    codes.push(
        fixture
            .run_graphical_with(&["set", "ana", "api-token"], "x\nx\n")
            .refusal_code(),
    );
    std::fs::remove_file(format!("{git_dir}/MERGE_HEAD")).unwrap();

    std::fs::create_dir(format!("{git_dir}/rebase-merge")).unwrap();
    fixture
        .set("ana", "api-token", "CANARY-rebase")
        .expect_refusal("a run mid-rebase")
        .says("mid-rebase-merge");
    std::fs::remove_dir(format!("{git_dir}/rebase-merge")).unwrap();

    // An unrecognised subcommand names the set it accepts rather than failing
    // bare.
    let refused = fixture
        .run(&["frobnicate"])
        .expect_refusal("an unknown subcommand");
    refused.says("unknown subcommand");
    refused.says("adduser");
    codes.push(fixture.run_graphical(&["frobnicate"]).refusal_code());

    assert_eq!(
        codes,
        vec![
            "unknown_name",
            "no_creation_rule",
            "not_a_yaml_path",
            "empty_value",
            "unknown_user",
            "uncommitted_changes",
            "mid_operation",
            "unknown_subcommand",
        ],
        "each refusal condition has its own code"
    );
    assert_eq!(
        codes.iter().collect::<BTreeSet<_>>().len(),
        codes.len(),
        "two conditions share a code"
    );
    assert_eq!(fixture.status(), "", "a refusal left the tree dirty");
}

/// A file whose recipients have drifted from the audience declared for it is
/// refused before the rename, in both directions.
///
/// `sops set` on an existing file reuses that file's own recipient metadata, so
/// a value minted into a drifted file would be wrapped for the audience that
/// used to be — and committed, which hands a removed reader a value minted after
/// their removal straight out of git history.
#[test]
fn recipient_drift_is_refused_before_anything_is_written() {
    let fixture = Fixture::new();
    let stranger = fixture.new_recipient();

    fixture.encrypt_to(
        ANA_FILE,
        &[&fixture.ana, &stranger],
        "api-token: \"fixture-value-for-api-token\"\n",
    );
    fixture.git(&["add", "--", ANA_FILE]);
    fixture.git(&[
        "commit",
        "-q",
        "-m",
        "fixture: recipients drifted from the declared audience",
    ]);
    assert!(
        fixture.read(ANA_FILE).contains(&stranger),
        "the fixture file is not actually drifted"
    );

    let head_before = fixture.head();
    let document_before = fixture.read(ANA_FILE);

    let refused = fixture
        .set("ana", "api-token", "CANARY-DRIFT-abcdef")
        .expect_refusal("a value minted into a drifted file");
    refused.says(&stranger);
    refused.says(ANA_FILE);
    refused.says("safix fix");

    // The whole claim: the run left nothing behind. A refusal that had already
    // renamed the scratch file into place would fail here while its message
    // still read correctly.
    assert_eq!(fixture.head(), head_before, "the refusal made a commit");
    assert_eq!(
        fixture.read(ANA_FILE),
        document_before,
        "the refusal rewrote the target file"
    );
    assert_eq!(fixture.status(), "", "the refusal left the tree dirty");
    assert!(
        fixture.scratch_files().is_empty(),
        "a scratch file was left beside the target"
    );
    assert!(
        fixture.holds_anywhere("CANARY-DRIFT-abcdef").is_none(),
        "the refused value survived"
    );

    // The other direction, and the other write path: a file that does not exist
    // yet takes its recipients from `.sops.yaml`, so the drift that reaches it
    // is a stale creation rule rather than stale metadata. Judging the file
    // already in place would miss this arm entirely — there is no file in place.
    fixture.write_policy(&["ana"]);
    fixture.git(&["add", "--", ".sops.yaml"]);
    fixture.git(&[
        "commit",
        "-q",
        "-m",
        "fixture: creation rule narrower than the declared audience",
    ]);

    let head_before = fixture.head();
    fixture
        .set("ana", "wifi-psk", "CANARY-narrowed")
        .expect_refusal("a value minted into a file one of its audience cannot open")
        .says(&fixture.bo);

    assert!(
        !fixture.exists(SHARED_FILE),
        "the refused creation left the file behind"
    );
    assert!(
        !fixture.exists("secrets/safix/shared/ana,bo"),
        "the refused creation left the audience directory behind"
    );
    assert!(
        !fixture.exists("secrets/safix/shared"),
        "the refused creation left the shared/ parent behind"
    );
    assert_eq!(fixture.head(), head_before, "the refusal made a commit");
    assert_eq!(fixture.status(), "", "the refusal left the tree dirty");

    // Repaired, the same set goes through. `sops updatekeys` is what `safix fix`
    // runs, and the rule grants ana alone, so it drops the extra identity.
    fixture.write_policy(&["ana", "bo"]);
    fixture.git(&["add", "--", ".sops.yaml"]);
    fixture.git(&[
        "commit",
        "-q",
        "-m",
        "fixture: creation rule back in step with the audience",
    ]);
    fixture.updatekeys(ANA_FILE);
    fixture.git(&["add", "--", ANA_FILE]);
    fixture.git(&[
        "commit",
        "-q",
        "-m",
        "fixture: re-wrapped to the declared audience",
    ]);
    assert!(
        !fixture.read(ANA_FILE).contains(&stranger),
        "the re-wrap did not drop the extra recipient"
    );

    let head_before = fixture.head();
    fixture
        .set("ana", "api-token", "CANARY-after-rewrap")
        .expect_success("the set after the drift was repaired");
    assert_ne!(fixture.head(), head_before, "the repaired set committed");
    assert_eq!(
        fixture.value(ANA_FILE, "api-token"),
        "CANARY-after-rewrap",
        "the repaired set did not store the value"
    );
}

/// Another path's staged change survives the run staged and uncommitted.
///
/// An unscoped commit would sweep it into a commit whose message names one
/// secret, and an unscoped emptiness test would read it as this command's own
/// work and commit on a run that wrote nothing.
#[test]
fn a_staged_bystander_survives_the_run_and_does_not_make_it_commit() {
    let fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token"]);

    fixture.write("unrelated.txt", "unrelated work in progress\n");
    fixture.git(&["add", "--", "unrelated.txt"]);

    fixture
        .set("ana", "api-token", "CANARY-scoped")
        .expect_success("the scoped set");

    assert_eq!(
        fixture.paths_in("HEAD"),
        vec![ANA_FILE.to_owned()],
        "the commit reached beyond the target"
    );
    assert_eq!(
        fixture.staged(),
        vec!["unrelated.txt".to_owned()],
        "the unrelated staging did not survive"
    );
    assert_eq!(
        fixture.read("unrelated.txt"),
        "unrelated work in progress\n",
        "the unrelated file's content was disturbed"
    );

    let head_before = fixture.head();
    fixture
        .set("ana", "api-token", "CANARY-scoped")
        .expect_success("the scoped re-run");
    assert_eq!(
        fixture.head(),
        head_before,
        "an unrelated staged path made the idempotent re-run commit"
    );
}

/// An interrupted run and a failing backend both leave the tree as they found
/// it, and the value nowhere on disk.
#[test]
fn an_aborted_run_leaves_no_file_no_scratch_and_no_value() {
    let fixture = Fixture::new();

    // A SIGINT while the prompt is waiting. Standard input is a pipe nobody
    // writes to, so the read blocks until the signal arrives, and the exit
    // status is what tells an interrupted run from one that ran out of input.
    let interrupted = fixture.interrupt_after("2", "INT", &["set", "ana", "wifi-psk"], "", &[]);
    assert_eq!(interrupted.code, Some(130), "the run was not interrupted");

    assert!(
        !fixture.exists(SHARED_FILE),
        "the interrupted run left a partial file behind"
    );
    assert!(
        fixture.scratch_files().is_empty(),
        "the interrupted run left a scratch file behind"
    );
    // `mkdir -p` creates two levels for a first shared audience, so both go.
    assert!(
        !fixture.exists("secrets/safix/shared/ana,bo"),
        "the interrupted run left the audience directory behind"
    );
    assert!(
        !fixture.exists("secrets/safix/shared"),
        "the interrupted run left the shared/ parent behind"
    );
    assert_eq!(
        fixture.status(),
        "",
        "the interrupted run left the tree dirty"
    );

    // A backend that fails after the value has been read. The value is a canary
    // and must appear nowhere on disk; the target file must be untouched so the
    // next run retries rather than finding a half-written one.
    fixture.make_sops_file(ANA_FILE, &["api-token", "bystander-one"]);
    let before = fixture.ciphertext_lines(ANA_FILE);
    fixture
        .run_env(
            &["set", "ana", "api-token"],
            Some("CANARY-LEAK-abcdef\nCANARY-LEAK-abcdef\n"),
            &[("SAFIX_SOPS", "false")],
        )
        .expect_refusal("a failing backend");

    assert!(
        fixture.holds_anywhere("CANARY-LEAK-abcdef").is_none(),
        "the value survived the aborted run"
    );
    assert_eq!(
        fixture.ciphertext_lines(ANA_FILE),
        before,
        "the failing backend still moved the target key"
    );
    assert!(
        fixture.scratch_files().is_empty(),
        "a scratch file was left beside the target"
    );
    assert_eq!(fixture.status(), "", "the failed run left the tree dirty");
}
