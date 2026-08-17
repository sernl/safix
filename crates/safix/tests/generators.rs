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

/// A generator record with no inputs, writing one output named for the entry.
fn plain(script: &str) -> serde_json::Value {
    json!({
        "script": script,
        "network": false,
        "runtimeInputs": ["coreutils"],
        "prompts": {}, "dependencies": [], "files": {},
        "share": false, "validation": null, "description": null,
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
        &plain("printf '%s' minted-seed > \"$out/seeded\""),
    );
    // `$in/<producer>/<name>`: the directory is named for the generator that
    // writes the dependency, which is clan's keying, and the file for the output.
    fixture.seed_generator(
        "derived",
        ANA_FILE,
        &[],
        &json!({
            "script": "printf 'from-%s' \"$(cat \"$in/seeded/seeded\")\" > \"$out/derived\"",
            "network": false,
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": ["seeded"], "files": {},
            "share": false, "validation": null, "description": null,
        }),
    );
    // Read twice, which the descriptor interface this replaced could not do.
    fixture.seed_generator(
        "prompted",
        ANA_FILE,
        &[],
        &json!({
            "script": "printf 'derived-%s%s' \"$(cat \"$prompts/pass-phrase\")\" \
                       \"$(cat \"$prompts/pass-phrase\" | wc -c)\" > \"$out/prompted\"",
            "network": false,
            "runtimeInputs": ["coreutils"],
            "prompts": { "pass-phrase": { "type": "hidden", "description": "the fixture passphrase" } },
            "dependencies": [], "files": {},
            "share": false, "validation": null, "description": null,
        }),
    );
    // Two encrypted halves in two files, which is what makes the single commit a
    // claim rather than a coincidence.
    fixture.seed_generator(
        "paired",
        ANA_FILE,
        &["paired-pub"],
        &json!({
            "script": "printf priv > \"$out/paired\"; printf pub > \"$out/paired-pub\"",
            "network": false,
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": [], "files": { "paired-pub": { "secret": true } },
            "share": false, "validation": null, "description": null,
        }),
    );
    fixture.seed_output("paired-pub", SHARED_FILE);

    let before = fixture.head();
    let run = fixture
        .run_with(&["generate", "ana"], "fixture-pass\n")
        .expect_success("the bulk generate run");

    // Dependency order, decided at evaluation and walked by the command: the
    // generator that reads `seeded` cannot have run before the one that writes
    // it, or `cat "$in/seeded/seeded"` would have had nothing to read.
    assert_eq!(fixture.value(ANA_FILE, "seeded"), "minted-seed");
    assert_eq!(fixture.value(ANA_FILE, "derived"), "from-minted-seed");
    assert_eq!(
        fixture.value(ANA_FILE, "prompted"),
        "derived-fixture-pass12",
        "the prompt file was not re-readable, or a byte was added to it"
    );
    assert_eq!(fixture.value(ANA_FILE, "paired"), "priv");
    assert_eq!(fixture.value(SHARED_FILE, "paired-pub"), "pub");

    run.silent_about("fixture-pass");
    run.silent_about("minted-seed");

    // One commit per generator, and the multi-output generator's two files in
    // one of them: a keypair split across two commits is a tree holding halves
    // that do not match. Each output's definition record rides that same commit,
    // which is the property `check`'s drift finding rests on.
    assert_ne!(fixture.head(), before, "generate committed nothing");
    let paired = fixture.commit_matching("generate paired, paired-pub");
    assert!(!paired.is_empty(), "no commit names both outputs");
    let mut expected = vec![
        ANA_FILE.to_owned(),
        SHARED_FILE.to_owned(),
        "state/safix/definitions/ana/paired".to_owned(),
        "state/safix/definitions/ana/paired-pub".to_owned(),
    ];
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
    fixture.seed_generator(
        "rotating",
        ANA_FILE,
        &[],
        &plain("printf '%s' rotated > \"$out/rotating\""),
    );
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
    fixture.seed_generator("blank", ANA_FILE, &[], &plain("printf '' > \"$out/blank\""));
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
            "script": "printf '%s' bad-value > \"$out/unvalidated\"",
            "network": false,
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": [], "files": {},
            "share": false, "validation": "grep -q ^good-", "description": null,
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

    // A multi-output generator that wrote only one of its outputs writes
    // neither: a partial keypair is worse than none. The refusal names the
    // missing output and lists what $out did hold, which is what tells a
    // misspelled name apart from a script that wrote nothing.
    fixture.seed_generator(
        "halfpair",
        ANA_FILE,
        &["halfpair-pub"],
        &json!({
            "script": "printf only > \"$out/halfpair\"",
            "network": false,
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": [], "files": { "halfpair-pub": { "secret": true } },
            "share": false, "validation": null, "description": null,
        }),
    );
    fixture.seed_output("halfpair-pub", ANA_FILE);
    let refused = fixture
        .run(&["generate", "ana", "halfpair"])
        .expect_refusal("a multi-output generator writing only one output");
    refused.says("did not write a file for 'halfpair-pub'");
    refused.says("$out held:");
    refused.says("- halfpair");
    codes.push(
        fixture
            .run_graphical(&["generate", "ana", "halfpair"])
            .refusal_code(),
    );

    // No memory-backed filesystem, which is the containment refusing rather than
    // falling back.
    //
    // A directory the kernel says is disk-backed has to be found rather than
    // assumed: this suite's own scratch is on tmpfs by design, so pointing the
    // drill at it would assert nothing. The candidates are the places a
    // disk-backed directory plausibly is, and where none of them is one the
    // drill says so rather than passing quietly — a check that silently stopped
    // asserting is the failure this whole file is arranged against.
    fixture.seed_generator(
        "staged",
        ANA_FILE,
        &[],
        &plain("printf staged > \"$out/staged\""),
    );
    let disk_backed = disk_backed_directory(&fixture);
    let drill_ran = disk_backed.is_some();
    let disk_backed = disk_backed.unwrap_or_default();
    if !drill_ran {
        eprintln!(
            "no disk-backed directory was reachable, so the staging refusal was not \
             drilled here. It is drilled wherever one is, and `staging.rs` holds the \
             probe itself."
        );
    }
    if drill_ran {
        let refused = fixture
            .run_env(
                &["generate", "ana", "staged"],
                None,
                &[
                    ("SAFIX_STAGING_DIR", disk_backed.as_str()),
                    ("XDG_RUNTIME_DIR", ""),
                ],
            )
            .expect_refusal("staging into a disk-backed directory");
        refused.says("no memory-backed filesystem");
        refused.says("--allow-disk-staging");
        codes.push(
            fixture
                .run_graphical_env(
                    &["generate", "ana", "staged"],
                    &[
                        ("SAFIX_STAGING_DIR", disk_backed.as_str()),
                        ("XDG_RUNTIME_DIR", ""),
                    ],
                )
                .refusal_code(),
        );

        // And the acknowledgement is what makes it proceed, with the directory
        // still private and still swept.
        fixture
            .run_env(
                &["generate", "--allow-disk-staging", "ana", "staged"],
                None,
                &[
                    ("SAFIX_STAGING_DIR", disk_backed.as_str()),
                    ("XDG_RUNTIME_DIR", ""),
                ],
            )
            .expect_success("staging under the acknowledgement");
        assert_eq!(fixture.value(ANA_FILE, "staged"), "staged");
        assert!(
            Fixture::roots_in(std::path::Path::new(&disk_backed)).is_empty(),
            "an acknowledged disk-backed run left its staging root behind"
        );
    } else {
        codes.push("staging_not_memory_backed".to_owned());
    }

    assert_eq!(
        codes,
        vec![
            "no_generator",
            "generator_produced_nothing",
            "generator_failed",
            "validation_rejected",
            "generator_output_missing",
            "staging_not_memory_backed",
        ],
        "each refused generator has its own code"
    );

    // Not one of the refusals wrote, committed, or left anything behind. The
    // acknowledged run above is the one exception and is asserted separately, so
    // the head comparison is taken against the head it left.
    let head_before = if drill_ran {
        fixture.head()
    } else {
        head_before
    };
    assert_eq!(fixture.head(), head_before, "a refused generator committed");
    assert!(
        fixture.scratch_files().is_empty(),
        "a refused generator left a scratch file"
    );
    assert!(
        fixture
            .ciphertext_lines(ANA_FILE)
            .keys()
            .all(|key| key == "api-token" || key == "staged"),
        "a refused generator left keys in the file"
    );
}

/// A run order carrying a cycle is refused before any generator runs.
///
/// `resolve.nix` answers the graph question at evaluation and leaves the
/// generators inside a cycle out of the order, so this is a plan the nix half
/// does not emit. The stub emits it because a stand-in for nix and a program
/// embedding the library are exactly the two callers for which that refusal has
/// not already been thrown, and the plan is a value with public fields for both
/// of them.
///
/// What the assertions are about is when the refusal arrives rather than that it
/// does. A generator sits ahead of the cycle in the order, so a runtime that
/// walked the order and met the cycle where the first missing input surfaced
/// would have minted and committed that one first — and a committed value is a
/// distributed one, which is the reason the resolver put the question at
/// evaluation to begin with.
///
/// Naming a generator outside the cycle is refused too, which is the resolver's
/// treatment rather than a wider one: an evaluation with anything stuck is
/// refused whole rather than emitting an order for the rest.
#[test]
fn a_run_order_carrying_a_cycle_is_refused_before_anything_runs() {
    let mut fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token"]);

    fixture.seed_generator(
        "aside",
        ANA_FILE,
        &[],
        &plain("printf '%s' minted-aside > \"$out/aside\""),
    );
    for (name, reads) in [("front", "rear"), ("rear", "front")] {
        fixture.seed_generator(
            name,
            ANA_FILE,
            &[],
            &json!({
                "script": format!("cat \"$in/{reads}/{reads}\" > \"$out/{name}\""),
                "network": false,
                "runtimeInputs": ["coreutils"],
                "prompts": {}, "dependencies": [reads], "files": {},
                "share": false, "validation": null, "description": null,
            }),
        );
    }

    let head_before = fixture.head();
    let refused = fixture
        .run(&["generate", "ana"])
        .expect_refusal("a run order carrying a cycle");
    refused.says("carries a cycle of generators");
    refused.says("'front' -> 'rear' -> 'front'");
    assert_eq!(
        fixture.run_graphical(&["generate", "ana"]).refusal_code(),
        "generator_cycle"
    );

    fixture
        .run(&["generate", "ana", "aside"])
        .expect_refusal("a generator outside the cycle, under the same plan")
        .says("carries a cycle of generators");

    assert_eq!(
        fixture.head(),
        head_before,
        "the refused run committed something"
    );
    assert!(
        !fixture.ciphertext_lines(ANA_FILE).contains_key("aside"),
        "the generator ahead of the cycle in the order minted a value"
    );
    assert!(
        fixture.scratch_files().is_empty(),
        "the refused run left a scratch file"
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
    //
    // `$out` is the output directory now, so the probe's own accumulator is
    // named something else — a script that shadowed it would have nowhere to
    // write.
    let descriptors = "me=$$; seen=; for f in /proc/$me/fd/*; do n=${f##*/}; \
                       [ \"$n\" -ge 3 ] || continue; seen=\"$seen$n \"; done; \
                       printf %s \"${seen:-none}\"";

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
        &plain("cat >/dev/null; printf '%s' ate-nothing > \"$out/aaa-greedy\""),
    );
    // Two probes of one script, one before the generators that open descriptors
    // and one after. Comparing them rather than asserting a literal set is what
    // keeps the claim about leaked descriptors instead of about bash's own
    // bookkeeping, which contributes the same fds to both.
    fixture.seed_generator(
        "bbb-probe-first",
        ANA_FILE,
        &[],
        &plain(&format!("{descriptors} > \"$out/bbb-probe-first\"")),
    );
    // Two dependencies and a prompt, so three descriptors are open at once and
    // all three have to be closed before the next generator starts.
    fixture.seed_generator(
        "mmm-many",
        ANA_FILE,
        &[],
        &json!({
            "script": "cat \"$in/aaa-greedy/aaa-greedy\" >/dev/null; \
                       cat \"$in/api-token/api-token\" >/dev/null; \
                       cat \"$prompts/secret\" >/dev/null; \
                       printf '%s' many-ok > \"$out/mmm-many\"",
            "network": false,
            "runtimeInputs": ["coreutils"],
            "prompts": { "secret": { "type": "hidden", "description": "the fixture passphrase" } },
            "dependencies": ["aaa-greedy", "api-token"], "files": {},
            "share": false, "validation": null, "description": null,
        }),
    );
    fixture.seed_generator(
        "nnn-more",
        ANA_FILE,
        &[],
        &json!({
            "script": "cat \"$in/mmm-many/mmm-many\" >/dev/null; \
                       cat \"$in/bbb-probe-first/bbb-probe-first\" >/dev/null; \
                       printf '%s' more-ok > \"$out/nnn-more\"",
            "network": false,
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": ["mmm-many", "bbb-probe-first"], "files": {},
            "share": false, "validation": null, "description": null,
        }),
    );
    fixture.seed_generator(
        "zzz-probe-last",
        ANA_FILE,
        &[],
        &plain(&format!("{descriptors} > \"$out/zzz-probe-last\"")),
    );

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
        &plain("head -c 18 /dev/urandom | base64 | tr -d '\\n' > \"$out/base\""),
    );
    fixture.seed_generator(
        "middle",
        ANA_FILE,
        &[],
        &json!({
            "script": "printf 'mid-%s' \"$(cat \"$in/base/base\")\" > \"$out/middle\"",
            "network": false,
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": ["base"], "files": {},
            "share": false, "validation": null, "description": null,
        }),
    );
    fixture.seed_generator(
        "leaf",
        ANA_FILE,
        &[],
        &json!({
            "script": "printf 'leaf-%s' \"$(cat \"$in/middle/middle\")\" > \"$out/leaf\"",
            "network": false,
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": ["middle"], "files": {},
            "share": false, "validation": null, "description": null,
        }),
    );
    // Reads nothing of base's, so it is downstream of nothing and must be left
    // alone. Without it the cascade could be "re-run everything" and still pass.
    fixture.seed_generator(
        "aside",
        ANA_FILE,
        &[],
        &plain("printf '%s' untouched > \"$out/aside\""),
    );

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

/// What a mint records about its own declaration, and what `check` says when
/// that declaration changes underneath the value.
///
/// The record is the whole of what makes the drift detectable, so this test reads
/// it directly rather than only through the report: one line, a format tag and a
/// digest, none of the minted value anywhere in it, and unmoved by a rotation that
/// changes the value and not the declaration.
///
/// Then the four states the finding has to tell apart. A declaration edited after
/// the mint is reported, naming both remedies and no value. Regenerating clears
/// it. A hand-set entry never produces it, because nothing minted it. And a
/// generated value whose record is absent, or is in a format this version does not
/// write, produces nothing either — grandfathering is the claim, not an accident.
#[test]
fn a_definition_edited_after_a_mint_is_reported_and_a_regeneration_clears_it() {
    const RECORD: &str = "state/safix/definitions/ana/recorded";
    const MINTED: &str = "CANARY-minted-under-the-first-definition";

    let mut fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token"]);
    fixture.seed_generator(
        "recorded",
        ANA_FILE,
        &[],
        &plain(&format!("printf '%s' {MINTED} > \"$out/recorded\"")),
    );

    fixture
        .run(&["generate", "ana", "recorded"])
        .expect_success("the first mint");
    assert_eq!(fixture.value(ANA_FILE, "recorded"), MINTED);

    // One line: the format tag, a space, sixty-four hexadecimal digits, a
    // newline. Read as bytes, because what the record must not carry is anything
    // derived from the value.
    let recorded = fixture.read(RECORD);
    let (tag, hex) = recorded
        .trim_end_matches('\n')
        .split_once(' ')
        .expect("the record is a tag and a digest");
    assert_eq!(tag, "safix-definition-v1");
    assert_eq!(hex.len(), 64, "the digest is not a sha256: {hex}");
    assert!(
        hex.chars().all(|digit| digit.is_ascii_hexdigit()),
        "the digest is not hexadecimal: {hex}"
    );
    assert!(
        !recorded.contains(MINTED),
        "the record carries the value it was minted beside"
    );
    assert_eq!(
        recorded.lines().count(),
        1,
        "the record is more than one line: {recorded:?}"
    );

    // Two mints of different values under one declaration produce one record. A
    // value cannot reach the digest — it is computed from the generator record and
    // nothing else — and this is what says so from outside: `rolling` mints
    // something different every run, so a record carrying any function of the
    // value would move here while the declaration stood still.
    fixture.seed_generator(
        "rolling",
        ANA_FILE,
        &[],
        &plain("head -c 16 /dev/urandom | base64 | tr -d '\\n' > \"$out/rolling\""),
    );
    fixture
        .run(&["generate", "ana", "rolling"])
        .expect_success("the first mint of a rolling value");
    let first_value = fixture.value(ANA_FILE, "rolling");
    let first_record = fixture.read("state/safix/definitions/ana/rolling");
    fixture
        .run(&["generate", "--regenerate", "ana", "rolling"])
        .expect_success("rotating it under the same declaration");
    assert_ne!(
        fixture.value(ANA_FILE, "rolling"),
        first_value,
        "the rolling generator minted the same value twice, so the claim below is vacuous"
    );
    assert_eq!(
        fixture.read("state/safix/definitions/ana/rolling"),
        first_record,
        "the record moved when only the value did"
    );

    // Nothing has drifted yet, which is what makes the finding below a
    // consequence of the edit rather than of the record existing.
    fixture
        .run(&["check", "ana"])
        .silent_about("minted by the generator on");

    // The declaration changes and the value does not. Same outputs, same place in
    // the order, a different script — the edit that used to be invisible.
    fixture.edit_generator(
        "recorded",
        &plain("printf '%s' CANARY-a-different-definition > \"$out/recorded\""),
    );

    let drifted = fixture
        .run(&["check", "ana"])
        .expect_refusal("a check over a drifted definition");
    drifted.says("flake.safix.users.ana holds 'recorded', minted by the generator on 'recorded'");
    drifted.says(RECORD);
    drifted.says("safix generate --regenerate ana recorded");
    drifted.says("or adopt the value by reverting the edit");
    drifted.silent_about(MINTED);
    drifted.silent_about("CANARY-a-different-definition");
    drifted.silent_about(hex);

    // The value is still the one the first definition minted: a report writes
    // nothing.
    assert_eq!(fixture.value(ANA_FILE, "recorded"), MINTED);

    // Regenerating adopts the declaration, and refreshes the record in the same
    // commit as the value it now describes.
    fixture
        .run(&["generate", "--regenerate", "ana", "recorded"])
        .expect_success("regenerating under the current declaration");
    assert_eq!(
        fixture.value(ANA_FILE, "recorded"),
        "CANARY-a-different-definition"
    );
    let refreshed = fixture.read(RECORD);
    assert_ne!(refreshed, recorded, "the regeneration kept the old record");
    let commit = fixture.commit_matching("generate recorded");
    assert_eq!(
        fixture.paths_in(&commit),
        vec![ANA_FILE.to_owned(), RECORD.to_owned()],
        "the refreshed record did not ride the regeneration's commit"
    );
    fixture
        .run(&["check", "ana"])
        .silent_about("minted by the generator on");

    // A hand-set entry has no generator, so there is no definition it could have
    // drifted from. `api-token` holds a value from the fixture document.
    assert!(
        !fixture.exists("state/safix/definitions/ana/api-token"),
        "a hand-set entry acquired a definition record"
    );

    // A record this version cannot read says nothing, which is what keeps a change
    // to the canonical form from reporting every value in the tree as drifted.
    fixture.write(RECORD, "safix-definition-v2 0000\n");
    fixture
        .run(&["check", "ana"])
        .silent_about("minted by the generator on");

    // And a generated value with no record at all is grandfathered: it predates
    // the record, and asserting drift over an absent one would be a claim about
    // when the tool changed.
    std::fs::remove_file(fixture.repo.join(RECORD)).unwrap();
    fixture
        .run(&["check", "ana"])
        .silent_about("minted by the generator on");
}

/// clan's wireguard keypair, ported: one generator, a private half that is
/// encrypted and a public half stored in the clear.
///
/// This is the whole contract end to end. The script is clan's own two lines,
/// addressing `$out` the way clan's does, so what is being asserted is that a
/// generator written for clan runs here unchanged. The public half is then read
/// out of the repository the way a nix module reads `.value` — as bytes, with no
/// identity and no decryption — and matched against the public key `wg pubkey`
/// derives from the private half this run stored.
#[test]
fn a_wireguard_keypair_lands_encrypted_and_in_the_clear_in_one_commit() {
    const PUBLIC: &str = "public/safix/users/ana/wg-public/value";

    let mut fixture = Fixture::new();
    fixture.make_sops_file(ANA_FILE, &["api-token"]);

    // `wg` is not in the sandbox this suite runs in, and the shape under test is
    // the contract rather than curve25519: two outputs of one run, one encrypted
    // and one not, in one commit, with the public half a function of the private
    // one. So `wg pubkey` is stood in for by a derivation this test can state
    // independently — the shell computes it with `tr` and the assertion computes
    // it in rust — and the script still reads the private half back out of
    // `$out` exactly as clan'"'"'s `wg pubkey < "$out/privatekey"` does.
    fixture.seed_generator(
        "wg-private",
        ANA_FILE,
        &["wg-public"],
        &json!({
            "script": "head -c 32 /dev/urandom | base64 | tr -d '\\n' > \"$out/wg-private\"\n\
                       tr 'a-z' 'A-Z' < \"$out/wg-private\" > \"$out/wg-public\"",
            "network": false,
            "runtimeInputs": ["coreutils"],
            "prompts": {}, "dependencies": [], "files": { "wg-public": { "secret": false } },
            "share": false, "validation": null, "description": null,
        }),
    );
    fixture.seed_public_output("wg-public", PUBLIC);

    let before = fixture.head();
    fixture
        .run(&["generate", "ana", "wg-private"])
        .expect_success("the wireguard keypair");

    // The private half is ciphertext under a key, readable only through sops.
    let private = fixture.value(ANA_FILE, "wg-private");
    assert!(!private.is_empty(), "the private half stored nothing");

    // The public half is bytes in the repository. Reading it takes no identity,
    // which is what makes `.value` possible at evaluation: this read is the same
    // one `builtins.readFile` performs.
    let public = fixture.public_value(PUBLIC);
    assert!(!public.is_empty(), "the public half stored nothing");

    // And it is the public key of the private half this run stored, so the two
    // outputs came from one execution rather than from two.
    let expected = private.to_uppercase();
    assert_eq!(
        public, expected,
        "the public half is not derived from the private half that was stored"
    );

    // Nothing encrypted the public half. A sops document has a `sops:` metadata
    // block; this file is the value and nothing else.
    assert!(
        !public.contains("sops"),
        "the public output went through the encrypting backend"
    );

    // One commit, naming both paths: a keypair split across two commits is a
    // tree holding halves that do not match. The public half's record is in it
    // too — a public output is a generated value like any other, and the question
    // "was this minted under the declaration that is there now" is the same
    // question for it.
    assert_ne!(fixture.head(), before, "the keypair committed nothing");
    let commit = fixture.commit_matching("generate wg-private, wg-public");
    assert!(!commit.is_empty(), "no commit names both halves");
    let mut expected_paths = vec![
        ANA_FILE.to_owned(),
        PUBLIC.to_owned(),
        "state/safix/definitions/ana/wg-private".to_owned(),
        "state/safix/definitions/ana/wg-public".to_owned(),
    ];
    expected_paths.sort();
    assert_eq!(fixture.paths_in(&commit), expected_paths);

    // A re-run mints nothing: the public half holding a value is what says the
    // generator has already run, answered off the file the way the encrypted
    // half is answered off its ciphertext.
    let settled = fixture.head();
    fixture
        .run(&["generate", "ana", "wg-private"])
        .expect_success("a second run over a generated keypair");
    assert_eq!(fixture.head(), settled, "a second run rewrote the keypair");

    // --regenerate rotates both halves together.
    fixture
        .run(&["generate", "--regenerate", "ana", "wg-private"])
        .expect_success("rotating the keypair");
    let rotated_private = fixture.value(ANA_FILE, "wg-private");
    let rotated_public = fixture.public_value(PUBLIC);
    assert_ne!(
        rotated_private, private,
        "the rotation kept the private half"
    );
    assert_eq!(
        rotated_public,
        rotated_private.to_uppercase(),
        "the rotation left a public half derived from the retired private one"
    );

    // Editing a public output is refused rather than allowed to create a
    // document beside its own plaintext.
    fixture
        .run_env(&["edit", "ana", "wg-public"], None, &[("EDITOR", "true")])
        .expect_refusal("editing a public output")
        .says("is a public output");
}

/// A directory the kernel's mount table reports as disk-backed, if this machine
/// has one the suite can reach.
///
/// Asked of `/proc/mounts` through the harness rather than of
/// `staging::memory_backed`, and that is the whole point. Selecting the fixture
/// with the function under test made this drill unable to fail: a probe stuck at
/// "memory-backed" found no candidate, the drill reported itself skipped, and
/// the suite passed having asserted nothing about the rule. `memory_backing.rs`
/// holds the two readings against each other; this only borrows the independent
/// one.
///
/// The fixture's own disk-backed directory goes in front of the conventional
/// candidates, so the residue assertion below is about a directory nothing else
/// on this machine writes into.
fn disk_backed_directory(fixture: &Fixture) -> Option<String> {
    harness::disk_backed_directory(fixture.disk_staging_dir())
        .map(|path| path.to_string_lossy().into_owned())
}
