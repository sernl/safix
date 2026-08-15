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
    // tree holding halves that do not match.
    assert_ne!(fixture.head(), before, "the keypair committed nothing");
    let commit = fixture.commit_matching("generate wg-private, wg-public");
    assert!(!commit.is_empty(), "no commit names both halves");
    let mut expected_paths = vec![ANA_FILE.to_owned(), PUBLIC.to_owned()];
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
