//! Declaring a person who holds nothing yet.
//!
//! `adduser` acts on none of the ciphertext the other tests build: it reads a
//! name alphabet and a hook, writes nix, and commits. What is asserted is what
//! an operator cannot check by reading the output — that the generated nix
//! parses, that the regenerated policy saw the scaffold, that the commit is the
//! scaffold and nothing else, that nothing was minted, and that every refusal
//! leaves the tree as it found it.
//!
//! The generated nix is parsed with the real `nix-instantiate`. A flake
//! evaluation is what a sandbox cannot do and the stub stands in for; parsing a
//! file needs no store and no daemon, so "the scaffold is valid nix" is a claim
//! made against the parser that will read it.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

mod harness;

use std::process::Command;

use harness::Fixture;

/// The scaffold, the policy regenerated from a tree that includes it, and the
/// two committed together — and nothing else.
#[test]
fn adduser_commits_the_scaffold_and_the_policy_that_saw_it() {
    let fixture = Fixture::new();
    fixture.seed_declarations();
    let recipient = fixture.new_recipient();

    // An unrelated staged change, to show the commit is scoped to the scaffold.
    fixture.write("bystander.txt", "bystander\n");
    fixture.git(&["add", "--", "bystander.txt"]);

    let run = fixture
        .run(&["adduser", "carol", &recipient, "--yes"])
        .expect_success("scaffolding a new person");

    let scaffold = fixture.read("safix/users/carol.nix");
    assert!(
        scaffold.contains(&format!("recipient = \"{recipient}\";")),
        "the recipient handed in is not the one recorded"
    );
    assert!(
        Command::new("nix-instantiate")
            .arg("--parse")
            .arg(fixture.repo.join("safix/users/carol.nix"))
            .output()
            .expect("could not run nix-instantiate")
            .status
            .success(),
        "the generated declaration does not parse"
    );
    assert!(scaffold.contains("carries = { };"), "the scaffold carries");
    assert!(scaffold.contains("private = { };"), "the scaffold declares");

    // The policy was regenerated from declarations that already contained
    // carol, which is only true if the scaffold was staged before the
    // evaluation: an evaluation reads the files git tracks, so regenerating
    // first writes the policy of the declarations as they stood a moment
    // earlier.
    let policy = fixture.read(".sops.yaml");
    assert!(
        policy.contains(&format!("- &carol {recipient}")),
        "the regenerated .sops.yaml does not carry the person just declared"
    );
    assert!(
        policy.contains("- &alice "),
        "the regenerated .sops.yaml dropped someone already declared"
    );
    // A person who holds nothing gets an anchor and no rule: a rule comes from a
    // declaration with a secret in it and from nothing else.
    assert!(
        !policy.contains("secrets/safix/users/carol/"),
        "a person who holds nothing produced a creation rule"
    );

    assert_eq!(
        fixture.paths_in("HEAD"),
        vec![".sops.yaml".to_owned(), "safix/users/carol.nix".to_owned()],
        "the commit is not exactly the scaffold and the regenerated policy"
    );
    assert_eq!(
        fixture.staged(),
        vec!["bystander.txt".to_owned()],
        "the bystander was swept into the commit"
    );

    // No key material anywhere: the recipient is public and is the only
    // key-shaped string the run may have written.
    assert!(
        fixture.holds_anywhere("AGE-SECRET-KEY").is_none(),
        "a private key reached the tree"
    );

    // The output says what it did and what it did not, and names the sequence
    // that gives them their first secret.
    run.says("safix/users/carol.nix");
    run.says("no key was minted");
    run.says("onboardingHook is unset");
    run.says("safix fix");
    run.says("safix set carol");

    let head = fixture.head();
    fixture
        .run(&["adduser", "carol", &recipient, "--yes"])
        .expect_refusal("scaffolding the same person twice");
    assert_eq!(fixture.head(), head, "the refusal to redeclare committed");
}

/// Every refusal about a name or a recipient, each leaving no scaffold, no
/// commit and no dirt.
#[test]
fn adduser_refusals_leave_the_tree_as_they_found_it() {
    let fixture = Fixture::new();
    fixture.seed_declarations();
    let recipient = fixture.new_recipient();
    let head = fixture.head();

    // A name outside the alphabet. The refusal has to happen here: the name is
    // not a declared user yet, so no resolver check can reach it, and the commit
    // that would make it reachable is the one being refused.
    let mut codes = Vec::new();
    for name in ["Carol", "carol/../root"] {
        fixture
            .run(&["adduser", name, &recipient, "--yes"])
            .expect_refusal("a name outside the alphabet");
        codes.push(
            fixture
                .run_graphical(&["adduser", name, &recipient, "--yes"])
                .refusal_code(),
        );
        assert_untouched(&fixture, &head, "a refused name");
    }
    // A name starting outside the alphabet is refused as an option rather than
    // as a name, which is a different refusal and stays one.
    fixture
        .run(&["adduser", "-carol", &recipient, "--yes"])
        .expect_refusal("a name starting outside the alphabet");

    // A recipient that is not one.
    for bad in ["not-an-age-key".to_owned(), format!("{recipient}extra")] {
        fixture
            .run(&["adduser", "carol", &bad, "--yes"])
            .expect_refusal("a malformed recipient");
        codes.push(
            fixture
                .run_graphical(&["adduser", "carol", &bad, "--yes"])
                .refusal_code(),
        );
        assert_untouched(&fixture, &head, "a malformed recipient");
    }

    // A hardware recipient, refused for what it cannot do rather than for its
    // shape: it is a well-formed recipient and activation still cannot use it.
    // Synthetic, and only the `age1yubikey1` prefix is load-bearing — the
    // refusal fires on that and never reaches the bech32 check, so no
    // plausible-looking suffix is needed and none is used.
    let card = "age1yubikey1fixture000000000000000000000000000000000000000000000000000";
    let refused = fixture
        .run(&["adduser", "carol", card, "--yes"])
        .expect_refusal("a recipient requiring a physical interaction");
    refused.says("recoveryRecipients");
    codes.push(
        fixture
            .run_graphical(&["adduser", "carol", card, "--yes"])
            .refusal_code(),
    );
    assert_untouched(&fixture, &head, "a hardware recipient");

    // An existing person.
    fixture
        .run(&["adduser", "alice", &recipient, "--yes"])
        .expect_refusal("redeclaring an existing person");
    codes.push(
        fixture
            .run_graphical(&["adduser", "alice", &recipient, "--yes"])
            .refusal_code(),
    );
    assert_untouched(&fixture, &head, "redeclaring an existing person");

    assert_eq!(
        codes,
        vec![
            "bad_user_name",
            "bad_user_name",
            "bad_recipient",
            "bad_recipient",
            "hardware_recipient",
            "already_declared",
        ],
        "a refusal about the name and one about the recipient are the same refusal"
    );
}

/// Host attachment reaches a consumer through the hook or not at all.
#[test]
fn host_attachment_is_refused_without_a_hook_and_handed_to_one_after_the_commit() {
    let fixture = Fixture::new();
    fixture.seed_declarations();
    let recipient = fixture.new_recipient();
    let head = fixture.head();

    let refused = fixture
        .run(&["adduser", "carol", &recipient, "--host", "somebox", "--yes"])
        .expect_refusal("--host with no hook configured");
    refused.says("onboardingHook is unset");
    refused.says("onboarding without it succeeds");
    assert_untouched(&fixture, &head, "--host with no hook configured");

    // A hook that records what it was handed. It writes into the repository
    // without staging anything, which is what lets the claim below distinguish
    // "ran after the commit" from "ran before it".
    fixture.set_hook(Some(
        "{\n  printf 'name=%s\\n' \"$1\"\n  printf 'recipient=%s\\n' \"$2\"\n  \
         shift 2\n  for host in \"$@\"; do printf 'host=%s\\n' \"$host\"; done\n\
         } >hook-log.txt\n",
    ));

    fixture
        .run(&[
            "adduser", "carol", &recipient, "--host", "somebox", "--host", "otherbox", "--yes",
        ])
        .expect_success("onboarding with a hook");

    let log = fixture.read("hook-log.txt");
    assert_eq!(
        log,
        format!("name=carol\nrecipient={recipient}\nhost=somebox\nhost=otherbox\n"),
        "the hook was not given the name, the recipient and every host"
    );

    // safix's commit is still exactly its own scaffolding, and the hook's output
    // is uncommitted: the package makes no assumption about what the hook does,
    // so it cannot claim its work in a message naming only what safix did.
    assert_eq!(
        fixture.paths_in("HEAD"),
        vec![".sops.yaml".to_owned(), "safix/users/carol.nix".to_owned()],
        "safix's commit carried the hook's work"
    );
    assert!(
        fixture.status().contains("hook-log.txt"),
        "the hook's output was not left uncommitted"
    );
}

/// No scaffold file and HEAD where it was. Every refusal owes this regardless of
/// which check it failed.
fn assert_untouched(fixture: &Fixture, head: &str, what: &str) {
    assert_eq!(fixture.head(), head, "{what} committed something");
    assert!(
        !fixture.exists("safix/users/carol.nix"),
        "{what} left a scaffold behind"
    );
    assert_eq!(fixture.status(), "", "{what} left the tree dirty");
}
