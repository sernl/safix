//! The tmpfs verification, held against an oracle that is not itself.
//!
//! `plaintext-staging`'s central claim is that a run stages plaintext only on a
//! filesystem the kernel says keeps its pages in memory, and refuses when it
//! cannot find one. Everything else in that rule — the mode, the sweep, the
//! acknowledgement flag — is downstream of the probe answering correctly.
//!
//! Until this file the probe had no test that fails when it is defeated, and the
//! reason is worth stating because it is a shape that recurs. The drill that
//! exercises the refusal needs a disk-backed directory to point a run at, and it
//! *found* one by asking `staging::memory_backed` which of its candidates was
//! disk-backed. So a probe stuck at "memory-backed" made the search find
//! nothing; the drill then reported that it had been skipped, pushed the refusal
//! code it was supposed to have observed, and passed. The function under test
//! was choosing whether it would be tested.
//!
//! So the selection moves to an independent reading — `/proc/mounts`, the
//! kernel's own table, via `harness::kernel_says_memory_backed` — and the
//! runtime's `statfs` probe is held against it here. Two readings of one fact,
//! sharing no code: a probe that answers wrongly for any mount this machine has
//! now fails an assertion instead of quietly removing one.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use std::path::{Path, PathBuf};

use harness::{Fixture, disk_backed_directory, kernel_says_memory_backed};
use safix_core::staging;

/// Every mount this machine has, as paths the probe can be asked about.
///
/// Read from the mount table rather than guessed at, and filtered to the ones
/// this process can actually stat: a mount it cannot enter answers `None` from
/// both readings, which is agreement about nothing.
fn interrogable_mounts() -> Vec<PathBuf> {
    let Ok(mounts) = std::fs::read_to_string("/proc/mounts") else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = mounts
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(PathBuf::from)
        .filter(|path| std::fs::metadata(path).is_ok())
        .collect();
    found.sort();
    found.dedup();
    found
}

/// The runtime's `statfs` probe and the kernel's mount table agree, mount by
/// mount.
///
/// The severe half is the disk-backed direction. A probe that answered
/// "memory-backed" for everything — which is exactly what defeating the rule
/// looks like, and exactly what the old drill could not see — fails here on the
/// first mount the table calls `ext4`, `btrfs`, `xfs`, `overlay` or anything
/// else that is not one of the two memory filesystems.
///
/// Disagreement is asserted per mount and the mount is named, because "some
/// mount disagrees" is not a report anybody can act on.
#[test]
fn the_runtime_probe_agrees_with_the_kernels_own_mount_table() {
    let mounts = interrogable_mounts();
    assert!(
        !mounts.is_empty(),
        "no mount was readable from /proc/mounts, so this asserted nothing"
    );

    let mut compared = 0_usize;
    let mut memory = 0_usize;
    let mut disk = 0_usize;

    for mount in &mounts {
        let (Some(kernel), Some(probe)) = (
            kernel_says_memory_backed(mount),
            staging::memory_backed(mount),
        ) else {
            continue;
        };
        assert_eq!(
            probe,
            kernel,
            "the runtime's probe and /proc/mounts disagree about {}",
            mount.display()
        );
        compared = compared.saturating_add(1);
        if kernel {
            memory = memory.saturating_add(1);
        } else {
            disk = disk.saturating_add(1);
        }
    }

    assert!(
        compared > 0,
        "no mount could be read by both, so the two readings were never compared"
    );
    // Both directions have to be present or the agreement is one-sided: a probe
    // stuck at either answer agrees with a machine that only has mounts of that
    // kind, and this suite would have proved nothing about it.
    assert!(
        memory > 0,
        "no memory-backed mount was compared, so a probe stuck at 'disk-backed' would pass"
    );
    assert!(
        disk > 0,
        "no disk-backed mount was compared, so a probe stuck at 'memory-backed' would pass — \
         which is the exact defeat this file exists to catch"
    );
}

/// A run pointed at a directory the *kernel* calls disk-backed is refused.
///
/// The end-to-end half. The directory is chosen by the oracle above rather than
/// by the probe, so a defeated probe reaches this drill with a genuinely
/// disk-backed directory in hand and fails to refuse — where before it would
/// have arrived with nothing to point at and reported itself skipped.
#[test]
fn a_run_pointed_at_a_disk_backed_directory_refuses() {
    let mut fixture = Fixture::new();
    fixture.seed_generator(
        "staged",
        harness::ANA_FILE,
        &[],
        &serde_json::json!({
            "dependencies": [], "description": null,
            "files": {}, "prompts": {}, "share": false,
            "runtimeInputs": [],
            "script": "printf 'CANARY-never-staged' > \"$out/staged\"",
            "validation": null,
        }),
    );
    let Some(disk_backed) = disk_backed_directory(&[]) else {
        // A real state on a machine whose every mount is a tmpfs. Said out loud
        // rather than passed over, and the agreement test above still fails on
        // such a machine, so the probe is not left unasserted either way.
        eprintln!(
            "no disk-backed mount is reachable, so the end-to-end refusal was not drilled here"
        );
        return;
    };

    let refused = fixture
        .run_env(
            &["generate", "ana", "staged"],
            None,
            &[
                ("SAFIX_STAGING_DIR", &disk_backed.to_string_lossy()),
                ("XDG_RUNTIME_DIR", ""),
            ],
        )
        .expect_refusal("staging into a directory the kernel calls disk-backed");

    refused.says("no memory-backed filesystem");
    refused.says("--allow-disk-staging");
    refused.silent_about("CANARY-never-staged");
    assert!(
        Fixture::roots_in(Path::new(&disk_backed)).is_empty(),
        "the refused run made a staging root in the directory it refused to use"
    );
}

/// The refusal's own code, and that the acknowledgement is what gets past it.
///
/// Split from the test above because they are two claims: that the rule refuses,
/// and that the one documented way past it works and still sweeps.
#[test]
fn the_acknowledgement_is_the_only_way_past_the_refusal() {
    let mut fixture = Fixture::new();
    let Some(disk_backed) = disk_backed_directory(&[]) else {
        eprintln!("no disk-backed mount is reachable, so the acknowledgement was not drilled here");
        return;
    };
    let named = disk_backed.to_string_lossy().into_owned();

    fixture.seed_generator(
        "staged",
        harness::ANA_FILE,
        &[],
        &serde_json::json!({
            "dependencies": [], "description": null,
            "files": {}, "prompts": {}, "share": false,
            "runtimeInputs": [],
            "script": "printf 'CANARY-acknowledged' > \"$out/staged\"",
            "validation": null,
        }),
    );

    assert_eq!(
        fixture
            .run_graphical_env(
                &["generate", "ana", "staged"],
                &[("SAFIX_STAGING_DIR", &named), ("XDG_RUNTIME_DIR", "")],
            )
            .refusal_code(),
        "staging_not_memory_backed",
    );

    fixture
        .run_env(
            &["generate", "--allow-disk-staging", "ana", "staged"],
            None,
            &[("SAFIX_STAGING_DIR", &named), ("XDG_RUNTIME_DIR", "")],
        )
        .expect_success("staging under the acknowledgement");

    assert_eq!(
        fixture.value(harness::ANA_FILE, "staged"),
        "CANARY-acknowledged"
    );
    assert!(
        Fixture::roots_in(Path::new(&named)).is_empty(),
        "an acknowledged disk-backed run left its staging root behind in {named}"
    );
}
