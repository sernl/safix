//! Every plaintext byte the runtime writes, observed at the system call.
//!
//! This is the retired `differential-strace` mode. `value_pipe.rs` shows the two
//! routes the value did *not* take; this shows the one it did, and shows it from
//! outside the runtime entirely: `strace -y` annotates each descriptor with what
//! it resolves to, so a write of a plaintext value carries the answer to "into
//! what" beside it. A pipe is a descriptor with no name in the filesystem and no
//! bytes at rest.
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

    use crate::harness::{ANA_FILE, Fixture, Run, SAFIX, SHIM};

    /// Distinctive enough that a match is this fixture's value rather than a
    /// coincidence in a store path, and short enough to survive strace's own
    /// truncation of the buffer it prints.
    const TYPED: &str = "CANARY-traced-value";

    /// The minted value, which the operator never types and which therefore
    /// travels only between the script, the runtime and sops.
    const MINTED: &str = "CANARY-minted-traced";

    /// A typed value goes into sops down a pipe and into nothing else.
    #[test]
    fn every_plaintext_write_of_a_typed_value_goes_to_a_pipe() {
        let fixture = Fixture::new();

        let observed = trace(
            &fixture,
            SAFIX,
            &["set", "ana", "api-token"],
            Some(&format!("{TYPED}\n{TYPED}\n")),
            &[],
            &[TYPED],
        )
        .unwrap_or_else(|reason| panic!("{reason}"));

        assert_eq!(
            fixture.value(ANA_FILE, "api-token"),
            TYPED,
            "the traced run did not store the value, so the trace proves nothing"
        );
        assert!(
            observed > 0,
            "no plaintext write was observed at all, so the assertion is vacuous"
        );
    }

    /// A minted value takes the same route.
    #[test]
    fn every_plaintext_write_of_a_minted_value_goes_to_a_pipe() {
        let mut fixture = Fixture::new();
        fixture.seed_generator(
            "api-token",
            ANA_FILE,
            &[],
            &serde_json::json!({
                "dependencies": [], "description": "a value minted from nothing",
                "files": [], "prompts": {},
                "runtimeInputs": [], "script": format!("printf '{MINTED}'"),
                "validation": null,
            }),
        );

        let observed = trace(&fixture, SAFIX, &["generate", "ana"], None, &[], &[MINTED])
            .unwrap_or_else(|reason| panic!("{reason}"));

        assert_eq!(
            fixture.value(ANA_FILE, "api-token"),
            MINTED,
            "the traced run did not store the minted value, so the trace proves nothing"
        );
        assert!(
            observed > 0,
            "no plaintext write was observed at all, so the assertion is vacuous"
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
            SHIM,
            &["set", "ana", "api-token"],
            Some(&format!("{TYPED}\n{TYPED}\n")),
            &[
                ("SAFIX_SHIM_ROLE", "mutate"),
                ("SAFIX_SHIM_TARGET", SAFIX),
                ("SAFIX_SHIM_MUTATION", "plaintext"),
                ("SAFIX_SHIM_VALUE", TYPED),
            ],
            &[TYPED],
        );

        let reason = outcome.expect_err("a plaintext write to a regular file was not caught");
        assert!(
            reason.contains("something other than a pipe"),
            "the drill was caught by something other than the pipe assertion: {reason}"
        );
        assert!(
            fixture.exists("a-plaintext-note"),
            "the mutation did not run, so the drill proves nothing"
        );
    }

    /// Run one invocation under `strace`, and hold every write carrying one of
    /// the values to being a write into a pipe.
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
        let log = fixture.scratch("trace");
        let mut traced = vec![
            "-f",
            "-y",
            "-s",
            "512",
            "-e",
            "trace=write",
            "-o",
            log.to_str().unwrap(),
            program,
        ];
        traced.extend_from_slice(arguments);

        let run = fixture.run_program("strace", &traced, stdin, extra);
        report(&run)?;

        let text = std::fs::read_to_string(&log)
            .map_err(|cause| format!("strace produced no trace: {cause}"))?;

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
            if !descriptor.contains("<pipe:[") {
                return Err(format!(
                    "a plaintext value was written to something other than a pipe: {descriptor}"
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

    /// No value left in the temporary directory the run staged in.
    fn residue_free(fixture: &Fixture, values: &[&str]) -> Result<(), String> {
        for value in values {
            if let Some(path) = find(&fixture.tmpdir(), value) {
                return Err(format!(
                    "a plaintext value was left in the temporary directory: {}",
                    path.display()
                ));
            }
        }
        Ok(())
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
         No write was observed on this platform. The claim is made on linux, and \
         `value_pipe.rs` holds the two channels a value must not travel down \
         everywhere."
    );
}
