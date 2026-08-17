//! Every plaintext byte the runtime writes, observed at the system call.
//!
//! This is the retired `differential-strace` mode. `value_pipe.rs` shows the two
//! routes the value did *not* take; this shows the ones it did, and shows them
//! from outside the runtime entirely: `strace -y` annotates each descriptor with
//! what it resolves to, so a write of a plaintext value carries the answer to
//! "into what" beside it. A pipe is a descriptor with no name in the filesystem
//! and no bytes at rest.
//!
//! Two destinations are admissible rather than one, and the second is 0.2's
//! deliberate weakening. A typed value still travels a pipe end to end. A minted
//! value reaches files, because the interoperable generator contract addresses
//! its inputs and outputs by path — but only files inside that run's private
//! staging root, which is what `plaintext-staging` bounds and what this reading
//! holds it to. A write of a minted value to any other named file is what this
//! is looking for.
//!
//! Linux only, and absent rather than trivially green elsewhere: this needs
//! ptrace, and darwin's `dtruss` needs system integrity protection disabled,
//! which a build sandbox cannot do. The non-linux half of this file says what it
//! did not do rather than passing silently.
//!
//! The reading carries its own drill. A trace-scanning loop that quietly matches
//! nothing passes over everything, so a runtime that writes a plaintext value to
//! a regular file is put in the real one's place and has to be caught — and
//! caught by the pipe assertion rather than incidentally by the residue sweep,
//! which is why the file it plants is neither in the temporary directory nor
//! named like a candidate document.
//!
//! The same tool reads the envelope, and reads it as the other half of the same
//! question. Where a value went is one claim; where a fragment holding one could
//! not go is the other, and both are answered from outside the runtime rather
//! than by the runtime. A hostile fragment asks the kernel to open a file in the
//! repository, the trace carries the refusal, and the same trace shows an open
//! inside the staging root succeeding — so the refusal is the envelope's and not
//! a fragment that never tried or a sandbox that refuses everything.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

mod harness;

#[cfg(target_os = "linux")]
mod linux {
    use std::path::Path;

    use crate::harness::{ALICE_FILE, Fixture, Run, safix, shim};

    /// Distinctive enough that a match is this fixture's value rather than a
    /// coincidence in a store path, and short enough to survive strace's own
    /// truncation of the buffer it prints.
    const TYPED: &str = "CANARY-traced-value";

    /// The minted value, which the operator never types and which therefore
    /// travels only between the script, the runtime and sops.
    const MINTED: &str = "CANARY-minted-traced";

    /// The value the import leg carries, distinct from the export's so that
    /// neither leg's observation can be satisfied by the other's write.
    const IMPORTED: &str = "CANARY-imported-traced";

    /// The value the export leg carries.
    const EXPORTED: &str = "CANARY-exported-traced";

    /// The machine and var the traced mappings name.
    const BRIDGE_MACHINE: &str = "meridian";
    const BRIDGE_VAR: &str = "ntfy/token";

    /// A fixture carrying one mapping of the given direction.
    fn bridged(direction: &str) -> Fixture {
        let mut fixture = Fixture::new();
        fixture.seed_mapping(
            "ntfy-token",
            direction,
            (BRIDGE_MACHINE, "ntfy", "token"),
            ("alice", "api-token"),
        );
        fixture
    }

    /// A value arriving on standard input goes into sops down a pipe and into
    /// nothing else.
    #[test]
    fn every_plaintext_write_of_a_typed_value_goes_to_a_pipe() {
        let fixture = Fixture::new();

        let observed = trace(
            &fixture,
            safix(),
            &["set", "alice", "api-token"],
            Some(TYPED),
            &[],
            &[TYPED],
        )
        .unwrap_or_else(|reason| panic!("{reason}"));

        assert_eq!(
            fixture.value(ALICE_FILE, "api-token"),
            TYPED,
            "the traced run did not store the value, so the trace proves nothing"
        );
        assert!(
            observed > 0,
            "no plaintext write was observed at all, so the assertion is vacuous"
        );
    }

    /// A minted value reaches a pipe and the staging root, and nothing else.
    #[test]
    fn every_plaintext_write_of_a_minted_value_goes_to_a_pipe_or_the_staging_root() {
        let mut fixture = Fixture::new();
        fixture.seed_generator(
            "api-token",
            ALICE_FILE,
            &[],
            &serde_json::json!({
                "dependencies": [], "description": "a value minted from nothing",
                "files": {}, "prompts": {}, "share": false,
                "network": false,
                "runtimeInputs": [],
                "script": format!("printf '{MINTED}' > \"$out/api-token\""),
                "validation": null,
            }),
        );

        let observed = trace(
            &fixture,
            safix(),
            &["generate", "alice"],
            None,
            &[],
            &[MINTED],
        )
        .unwrap_or_else(|reason| panic!("{reason}"));

        assert_eq!(
            fixture.value(ALICE_FILE, "api-token"),
            MINTED,
            "the traced run did not store the minted value, so the trace proves nothing"
        );
        assert!(
            observed > 0,
            "no plaintext write was observed at all, so the assertion is vacuous"
        );
    }

    /// A bridged value crosses the clan boundary on a pipe, in both directions.
    ///
    /// The bridge is where the pipes-only reading is easiest to lose without
    /// noticing, because the value now crosses a boundary to a program this
    /// repository does not own. Both legs are traced in one reading rather than
    /// argued from the shape of the code: `clan vars get` writes the value to
    /// its standard output and `clan vars set` reads it from its standard input,
    /// and what this establishes is that between those two and sops the value
    /// touched a pipe and the run's own staging root and nothing else.
    ///
    /// The import leg is traced first and its result seeded into the export
    /// fixture, so the two legs carry different values: a single value would
    /// make "the export's value was observed" satisfiable by the import's write.
    #[test]
    fn every_plaintext_write_of_a_bridged_value_goes_to_a_pipe() {
        let down = bridged("clan-to-safix");
        down.clan_seed(BRIDGE_MACHINE, BRIDGE_VAR, IMPORTED);

        let mut environment = down.clan_env();
        let borrowed: Vec<(&str, &str)> = environment
            .iter_mut()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        let observed = trace(&down, safix(), &["import"], None, &borrowed, &[IMPORTED])
            .unwrap_or_else(|reason| panic!("{reason}"));

        assert_eq!(
            down.value(ALICE_FILE, "api-token"),
            IMPORTED,
            "the traced import did not store the value, so the trace proves nothing"
        );
        assert!(
            observed > 0,
            "no plaintext write was observed on the import leg, so the assertion is vacuous"
        );

        let up = bridged("safix-to-clan");
        up.set("alice", "api-token", EXPORTED)
            .expect_success("seeding the export's source");

        let mut environment = up.clan_env();
        let borrowed: Vec<(&str, &str)> = environment
            .iter_mut()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        let observed = trace(&up, safix(), &["export"], None, &borrowed, &[EXPORTED])
            .unwrap_or_else(|reason| panic!("{reason}"));

        assert_eq!(
            up.clan_holds(BRIDGE_MACHINE, BRIDGE_VAR).as_deref(),
            Some(EXPORTED),
            "the traced export did not reach clan, so the trace proves nothing"
        );
        assert!(
            observed > 0,
            "no plaintext write was observed on the export leg, so the assertion is vacuous"
        );
    }

    /// The pipe assertion shown to fail, on the mutation it exists to catch.
    ///
    /// A runtime that writes a plaintext value to a regular file is exactly what
    /// `-y` is there to see. The file is planted in the repository rather than
    /// in the temporary directory and is not named like a candidate document, so
    /// neither the residue sweep nor the scratch sweep can reach it: if the
    /// reading passes, nothing catches it.
    #[test]
    fn a_plaintext_write_to_a_regular_file_is_caught_by_the_pipe_assertion() {
        let fixture = Fixture::new();

        let outcome = trace(
            &fixture,
            shim(),
            &["set", "alice", "api-token"],
            Some(TYPED),
            &[
                ("SAFIX_SHIM_ROLE", "mutate"),
                ("SAFIX_SHIM_TARGET", safix()),
                ("SAFIX_SHIM_MUTATION", "plaintext"),
                ("SAFIX_SHIM_VALUE", TYPED),
            ],
            &[TYPED],
        );

        let reason = outcome.expect_err("a plaintext write to a regular file was not caught");
        assert!(
            reason.contains("neither a pipe"),
            "the drill was caught by something other than the pipe assertion: {reason}"
        );
        assert!(
            fixture.exists("a-plaintext-note"),
            "the mutation did not run, so the drill proves nothing"
        );
    }

    /// The envelope, read from outside the runtime at the system call.
    ///
    /// The readings above say where a plaintext value went. This says where one
    /// could not go, and says it from the same place and with the same tool: a
    /// hostile fragment asks the kernel to open a file in the repository, the
    /// kernel refuses, and the trace carries the refusal beside the path it was
    /// refused for. Nothing in the runtime is consulted for that answer — which is
    /// the point, since the runtime is what would be lying.
    ///
    /// `crates/safix/tests/sandbox.rs` makes the same claim from the other side by
    /// observing that no such file exists afterwards. Both are worth having: a
    /// file that is absent could be a fragment that never tried, and this shows
    /// the attempt as well as its refusal.
    ///
    /// The trace also has to be non-empty in the way that matters, so the fragment
    /// writes its output too: `$out` is inside the staging root and is bound
    /// read-write, so the same trace shows one open refused and one allowed. A
    /// sandbox that refused everything would fail on the second.
    #[test]
    fn the_envelope_refuses_a_fragments_open_outside_the_staging_root() {
        /// Where the hostile fragment tries to put the value, and a name no
        /// other fixture in this file writes.
        const ESCAPE: &str = "leaked-by-the-traced-fragment";

        let mut fixture = Fixture::new();
        let escape = fixture.repo.join(ESCAPE);
        fixture.seed_generator(
            "api-token",
            ALICE_FILE,
            &[],
            &serde_json::json!({
                "dependencies": [], "description": null,
                "files": {}, "prompts": {}, "share": false,
                "network": false,
                "runtimeInputs": [],
                "script": format!(
                    "printf '{MINTED}' > \"$out/api-token\"\n\
                     printf '{MINTED}' > \"$SAFIX_TEST_ESCAPE\""
                ),
                "validation": null,
            }),
        );

        let (run, text) = traced(
            &fixture,
            safix(),
            &["generate", "alice"],
            None,
            &[("SAFIX_TEST_ESCAPE", &escape.to_string_lossy())],
            "trace=open,openat,write",
        );
        assert!(
            !run.succeeded(),
            "a fragment that wrote outside its staging root was not refused\n{}",
            run.combined()
        );
        let text = text.unwrap_or_else(|reason| panic!("{reason}"));

        let opens = |needle: &str| -> Vec<&str> {
            text.lines()
                .filter(|line| line.contains("open") && line.contains(needle))
                .collect()
        };

        let escapes = opens(ESCAPE);
        assert!(
            !escapes.is_empty(),
            "the fragment's attempt to open a file outside its staging root was not \
             observed at all, so the reading is vacuous"
        );
        assert!(
            escapes.iter().all(|line| line.contains("= -1 ")),
            "an open outside the staging root succeeded: {escapes:?}"
        );

        let staged = opens(STAGING);
        assert!(
            staged.iter().any(|line| !line.contains("= -1 ")),
            "no open inside the staging root succeeded, so the refusal above says \
             nothing about the envelope and everything about the fragment"
        );
        assert!(
            !escape.exists(),
            "the refused open left a file in the repository"
        );
    }

    /// The prefix `staging.rs` names every staging root with.
    ///
    /// A literal rather than a read of the constant, so that renaming the
    /// directory scheme without revisiting this reading fails here: the point of
    /// the assertion is what an outside observer sees, and an outside observer
    /// has the name and not the constant.
    const STAGING: &str = "safix-stage-";

    /// Run one invocation under `strace` and hand back what it did and what the
    /// trace holds.
    ///
    /// The two readings in this file differ in which syscalls they ask for and in
    /// whether the run is expected to succeed, and share everything else.
    fn traced(
        fixture: &Fixture,
        program: &str,
        arguments: &[&str],
        stdin: Option<&str>,
        extra: &[(&str, &str)],
        syscalls: &str,
    ) -> (Run, Result<String, String>) {
        let log = fixture.scratch("trace");
        let mut traced = vec![
            "-f",
            "-y",
            "-s",
            "512",
            "-e",
            syscalls,
            "-o",
            log.to_str().unwrap(),
            program,
        ];
        traced.extend_from_slice(arguments);

        let run = fixture.run_program("strace", &traced, stdin, extra);
        let text = std::fs::read_to_string(&log)
            .map_err(|cause| format!("strace produced no trace: {cause}"));
        (run, text)
    }

    /// Run one invocation under `strace`, and hold every write carrying one of
    /// the values to being a write into a pipe or into the staging root.
    ///
    /// The reason comes back rather than being asserted here, so the drill can
    /// require that a failure is *this* assertion's failure rather than some
    /// other one further down.
    fn trace(
        fixture: &Fixture,
        program: &str,
        arguments: &[&str],
        stdin: Option<&str>,
        extra: &[(&str, &str)],
        values: &[&str],
    ) -> Result<usize, String> {
        let (run, text) = traced(fixture, program, arguments, stdin, extra, "trace=write");
        report(&run)?;
        let text = text?;

        let mut seen = 0;
        for line in text.lines() {
            if !line.contains("write(") || !values.iter().any(|value| line.contains(value)) {
                continue;
            }
            seen += 1;
            // Everything up to the first comma is the descriptor and the
            // resolution `-y` annotates it with; the buffer follows and may hold
            // commas of its own, so the split is deliberate.
            let descriptor = line.split_once(',').map_or(line, |(head, _)| head);
            if !descriptor.contains("<pipe:[") && !descriptor.contains(STAGING) {
                return Err(format!(
                    "a plaintext value was written to something that is neither a pipe \
                     nor a file in the run's staging root: {descriptor}"
                ));
            }
        }

        residue_free(fixture, values)?;
        Ok(seen)
    }

    /// A traced run that failed is a trace about nothing.
    fn report(run: &Run) -> Result<(), String> {
        if run.succeeded() {
            return Ok(());
        }
        Err(format!(
            "the traced run exited {:?}\n{}",
            run.code,
            run.combined()
        ))
    }

    /// No value left in the temporary directory the run staged in, nor in any
    /// staging root it made.
    ///
    /// The second half is what makes admitting a staging-root write above safe
    /// to admit: the trace shows plaintext going into a file, and this shows the
    /// file is gone by the time the run returned. Without it the reading would
    /// have been weakened to "a file somewhere" and nothing would hold the
    /// shred.
    fn residue_free(fixture: &Fixture, values: &[&str]) -> Result<(), String> {
        for value in values {
            if let Some(path) = find(&fixture.tmpdir(), value) {
                return Err(format!(
                    "a plaintext value was left in the temporary directory: {}",
                    path.display()
                ));
            }
        }
        for root in ["/dev/shm", "/run/user"] {
            for value in values {
                if let Some(path) = find_staged(Path::new(root), value) {
                    return Err(format!(
                        "a plaintext value was left in a staging root: {}",
                        path.display()
                    ));
                }
            }
        }
        Ok(())
    }

    /// The first file under a staging root anywhere below this directory holding
    /// this text.
    ///
    /// Scoped to directories named the way `staging.rs` names them, because
    /// `/dev/shm` is shared with everything else running as this user and a
    /// blind walk of it would report somebody else's file as this run's residue.
    fn find_staged(root: &Path, needle: &str) -> Option<std::path::PathBuf> {
        let entries = std::fs::read_dir(root).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let named = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(STAGING));
            let found = if named {
                find(&path, needle)
            } else {
                find_staged(&path, needle)
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// The first file under a directory holding this text.
    fn find(root: &Path, needle: &str) -> Option<std::path::PathBuf> {
        let entries = std::fs::read_dir(root).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let found = if path.is_dir() {
                find(&path, needle)
            } else {
                std::fs::read_to_string(&path)
                    .ok()
                    .filter(|text| text.contains(needle))
                    .map(|_| path.clone())
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }
}

/// What was not observed here, said out loud.
///
/// An attribute that is simply absent would be cleaner than a check that passes
/// having done nothing, but a test that quietly does not exist on a platform is
/// how a claim stops being made without anybody deciding to stop making it.
#[cfg(not(target_os = "linux"))]
#[test]
fn the_syscall_proof_needs_ptrace_and_was_not_made_here() {
    eprintln!(
        "the syscall proof needs ptrace, which is linux only; darwin's dtruss needs \
         system integrity protection disabled, which a build sandbox cannot do. \
         No write was observed on this platform, and neither was the envelope's \
         refusal of an open outside the staging root. Both claims are made on \
         linux; `value_pipe.rs` holds the two channels a value must not travel \
         down everywhere, and `crates/safix-core/src/sandbox.rs` unit-tests the \
         envelope this platform would be confined by."
    );
}
