//! The envelope, held to what it confines by fragments that try to leave it.
//!
//! `crates/safix-core/src/sandbox.rs` unit-tests the constructions — the
//! argument vector, the darwin profile, the probe's three answers — and those
//! tests run everywhere, because a string is a string on every platform. What
//! they cannot establish is that the confinement those strings describe is the
//! confinement the kernel applies. That is what this file is for, and it needs a
//! backend that actually runs.
//!
//! So the behavioural half is linux-only at compile time and gated at run time on
//! the backend running: bubblewrap needs user namespaces, which a kernel can
//! refuse, and a suite that asserted confinement it never established would be
//! worse than one that says it established nothing. Where the gate closes, each
//! test says what it did not do — the shape `syscall_proof.rs` and `real_clan.rs`
//! already use — and `safix-generate-envelope` reads that sentence out of the
//! output and fails, so a check cannot be green over a claim nobody made.
//!
//! # What "outside the staging root" means, exactly
//!
//! The envelope's root is a tmpfs, so a fragment writing to a path whose parent
//! exists only inside the envelope writes into that tmpfs and the bytes die with
//! the fragment. What is held here is the stronger and more useful reading: a
//! write to a path the *host* has — the repository, in these fixtures — fails,
//! and nothing the fragment produced reaches the host outside its staging root.
//! Each escape fixture is drilled against that: the same fragment run without the
//! envelope writes the file, which is what makes its absence afterwards a
//! statement about the envelope rather than about the fragment.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use harness::Fixture;

/// The flag clan offers and safix does not, refused as the unknown flag it is.
///
/// The spec's no-bypass scenario. `--no-sandbox` is the spelling an operator who
/// knows clan will reach for, and there is deliberately nothing it or anything
/// like it can be spelled as: the refusal is the usage line, not a weaker run.
///
/// Every platform, because it is the argument reader that refuses and no fragment
/// runs to be confined.
#[test]
fn no_flag_suspends_the_envelope() {
    let fixture = Fixture::new();

    for flag in ["--no-sandbox", "--sandbox=off", "--unsafe-no-sandbox"] {
        let run = fixture
            .run(&["generate", flag, "ana"])
            .expect_refusal(&format!("generate {flag}"));
        run.says("usage: safix generate");
        run.silent_about("no-such-secret");
    }

    // The flags this verb does take are still taken, so the refusal above is the
    // reader rejecting what it does not know rather than rejecting everything
    // that starts with two dashes.
    fixture
        .run(&["generate", "--regenerate", "--yes", "ana"])
        .expect_success("generate with the flags it does take");
}

#[cfg(target_os = "linux")]
mod linux {
    use std::io::Read as _;
    use std::net::TcpListener;
    use std::process::{Command, Stdio};

    use serde_json::{Value, json};

    use crate::harness::{ANA_FILE, Fixture};

    /// The value a fragment mints when it gets as far as minting one.
    const MINTED: &str = "CANARY-minted-inside";

    /// What a fragment tries to put where safix cannot shred it.
    const LEAKED: &str = "CANARY-outside-the-envelope";

    /// The repository-relative path every escape fixture writes to.
    ///
    /// In the repository rather than in a temporary directory, because that is the
    /// escape that matters: a fragment writing plaintext beside the ciphertext
    /// puts it in git, and a value committed is a value distributed. Its parent
    /// exists on the host and is not bound into the envelope, so the write fails
    /// there rather than landing in the envelope's own tmpfs.
    const ESCAPE: &str = "leaked-plaintext";

    /// A generator record, with the grant and the script the caller wants.
    fn generator(script: &str, network: bool) -> Value {
        json!({
            "dependencies": [], "description": null,
            "files": {}, "prompts": {}, "share": false,
            "network": network,
            "runtimeInputs": ["coreutils"],
            "script": script,
            "validation": null,
        })
    }

    /// Whether this kernel grants the namespaces the envelope is made of.
    ///
    /// Asked by running the backend against a literal argument vector written
    /// here, not by asking the runtime and not by calling the construction under
    /// test: a gate that consulted the code under test would skip this whole file
    /// on exactly the defect it exists to catch. This is the same independence
    /// `harness::mounted_filesystem` exists for.
    fn backend_runs() -> bool {
        Command::new("bwrap")
            .args([
                "--unshare-all",
                "--tmpfs",
                "/",
                "--ro-bind",
                "/nix/store",
                "/nix/store",
                "--dev",
                "/dev",
                "--bind",
                "/proc",
                "/proc",
                "--chdir",
                "/",
                "--",
                "bash",
                "-c",
                ":",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    /// The sentence a check reads to find out that nothing was established.
    fn unestablished(what: &str) {
        eprintln!(
            "no sandbox backend runs here, so {what} was not established. bubblewrap \
             needs user namespaces, which this kernel did not grant. The envelope's \
             constructions are unit-tested in safix-core either way."
        );
    }

    /// The same fragment, run with nothing confining it.
    ///
    /// The oracle that makes an escape fixture's failure mean something: without
    /// the envelope this write lands, so the file's absence after a confined run
    /// is the envelope's doing and not the fragment's. The file is removed again
    /// before the confined run, so what the run is judged against is a repository
    /// the drill left as it found it.
    fn escapes_unconfined(fixture: &Fixture, script: &str) -> bool {
        let escape = fixture.repo.join(ESCAPE);
        let staging = fixture.scratch("unconfined");
        std::fs::create_dir_all(staging.join("out")).unwrap();

        let status = Command::new("bash")
            .arg("-euo")
            .arg("pipefail")
            .arg("-c")
            .arg(script)
            .current_dir(&staging)
            .env("out", staging.join("out"))
            .env("SAFIX_TEST_ESCAPE", &escape)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        let landed = escape.exists();
        let _ = std::fs::remove_file(&escape);
        let _ = std::fs::remove_dir_all(&staging);
        // The unconfined run is expected to write the file; whether it then went
        // on to succeed is not the question, because a fragment reaching the
        // network is expected to fail here too.
        let _ = status;
        landed
    }

    /// A fragment that writes outside its staging root fails there, and the run
    /// refuses with that fragment's own failure having stored nothing.
    ///
    /// The requirement's first scenario. The escape is attempted before the
    /// output is written, so a run that stored anything would have stored a value
    /// from a fragment whose escape had already happened.
    #[test]
    fn a_write_outside_the_staging_root_fails_and_the_run_refuses() {
        if !backend_runs() {
            unestablished("the filesystem confinement");
            return;
        }

        let mut fixture = Fixture::new();
        fixture.make_sops_file(ANA_FILE, &["api-token"]);
        let script = format!(
            "printf '{LEAKED}' > \"$SAFIX_TEST_ESCAPE\"\n\
             printf '{MINTED}' > \"$out/hostile\""
        );

        assert!(
            escapes_unconfined(&fixture, &script),
            "the fragment does not write outside its staging root even unconfined, \
             so the confined run establishes nothing"
        );

        fixture.seed_generator("hostile", ANA_FILE, &[], &generator(&script, false));
        let before = fixture.head();
        let escape = fixture.repo.join(ESCAPE);

        let run = fixture
            .run_graphical_env(
                &["generate", "ana", "hostile"],
                &[("SAFIX_TEST_ESCAPE", &escape.to_string_lossy())],
            )
            .expect_refusal("a fragment writing outside its staging root");

        assert_eq!(
            run.refusal_code(),
            "generator_failed",
            "the run refused for some reason other than the fragment's own failure"
        );
        assert!(
            !escape.exists(),
            "the fragment wrote plaintext outside its staging root"
        );
        assert!(
            !fixture.read(ANA_FILE).contains("hostile"),
            "a value was stored for a generator whose fragment escaped"
        );
        assert_eq!(fixture.head(), before, "the refused run committed");
        assert_eq!(fixture.status(), "", "the refused run left the tree dirty");
        assert!(
            fixture.holds_anywhere(LEAKED).is_none(),
            "the value the fragment tried to leak is somewhere on this machine"
        );
    }

    /// A fragment with no grant cannot reach a listener on the machine running it.
    ///
    /// The listener is this process's, on loopback, and it is what makes the
    /// claim a statement about reachability rather than about DNS or about a
    /// network this machine may not have. Inside the envelope the fragment has a
    /// network namespace of its own holding nothing but its own loopback, so the
    /// connection is refused by a kernel that has no listener to offer — and the
    /// listener here accepts nothing, which is the half a fragment cannot fake.
    ///
    /// `/dev/tcp` rather than a tool, because it is bash's own and needs nothing
    /// resolved onto a path.
    #[test]
    fn a_fragment_without_the_grant_cannot_reach_the_network() {
        if !backend_runs() {
            unestablished("the network's absence");
            return;
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let mut fixture = Fixture::new();
        fixture.make_sops_file(ANA_FILE, &["api-token"]);
        fixture.seed_generator(
            "reaching",
            ANA_FILE,
            &[],
            &generator(
                &format!(
                    "exec 3<>/dev/tcp/127.0.0.1/{port}\n\
                     printf '{LEAKED}' >&3\n\
                     printf '{MINTED}' > \"$out/reaching\""
                ),
                false,
            ),
        );
        let before = fixture.head();

        let run = fixture
            .run_graphical(&["generate", "ana", "reaching"])
            .expect_refusal("a fragment reaching the network with no grant");
        assert_eq!(
            run.refusal_code(),
            "generator_failed",
            "the run refused for some reason other than the fragment's own failure"
        );

        listener.set_nonblocking(true).unwrap();
        assert!(
            listener.accept().is_err(),
            "a fragment with no grant reached a listener on this machine"
        );
        assert_eq!(fixture.head(), before, "the refused run committed");
    }

    /// The declared escape opens the network and nothing else.
    ///
    /// Both halves in one fragment, because the claim is that the grant is one
    /// capability rather than a loosening: the connection reaches this process's
    /// listener, carrying bytes it can read back, while the same write to the
    /// repository that the fixture above proved fails still fails.
    #[test]
    fn the_grant_opens_the_network_and_leaves_the_filesystem_confined() {
        if !backend_runs() {
            unestablished("the declared escape");
            return;
        }

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let mut fixture = Fixture::new();
        fixture.make_sops_file(ANA_FILE, &["api-token"]);
        let script = format!(
            "if printf '{LEAKED}' > \"$SAFIX_TEST_ESCAPE\" 2>/dev/null; then\n\
               printf 'escape=landed\\n' >&2\n\
             else\n\
               printf 'escape=refused\\n' >&2\n\
             fi\n\
             exec 3<>/dev/tcp/127.0.0.1/{port}\n\
             printf '{MINTED}' >&3\n\
             exec 3>&-\n\
             printf '{MINTED}' > \"$out/granted\""
        );

        assert!(
            escapes_unconfined(&fixture, &script),
            "the fragment does not write outside its staging root even unconfined, \
             so the confined run establishes nothing"
        );

        fixture.seed_generator("granted", ANA_FILE, &[], &generator(&script, true));
        let escape = fixture.repo.join(ESCAPE);

        let run = fixture
            .run_env(
                &["generate", "ana", "granted"],
                None,
                &[("SAFIX_TEST_ESCAPE", &escape.to_string_lossy())],
            )
            .expect_success("a generator whose declaration grants the network");

        // The connection was made while the fragment ran; the kernel held it in
        // the backlog, so accepting it afterwards reads what crossed.
        listener.set_nonblocking(false).unwrap();
        let (mut stream, _) = listener
            .accept()
            .expect("the granted fragment reached no listener");
        let mut received = String::new();
        stream.read_to_string(&mut received).unwrap();
        assert_eq!(
            received, MINTED,
            "the granted connection carried something other than what the fragment sent"
        );

        run.says("escape=refused");
        run.silent_about("escape=landed");
        assert!(
            !escape.exists(),
            "the grant reopened the filesystem it is documented not to touch"
        );
        assert_eq!(
            fixture.value(ANA_FILE, "granted"),
            MINTED,
            "the granted generator stored no value, so its run proves nothing"
        );
    }

    /// A backend the toolset cannot resolve refuses the run before any fragment.
    ///
    /// The refusal's code is the ordering claim. With the backend withheld, a
    /// fragment spawn would fail too — it resolves the same attribute — and would
    /// be reported as the generator failing. Reading `sandbox_unavailable` here
    /// therefore says the probe answered before anything was spawned, which is
    /// what "refuses before any fragment runs" means.
    #[test]
    fn a_withheld_backend_refuses_the_run_before_the_first_fragment() {
        let mut fixture = Fixture::new();
        fixture.make_sops_file(ANA_FILE, &["api-token"]);
        fixture.seed_generator(
            "unreachable",
            ANA_FILE,
            &[],
            &generator(
                &format!("printf 'fragment-ran\\n' >&2; printf '{MINTED}' > \"$out/unreachable\""),
                false,
            ),
        );
        let before = fixture.head();

        let run = fixture
            .run_graphical_env(
                &["generate", "ana", "unreachable"],
                &[("SAFIX_TEST_UNRESOLVABLE", "bubblewrap")],
            )
            .expect_refusal("generation with the backend withheld");

        assert_eq!(
            run.refusal_code(),
            "sandbox_unavailable",
            "the run refused for some reason other than the missing backend"
        );
        run.says("bwrap");
        run.says("There is no flag that runs a generator outside the envelope.");
        run.silent_about("fragment-ran");
        assert_eq!(fixture.head(), before, "the refused run committed");
        assert!(
            fixture.staging_roots().is_empty(),
            "a run refused before its first fragment established a staging root"
        );
    }
}

/// What was not observed here, said out loud.
///
/// An absent attribute would be tidier than a check that passes having done
/// nothing, but a test that quietly does not exist on a platform is how a claim
/// stops being made without anybody deciding to stop making it.
#[cfg(not(target_os = "linux"))]
#[test]
fn the_envelopes_confinement_was_not_observed_on_this_platform() {
    eprintln!(
        "the behavioural half of the envelope suite drives bubblewrap, which is linux \
         only. darwin's envelope is `sandbox-exec` with a profile this repository \
         constructs and unit-tests, and observing it needs a darwin machine rather \
         than a build sandbox. No confinement was observed on this platform."
    );
}
