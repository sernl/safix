//! The generator graph: what a run mints, what it refuses, what one generator's
//! process may see of another's, and how far a rotation carries.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

mod harness;

use harness::{ANA_FILE, Fixture, SHARED_FILE};
use serde_json::json;

/// A generator record with no inputs.
fn plain(script: &str) -> serde_json::Value {
    json!({
        "script": script,
        "runtimeInputs": ["coreutils"],
        "prompts": {}, "dependencies": [], "files": [],
        "validation": null, "description": null,
    })
}

/// A generator with no inputs mints and commits; one with a prompt reads it
/// unechoed and derives from it; one with a dependency runs after the generator
/// that writes what it reads and sees that plaintext down a descriptor; one with
/// several outputs writes both, in different files, in one commit.
#[test]
fn generate_mints_in_dependency_order_and_commits_each_generator() {
    let mut fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token"]);
    fixture.make_sops_file(SHARED_FILE, &["wifi-psk"]);

    fixture.seed_generator(
        "seeded",
        ANA_FILE,
        &[],
        &plain("printf '%s\\n' minted-seed"),
    );
    fixture.seed_generator(
        "derived",
        ANA_FILE,
        &[],
        &json!({
            "script": "printf 'from-%s\\n' \"$(cat \"$in_seeded\")\"",
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": ["seeded"], "files": [],
            "validation": null, "description": null,
        }),
    );
    fixture.seed_generator(
        "prompted",
        ANA_FILE,
        &[],
        &json!({
            "script": "printf 'derived-%s\\n' \"$(cat \"$in_pass_phrase\")\"",
            "runtimeInputs": ["coreutils"],
            "prompts": { "pass-phrase": { "type": "hidden", "description": "the fixture passphrase" } },
            "dependencies": [], "files": [],
            "validation": null, "description": null,
        }),
    );
    // The two halves land in different files, which is what makes the single
    // commit a claim rather than a coincidence.
    fixture.seed_generator(
        "paired",
        ANA_FILE,
        &["paired-pub"],
        &json!({
            "script": "printf '%s' '{\"paired\":\"priv\",\"paired-pub\":\"pub\"}'",
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": [], "files": ["paired-pub"],
            "validation": null, "description": null,
        }),
    );
    fixture.seed_output("paired-pub", SHARED_FILE);

    let before = fixture.head();
    let run = fixture
        .run_with(&["generate", "ana"], "fixture-pass\n")
        .expect_success("the bulk generate run");

    // Dependency order, decided at evaluation and walked by the command: the
    // generator that reads `seeded` cannot have run before the one that writes
    // it, or `cat "$in_seeded"` would have had nothing to read.
    assert_eq!(fixture.value(ANA_FILE, "seeded"), "minted-seed");
    assert_eq!(fixture.value(ANA_FILE, "derived"), "from-minted-seed");
    assert_eq!(fixture.value(ANA_FILE, "prompted"), "derived-fixture-pass");
    assert_eq!(fixture.value(ANA_FILE, "paired"), "priv");
    assert_eq!(fixture.value(SHARED_FILE, "paired-pub"), "pub");

    run.silent_about("fixture-pass");
    run.silent_about("minted-seed");

    // One commit per generator, and the multi-output generator's two files in
    // one of them: a keypair split across two commits is a tree holding halves
    // that do not match.
    assert_ne!(fixture.head(), before, "generate committed nothing");
    let paired = fixture.commit_matching("generate paired, paired-pub");
    assert!(!paired.is_empty(), "no commit names both outputs");
    let mut expected = vec![ANA_FILE.to_owned(), SHARED_FILE.to_owned()];
    expected.sort();
    assert_eq!(fixture.paths_in(&paired), expected);
    assert!(
        !fixture
            .git(&["log", "--format=%s%n%b"])
            .contains("fixture-pass"),
        "a commit message carries a value"
    );

    // A second bulk run mints nothing: every output already holds a value, and
    // that is the difference --regenerate is for.
    let head_before = fixture.head();
    fixture
        .run_with(&["generate", "ana"], "fixture-pass\n")
        .expect_success("the second bulk run");
    assert_eq!(fixture.head(), head_before, "a second bulk run rewrote");

    // --regenerate over one name rotates exactly that name. `rotating` is
    // deterministic, so the rotation that proves the point is the untouched
    // neighbour: its ciphertext comes through byte-identical.
    let neighbour = fixture.ciphertext_lines(ANA_FILE);
    fixture.seed_generator("rotating", ANA_FILE, &[], &plain("printf '%s\\n' rotated"));
    fixture
        .run(&["generate", "ana", "rotating"])
        .expect_success("minting the rotating generator");
    fixture
        .run(&["generate", "--regenerate", "ana", "rotating"])
        .expect_success("rotating it");
    assert_eq!(fixture.value(ANA_FILE, "rotating"), "rotated");
    assert_eq!(
        fixture.ciphertext_lines(ANA_FILE)["api-token"],
        neighbour["api-token"],
        "--regenerate disturbed a neighbouring key's ciphertext"
    );
}

/// Every way a generator's run is refused, each with its own code, none leaving
/// a value, a commit or a scratch file.
#[test]
fn generate_refusals_each_have_their_own_code_and_write_nothing() {
    let mut fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token"]);
    let head_before = fixture.head();
    let mut codes = Vec::new();

    // A name with no generator is refused by naming what to do instead: the
    // operator's next move differs depending on why the value cannot be minted.
    let refused = fixture
        .run(&["generate", "ana", "api-token"])
        .expect_refusal("a name with no generator");
    refused.says("has no generator");
    refused.says("safix set ana api-token");
    codes.push(
        fixture
            .run_graphical(&["generate", "ana", "api-token"])
            .refusal_code(),
    );

    // Empty output is the state a truncated write leaves behind, so it may never
    // be stored as though it were a value.
    fixture.seed_generator("blank", ANA_FILE, &[], &plain("printf '%s' ''"));
    fixture
        .run(&["generate", "ana", "blank"])
        .expect_refusal("an empty generator output")
        .says("produced nothing");
    codes.push(
        fixture
            .run_graphical(&["generate", "ana", "blank"])
            .refusal_code(),
    );

    // A non-zero exit from the script itself, which must not be reported as an
    // empty value: the two have different causes and different fixes.
    fixture.seed_generator(
        "broken",
        ANA_FILE,
        &[],
        &plain("echo diagnostic-on-stderr >&2; exit 3"),
    );
    let refused = fixture
        .run(&["generate", "ana", "broken"])
        .expect_refusal("a failing generator");
    refused.says("exited 3");
    refused.says("diagnostic-on-stderr");
    codes.push(
        fixture
            .run_graphical(&["generate", "ana", "broken"])
            .refusal_code(),
    );

    // Validation refuses a candidate the script was happy to produce, and
    // refuses before anything is written.
    fixture.seed_generator(
        "unvalidated",
        ANA_FILE,
        &[],
        &json!({
            "script": "printf '%s\\n' bad-value",
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": [], "files": [],
            "validation": "grep -q ^good-", "description": null,
        }),
    );
    fixture
        .run(&["generate", "ana", "unvalidated"])
        .expect_refusal("a value the validation rejects")
        .says("validation");
    codes.push(
        fixture
            .run_graphical(&["generate", "ana", "unvalidated"])
            .refusal_code(),
    );

    // A multi-output generator whose script prints the wrong keys writes neither
    // half: a partial keypair is worse than none.
    fixture.seed_generator(
        "halfpair",
        ANA_FILE,
        &["halfpair-pub"],
        &json!({
            "script": "printf '%s' '{\"halfpair\":\"only\"}'",
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": [], "files": ["halfpair-pub"],
            "validation": null, "description": null,
        }),
    );
    fixture.seed_output("halfpair-pub", ANA_FILE);
    fixture
        .run(&["generate", "ana", "halfpair"])
        .expect_refusal("a multi-output generator printing the wrong keys")
        .says("declares outputs");
    codes.push(
        fixture
            .run_graphical(&["generate", "ana", "halfpair"])
            .refusal_code(),
    );

    assert_eq!(
        codes,
        vec![
            "no_generator",
            "generator_produced_nothing",
            "generator_failed",
            "validation_rejected",
            "generator_keys_differ",
        ],
        "each refused generator has its own code"
    );

    // Not one of the five wrote, committed, or left anything behind.
    assert_eq!(fixture.head(), head_before, "a refused generator committed");
    assert!(
        fixture.scratch_files().is_empty(),
        "a refused generator left a scratch file"
    );
    assert_eq!(
        fixture
            .ciphertext_lines(ANA_FILE)
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["api-token".to_owned()],
        "a refused generator left keys in the file"
    );
}

/// What one generator's process may see of another's.
///
/// Both claims are about the boundary between consecutive generators rather than
/// about any one of them, so both need a run of several to be visible at all.
/// The values these generators mint are descriptions of their own process, which
/// is the only way to read a claim about a child's environment out of a run that
/// deliberately renders nothing.
#[test]
fn one_generator_sees_neither_the_stdin_nor_the_descriptors_of_another() {
    let mut fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token"]);

    // `$$` rather than /proc/self, because /proc/self inside a substitution is
    // that substitution's process. Only the numbers are reported: a pipe's inode
    // moves between runs, and the claim is about which descriptors are open.
    let descriptors = "me=$$; out=; for f in /proc/$me/fd/*; do n=${f##*/}; \
                       [ \"$n\" -ge 3 ] || continue; out=\"$out$n \"; done; \
                       printf %s \"${out:-none}\"";

    // Ordered before a generator with a prompt, which is the arrangement that
    // tells the two apart: the command's own stdin is where an operator's prompt
    // answers arrive, so a script inheriting it consumes the answer to every
    // prompt after it. The failure is silent — a prompt that reads end-of-input
    // looks exactly like one nobody answered — so it is asserted on the value the
    // later generator stored rather than on an exit status.
    fixture.seed_generator(
        "aaa-greedy",
        ANA_FILE,
        &[],
        &plain("cat >/dev/null; printf '%s' ate-nothing"),
    );
    // Two probes of one script, one before the generators that open descriptors
    // and one after. Comparing them rather than asserting a literal set is what
    // keeps the claim about leaked descriptors instead of about bash's own
    // bookkeeping, which contributes the same fds to both.
    fixture.seed_generator("bbb-probe-first", ANA_FILE, &[], &plain(descriptors));
    // Two dependencies and a prompt, so three descriptors are open at once and
    // all three have to be closed before the next generator starts.
    fixture.seed_generator(
        "mmm-many",
        ANA_FILE,
        &[],
        &json!({
            "script": "cat \"$in_aaa_greedy\" >/dev/null; cat \"$in_api_token\" >/dev/null; \
                       cat \"$in_secret\" >/dev/null; printf '%s' many-ok",
            "runtimeInputs": ["coreutils"],
            "prompts": { "secret": { "type": "hidden", "description": "the fixture passphrase" } },
            "dependencies": ["aaa-greedy", "api-token"], "files": [],
            "validation": null, "description": null,
        }),
    );
    fixture.seed_generator(
        "nnn-more",
        ANA_FILE,
        &[],
        &json!({
            "script": "cat \"$in_mmm_many\" >/dev/null; cat \"$in_bbb_probe_first\" >/dev/null; \
                       printf '%s' more-ok",
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": ["mmm-many", "bbb-probe-first"], "files": [],
            "validation": null, "description": null,
        }),
    );
    fixture.seed_generator("zzz-probe-last", ANA_FILE, &[], &plain(descriptors));

    fixture
        .run_with(&["generate", "ana"], "fixture-pass\n")
        .expect_success("the bulk run");

    // The prompt belongs to a generator ordered after one whose script read
    // stdin to end of input. It got its answer anyway, so the generator's stdin
    // is not the command's.
    assert_eq!(
        fixture.value(ANA_FILE, "mmm-many"),
        "many-ok",
        "a generator ordered after one that consumed stdin did not get its prompt answered"
    );
    assert_eq!(fixture.value(ANA_FILE, "aaa-greedy"), "ate-nothing");

    // Nothing that ran between them left a descriptor behind. Each of those
    // descriptors carried a decrypted value, so one surviving into a later
    // generator's process is that generator holding plaintext it never declared.
    let first = fixture.value(ANA_FILE, "bbb-probe-first");
    let last = fixture.value(ANA_FILE, "zzz-probe-last");
    assert!(!first.is_empty(), "the first probe stored nothing");
    assert_eq!(
        first, last,
        "a generator running last sees descriptors one running first does not"
    );
}

/// `--regenerate` of a named generator carries everything downstream of it, in
/// dependency order, after saying which and being told to go ahead.
///
/// `base` mints a fresh random value on each run, which is what makes the
/// derivation checkable: a downstream value is asserted to be a function of the
/// value `base` holds now, so a downstream generator that did not re-run holds a
/// function of the retired one and fails. The derivation is concatenation rather
/// than a digest because the test has to state the expected value as a literal,
/// and hashing it here would need the same tool the script used — which is the
/// one machine-dependent thing in a run that is otherwise all fixture.
#[test]
fn a_rotation_carries_its_downstream_set_and_nothing_else() {
    let mut fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token"]);

    fixture.seed_generator(
        "base",
        ANA_FILE,
        &[],
        &plain("head -c 18 /dev/urandom | base64 | tr -d '\\n'"),
    );
    fixture.seed_generator(
        "middle",
        ANA_FILE,
        &[],
        &json!({
            "script": "printf 'mid-%s' \"$(cat \"$in_base\")\"",
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": ["base"], "files": [],
            "validation": null, "description": null,
        }),
    );
    fixture.seed_generator(
        "leaf",
        ANA_FILE,
        &[],
        &json!({
            "script": "printf 'leaf-%s' \"$(cat \"$in_middle\")\"",
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": ["middle"], "files": [],
            "validation": null, "description": null,
        }),
    );
    // Reads nothing of base's, so it is downstream of nothing and must be left
    // alone. Without it the cascade could be "re-run everything" and still pass.
    fixture.seed_generator("aside", ANA_FILE, &[], &plain("printf '%s' untouched"));

    fixture
        .run(&["generate", "ana"])
        .expect_success("the first bulk run");

    let base_before = fixture.value(ANA_FILE, "base");
    let aside_before = fixture.ciphertext_lines(ANA_FILE)["aside"].clone();

    // Declining writes nothing, asserted before the accepting run: a cascade
    // commits as it goes, and a decline that had already written could not be
    // told from one that had not.
    let head_before = fixture.head();
    fixture
        .run_with(&["generate", "--regenerate", "ana", "base"], "n\n")
        .expect_refusal("a declined cascade")
        .says("declined");
    assert_eq!(fixture.head(), head_before, "a declined cascade committed");
    assert_eq!(
        fixture.value(ANA_FILE, "base"),
        base_before,
        "a declined cascade rotated its target"
    );

    // The listing names the downstream set, in order, and nothing else.
    let accepted = fixture
        .run_with(&["generate", "--regenerate", "ana", "base"], "y\n")
        .expect_success("the accepted cascade");
    let listed: Vec<&str> = accepted
        .stderr
        .lines()
        .filter_map(|line| line.strip_prefix("    "))
        .filter(|name| !name.contains(' '))
        .collect();
    assert_eq!(
        listed,
        vec!["base", "middle", "leaf"],
        "the cascade listed the wrong set or the wrong order:\n{}",
        accepted.stderr
    );

    // Every downstream value is a function of the value base holds now.
    let base_now = fixture.value(ANA_FILE, "base");
    assert_ne!(
        base_now, base_before,
        "the cascade did not rotate its target"
    );
    let middle = format!("mid-{base_now}");
    assert_eq!(
        fixture.value(ANA_FILE, "middle"),
        middle,
        "a generator downstream of the rotated value still holds one derived from the retired one"
    );
    assert_eq!(
        fixture.value(ANA_FILE, "leaf"),
        format!("leaf-{middle}"),
        "the cascade stopped short of the second generation downstream"
    );

    // And nothing else moved.
    assert_eq!(
        fixture.ciphertext_lines(ANA_FILE)["aside"],
        aside_before,
        "the cascade re-ran a generator that reads nothing of the rotated value"
    );

    // --yes answers the confirmation in advance. Driven with no stdin at all, so
    // a run that still tried to read one fails here rather than passing on an
    // empty answer.
    fixture
        .run(&["generate", "--regenerate", "--yes", "ana", "base"])
        .expect_success("--yes answering the cascade confirmation");

    // A generator nothing reads is not a cascade and asks nothing.
    fixture
        .run(&["generate", "--regenerate", "ana", "aside"])
        .expect_success("rotating a generator with no dependents");
}
