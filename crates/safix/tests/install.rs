//! `safix install`, driven as an activation drives it.
//!
//! The verb no operator types, so the suite is what types it. Every run here
//! is user-mode or a check mode: a system-scope install mounts a filesystem
//! and chowns to a group, and neither is something a test process may do — the
//! booted-host claim is the VM test's, and what is left to hold here is the
//! part a user scope exercises in full. That is not a narrowing of the claim:
//! the user-mode path runs the same sequence with the three privileged steps
//! omitted by the manifest's own field, so the expansion, the decryption, the
//! generation rotation, the write modes, the symlinks and the dry run are all
//! measured against the real binary and the real sops.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use harness::{Fixture, mint_identity, recipient_of};
use serde_json::{Value, json};

/// The document the fixture entries are read out of.
const DOCUMENT: &str = "secrets/safix/users/alice/install.yaml";

/// One store, its identity, and the runtime directory a user-mode install
/// expands `%r` against.
struct Store {
    fixture: Fixture,
    runtime: PathBuf,
    identity: PathBuf,
}

impl Store {
    /// A fixture holding one document of two entries, encrypted to an identity
    /// this test minted and nothing else holds.
    fn new(name: &str) -> Self {
        let fixture = Fixture::new();
        let identity = fixture.tmpdir().join(format!("{name}-identity.txt"));
        mint_identity(&identity);
        let recipient = recipient_of(&identity);
        fixture.encrypt_to(
            DOCUMENT,
            &[&recipient],
            "alice-alone: a value\nservices:\n  nginx:\n    token: deep\n",
        );
        let runtime = fixture.tmpdir().join(format!("{name}-runtime"));
        std::fs::create_dir_all(&runtime).unwrap();
        Self {
            fixture,
            runtime,
            identity,
        }
    }

    /// The manifest a user-scope profile would build, as JSON.
    fn manifest(&self) -> Value {
        json!({
            "version": 1,
            "secrets": [
                self.entry("alice-alone", "alice-alone", "%r/safix/alice-alone", "0400"),
                self.entry("team-token", "services/nginx/token", "%r/elsewhere/token", "0440"),
            ],
            "secretsMountPoint": "%r/safix.d",
            "symlinkPath": "%r/safix",
            "keepGenerations": 1,
            "ageKeyFile": self.identity.display().to_string(),
            "ageSshKeyPaths": [],
            "useTmpfs": false,
            "userMode": true,
            "logging": { "keyImport": false, "secretChanges": true },
            "manifestInputHash": null
        })
    }

    fn entry(&self, name: &str, key: &str, path: &str, mode: &str) -> Value {
        json!({
            "name": name,
            "key": key,
            "path": path,
            "owner": null,
            "group": null,
            "uid": 0,
            "gid": 0,
            "sopsFile": self.fixture.repo.join(DOCUMENT).display().to_string(),
            "format": "yaml",
            "mode": mode,
            "restartUnits": [],
            "reloadUnits": []
        })
    }

    /// One manifest written where the run can read it.
    fn write(&self, name: &str, manifest: &Value) -> PathBuf {
        let path = self.fixture.tmpdir().join(format!("{name}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(manifest).unwrap()).unwrap();
        path
    }

    /// One `safix install` run, with the runtime directory and an identity in
    /// its environment.
    ///
    /// An installing run assembles its own identity from the manifest and
    /// reads no such variable; the check modes decrypt nothing and need none
    /// either. It is set because the fixture's own environment names the
    /// fixture's key, and a run here should be reading this store's.
    fn install(&self, arguments: &[&str]) -> harness::Run {
        let runtime = self.runtime.display().to_string();
        let identity = self.identity.display().to_string();
        self.fixture.run_env(
            arguments,
            None,
            &[
                ("XDG_RUNTIME_DIR", &runtime),
                ("SOPS_AGE_KEY_FILE", &identity),
            ],
        )
    }

    fn store_root(&self) -> PathBuf {
        self.runtime.join("safix")
    }
}

/// What a path's mode is, without its file type.
fn mode_of(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

/// Which filesystem a path is on, which is how "nothing was mounted here"
/// is stated without reading the mount table.
fn device_of(path: &Path) -> u64 {
    std::fs::metadata(path).unwrap().dev()
}

#[test]
fn stored_binary_and_intentional_empty_values_install_exactly() {
    let store = Store::new("binary");
    let recipient = recipient_of(&store.identity);
    store.fixture.encrypt_to(
        DOCUMENT,
        &[&recipient],
        "binary:\n  __safix_bytes_v1: [0, 255, 254, 65, 10]\nempty:\n  __safix_bytes_v1: []\n",
    );
    let mut manifest = store.manifest();
    manifest["secrets"] = json!([
        store.entry("binary", "binary", "%r/safix/binary", "0400"),
        store.entry("empty", "empty", "%r/safix/empty", "0400"),
    ]);
    let path = store.write("binary", &manifest);
    store
        .install(&["install", "--check-mode=document", path.to_str().unwrap()])
        .expect_success("binary storage is a value, not a nested selection");
    store
        .install(&["install", path.to_str().unwrap()])
        .expect_success("install exact binary and empty values");
    assert_eq!(
        std::fs::read(store.store_root().join("binary")).unwrap(),
        b"\0\xff\xfeA\n"
    );
    assert_eq!(
        std::fs::read(store.store_root().join("empty")).unwrap(),
        b""
    );
    assert_eq!(mode_of(&store.store_root().join("binary")), 0o400);
}

#[test]
fn native_rsa_identity_installs_without_leaking_failed_conversion_input() {
    let store = Store::new("native-rsa");
    let private = store.fixture.tmpdir().join("native-rsa");
    let generated = std::process::Command::new("ssh-keygen")
        .args(["-q", "-t", "rsa", "-b", "2048", "-N", "", "-f"])
        .arg(&private)
        .output()
        .unwrap();
    assert!(generated.status.success());
    let public = std::fs::read_to_string(private.with_extension("pub")).unwrap();
    let ciphertext = store.fixture.tmpdir().join("native-rsa.age");
    let mut encrypt = std::process::Command::new("age")
        .arg("-r")
        .arg(public.trim())
        .arg("-o")
        .arg(&ciphertext)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(&mut encrypt.stdin.take().unwrap(), b"\0\xff\xfeA\n").unwrap();
    assert!(encrypt.wait_with_output().unwrap().status.success());
    let mut manifest = store.manifest();
    manifest["ageKeyFile"] = serde_json::Value::Null;
    manifest["ageSshKeyPaths"] = json!([private]);
    let mut entry = store.entry("rsa", "", "%r/safix/rsa", "0400");
    entry["format"] = json!("age");
    entry["sopsFile"] = json!(ciphertext);
    manifest["secrets"] = json!([entry]);
    let path = store.write("native-rsa", &manifest);
    let installed = store.install(&["install", path.to_str().unwrap()]);
    assert!(!installed.combined().contains("BEGIN OPENSSH PRIVATE KEY"));
    installed.expect_success("native SSH identities do not require a converted age identity");
    assert_eq!(
        std::fs::read(store.store_root().join("rsa")).unwrap(),
        b"\0\xff\xfeA\n"
    );
}

#[test]
fn publication_refuses_store_ancestor_destinations_without_deleting_data() {
    let store = Store::new("ancestor");
    let sentinel = store.runtime.join("unrelated-data");
    std::fs::write(&sentinel, b"must survive").unwrap();
    let mut manifest = store.manifest();
    manifest["secrets"] = json!([store.entry(
        "token",
        "alice-alone",
        store.runtime.to_str().unwrap(),
        "0400",
    )]);
    let path = store.write("ancestor", &manifest);
    store
        .install(&["install", path.to_str().unwrap()])
        .expect_refusal("an output cannot replace an ancestor of either store root");
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"must survive");
    assert!(!store.store_root().exists());
}

#[test]
fn publication_failure_preserves_the_previous_generation() {
    let store = Store::new("publication");
    let path = store.write("first", &store.manifest());
    store
        .install(&["install", path.to_str().unwrap()])
        .expect_success("initial generation");
    let original = std::fs::read_link(store.store_root()).unwrap();
    let blocker = store.runtime.join("not-a-directory");
    std::fs::write(&blocker, b"must survive").unwrap();
    let recipient = recipient_of(&store.identity);
    store
        .fixture
        .encrypt_to(DOCUMENT, &[&recipient], "alice-alone: replacement\n");
    let mut manifest = store.manifest();
    manifest["secrets"] = json!([store.entry(
        "alice-alone",
        "alice-alone",
        blocker.join("token").to_str().unwrap(),
        "0400",
    )]);
    let path = store.write("blocked", &manifest);
    store
        .install(&["install", path.to_str().unwrap()])
        .expect_refusal("an unpublishable external path must not promote new values");
    assert_eq!(std::fs::read_link(store.store_root()).unwrap(), original);
    assert_eq!(
        std::fs::read(store.store_root().join("alice-alone")).unwrap(),
        b"a value"
    );
    assert_eq!(std::fs::read(&blocker).unwrap(), b"must survive");
}

#[test]
fn a_manifest_is_accepted_under_the_schema_mode_without_reading_any_ciphertext() {
    let store = Store::new("schema");
    let mut manifest = store.manifest();
    // A document that is not there at all: the schema mode reads none, so its
    // acceptance is evidence that it read none rather than that it happened to
    // find them.
    manifest["secrets"][0]["sopsFile"] = json!("/nowhere/at/all.yaml");
    manifest["secrets"][1]["sopsFile"] = json!("/nowhere/at/all.yaml");
    let path = store.write("schema", &manifest);

    store
        .install(&["install", "--check-mode=manifest", path.to_str().unwrap()])
        .expect_success("a well-formed manifest is accepted under the schema mode");
}

#[test]
fn the_document_mode_refuses_what_the_schema_mode_accepts() {
    let store = Store::new("tiers");
    let mut manifest = store.manifest();
    manifest["secrets"][0]["key"] = json!("services/nginx/absent");
    let path = store.write("tiers", &manifest);

    store
        .install(&["install", "--check-mode=manifest", path.to_str().unwrap()])
        .expect_success("a key absent from its document is not a schema fault");

    store
        .install(&["install", "--check-mode=document", path.to_str().unwrap()])
        .expect_refusal("the document mode resolves every declared key")
        .says("services/nginx/absent");
}

#[test]
fn the_document_mode_needs_no_identity_and_no_sops() {
    let store = Store::new("cleartext");
    let path = store.write("cleartext", &store.manifest());

    // A sops that refuses to be anything: if the document mode reached a
    // subprocess at all, this run would fail. The identity is pointed at a
    // path that does not exist for the same reason — the mode reads the
    // document's cleartext structure, where the key names live.
    let refusing = store.fixture.tmpdir().join("no-sops");
    std::fs::write(
        &refusing,
        "#!/bin/sh\necho 'sops was invoked' >&2\nexit 97\n",
    )
    .unwrap();
    std::fs::set_permissions(&refusing, std::fs::Permissions::from_mode(0o755)).unwrap();

    let runtime = store.runtime.display().to_string();
    let sops = refusing.display().to_string();
    let nowhere = store
        .fixture
        .tmpdir()
        .join("no-identity")
        .display()
        .to_string();
    store
        .fixture
        .run_env(
            &["install", "--check-mode=document", path.to_str().unwrap()],
            None,
            &[
                ("XDG_RUNTIME_DIR", &runtime),
                ("SAFIX_SOPS", &sops),
                ("SOPS_AGE_KEY_FILE", &nowhere),
            ],
        )
        .expect_success("the document mode reads cleartext structure and decrypts nothing")
        .silent_about("sops was invoked");
}

#[test]
fn a_user_mode_manifest_checks_with_no_runtime_directory_in_the_environment() {
    let store = Store::new("sandboxed");
    let path = store.write("sandboxed", &store.manifest());

    // What the manifest derivation's check phase is: a build with no session
    // and therefore no runtime directory. An empty `XDG_RUNTIME_DIR` is how
    // that is reachable from here, since a test process cannot unset a
    // variable in its own environment, and it takes the same path an unset
    // one does. Both modes must validate the manifest as written: `%r` is
    // expanded by the installation and by nothing else.
    let runtime = store.runtime.display().to_string();
    for mode in ["--check-mode=manifest", "--check-mode=document"] {
        store
            .fixture
            .run_env(
                &["install", mode, path.to_str().unwrap()],
                None,
                &[("XDG_RUNTIME_DIR", "")],
            )
            .expect_success("a check mode expands nothing and needs no runtime directory");
    }

    // And the installing path still does expand it, so the omission above is
    // the check modes' and not a capability that quietly went missing.
    store
        .fixture
        .run_env(
            &["install", path.to_str().unwrap()],
            None,
            &[("XDG_RUNTIME_DIR", &runtime)],
        )
        .expect_success("the install");
    assert_eq!(
        std::fs::read_link(store.store_root()).unwrap(),
        store.runtime.join("safix.d").join("1")
    );
}

#[test]
fn an_unknown_check_mode_is_refused_naming_the_three() {
    let store = Store::new("mode");
    let path = store.write("mode", &store.manifest());

    store
        .install(&["install", "--check-mode=sopsfile", path.to_str().unwrap()])
        .expect_refusal("there are three modes and that is not one of them")
        .says("--check-mode=off|manifest|document");
}

#[test]
fn a_version_this_binary_does_not_know_is_refused_naming_both() {
    let store = Store::new("version");
    let mut manifest = store.manifest();
    manifest["version"] = json!(2);
    let path = store.write("version", &manifest);

    store
        .install(&["install", "--check-mode=manifest", path.to_str().unwrap()])
        .expect_refusal("version 2 is not a schema this binary reads")
        .says("version 2");
}

#[test]
fn an_unknown_field_is_refused_rather_than_ignored() {
    let store = Store::new("unknown");
    let mut manifest = store.manifest();
    manifest["template"] = json!([]);
    let path = store.write("unknown", &manifest);

    store
        .install(&["install", "--check-mode=manifest", path.to_str().unwrap()])
        .expect_refusal("a field this runtime does not declare is a diagnosis")
        .says("template");
}

#[test]
fn a_user_mode_install_lands_under_the_runtime_directory_and_rotates_its_generations() {
    let store = Store::new("install");
    let path = store.write("install", &store.manifest());

    store
        .install(&["install", path.to_str().unwrap()])
        .expect_success("the first install");

    let root = store.store_root();
    let first = std::fs::read_link(&root).unwrap();
    assert_eq!(
        first,
        store.runtime.join("safix.d").join("1"),
        "%r expanded to the runtime directory and the first generation is 1"
    );

    let held = root.join("alice-alone");
    assert_eq!(std::fs::read_to_string(&held).unwrap(), "a value");
    assert_eq!(mode_of(&held), 0o400, "the declared mode is the file's");

    let nested = root.join("team-token");
    assert_eq!(
        std::fs::read_to_string(&nested).unwrap(),
        "deep",
        "the /-nested key resolved"
    );
    assert_eq!(mode_of(&nested), 0o440);

    let elsewhere = store.runtime.join("elsewhere").join("token");
    assert_eq!(
        std::fs::read_link(&elsewhere).unwrap(),
        root.join("team-token"),
        "an entry whose declared path is outside the store is linked back into it"
    );

    // A second run rotates, and prunes to keepGenerations = 1.
    store
        .install(&["install", path.to_str().unwrap()])
        .expect_success("the second install");
    assert_eq!(
        std::fs::read_link(&root).unwrap(),
        store.runtime.join("safix.d").join("2")
    );
    assert!(
        !store.runtime.join("safix.d").join("1").exists(),
        "keepGenerations = 1 leaves one generation"
    );
}

#[test]
fn a_doubled_per_cent_in_a_declared_path_installs_one_literal_per_cent() {
    let store = Store::new("literal");
    let mut manifest = store.manifest();
    // `%%` is the escape, so a path meaning to carry a per cent of its own
    // writes two. The entry is a third one beside the fixture's two, because
    // the expansion runs over every declared path and the claim is about what
    // reaches the filesystem rather than about the one path the store root is.
    manifest["secrets"]
        .as_array_mut()
        .unwrap()
        .push(store.entry(
            "literal-token",
            "alice-alone",
            "%r/100%% sure/token",
            "0400",
        ));
    let path = store.write("literal", &manifest);

    store
        .install(&["install", path.to_str().unwrap()])
        .expect_success("the install");

    let literal = store.runtime.join("100% sure").join("token");
    assert_eq!(
        std::fs::read_link(&literal).unwrap(),
        store.store_root().join("literal-token"),
        "%% became one literal per cent and %r expanded around it"
    );
    assert!(
        !store.runtime.join("100%% sure").exists(),
        "and the doubled spelling is nowhere on disk"
    );
}

#[test]
fn a_user_mode_install_mounts_nothing_chowns_nothing_and_restarts_nothing() {
    let store = Store::new("omissions");
    // The fixture's entries each declare `restartUnits`, so there is something
    // to propagate and the silence below is a decision rather than an empty
    // set. Both are also written afresh, so every entry reads as changed.
    let path = store.write("omissions", &store.manifest());

    // `systemctl` is resolved off PATH, so a script ahead of the real one on
    // PATH is what turns "no restart" into an observation. The environment
    // variable is the one that selects `systemctl` over the activation lists,
    // set so that the run would reach the shim if it propagated at all —
    // without it the omission could be the unit's registration path instead.
    let calls = store.fixture.tmpdir().join("systemctl-calls");
    let bin = store.fixture.tmpdir().join("omissions-bin");
    std::fs::create_dir_all(&bin).unwrap();
    let systemctl = bin.join("systemctl");
    std::fs::write(
        &systemctl,
        format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\n", calls.display()),
    )
    .unwrap();
    std::fs::set_permissions(&systemctl, std::fs::Permissions::from_mode(0o755)).unwrap();

    let runtime = store.runtime.display().to_string();
    let identity = store.identity.display().to_string();
    let search_path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    store
        .fixture
        .run_env(
            &["install", path.to_str().unwrap()],
            None,
            &[
                ("XDG_RUNTIME_DIR", &runtime),
                ("SOPS_AGE_KEY_FILE", &identity),
                ("PATH", &search_path),
                ("SOPS_RESTART_UNITS_VIA_SYSTEMCTL", "1"),
            ],
        )
        .expect_success("an unprivileged user-mode install");

    assert!(
        !calls.exists(),
        "a user-mode install propagated a restart, to units it declared but cannot own"
    );

    // Nothing was mounted over the store's root: a memory-backed filesystem
    // there would give the mount point a device of its own, and the runtime
    // directory it was made under is the device it has.
    let mount_point = store.runtime.join("safix.d");
    assert_eq!(
        device_of(&mount_point),
        device_of(&store.runtime),
        "a user-mode install mounted a filesystem over its store root"
    );

    // Nothing was chowned: the manifest declares uid 0 and gid 0 on every
    // entry, and what is on disk is the account that ran the install. This is
    // the one assertion the runner's own privileges could make vacuous, so the
    // two are compared rather than 0 being asserted absent — under a root
    // runner the claim is the VM test's, which installs system-scope and
    // measures the chown happening.
    let me = (
        rustix::process::getuid().as_raw(),
        rustix::process::getgid().as_raw(),
    );
    let generation = store.runtime.join("safix.d").join("1");
    for path in [generation.clone(), generation.join("alice-alone")] {
        let metadata = std::fs::metadata(&path).unwrap();
        assert_eq!(
            (metadata.uid(), metadata.gid()),
            me,
            "{} changed hands, against a manifest in user mode",
            path.display()
        );
    }
}

#[test]
fn dry_run_preserves_the_live_generation() {
    let store = Store::new("dry");
    let path = store.write("dry", &store.manifest());

    store
        .install(&["install", path.to_str().unwrap()])
        .expect_success("the first install");
    let before = std::fs::read_link(store.store_root()).unwrap();

    store
        .install(&["install", "--dry-run", path.to_str().unwrap()])
        .expect_success("a dry run is a run");

    assert_eq!(
        std::fs::read_link(store.store_root()).unwrap(),
        before,
        "the symlink did not move"
    );
    // The defect a booted host found: with keepGenerations = 1, a dry run
    // that went on to prune deleted generation 1 — the one the symlink still
    // names — so a run promising to change nothing emptied the store.
    assert!(
        store.runtime.join("safix.d").join("1").is_dir(),
        "the live generation survives a dry run"
    );
    assert_eq!(
        std::fs::read_to_string(store.store_root().join("alice-alone")).unwrap(),
        "a value",
        "and is still readable through the symlink"
    );
}

#[test]
fn the_host_s_own_dry_activation_signal_implies_the_switch() {
    let store = Store::new("activation");
    let path = store.write("activation", &store.manifest());

    store
        .install(&["install", path.to_str().unwrap()])
        .expect_success("the first install");
    let before = std::fs::read_link(store.store_root()).unwrap();

    let runtime = store.runtime.display().to_string();
    store
        .fixture
        .run_env(
            &["install", path.to_str().unwrap()],
            None,
            &[
                ("XDG_RUNTIME_DIR", &runtime),
                ("NIXOS_ACTION", "dry-activate"),
            ],
        )
        .expect_success("a dry activation is a run");

    assert_eq!(
        std::fs::read_link(store.store_root()).unwrap(),
        before,
        "NIXOS_ACTION=dry-activate is read by the program, not translated by the script"
    );
}

#[test]
fn two_entries_of_one_document_cost_one_sops_invocation() {
    let store = Store::new("granularity");
    let path = store.write("granularity", &store.manifest());

    // A sops that records that it ran and then is the real sops. Counting is
    // the claim: both entries are read out of one document, so a run that
    // decrypted per entry would be two lines here and the same store on disk.
    let ledger = store.fixture.tmpdir().join("sops-invocations");
    let counting = store.fixture.tmpdir().join("counting-sops");
    std::fs::write(
        &counting,
        format!(
            "#!/bin/sh\necho ran >> {}\nexec {} \"$@\"\n",
            ledger.display(),
            harness::real_sops()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&counting, std::fs::Permissions::from_mode(0o755)).unwrap();

    let runtime = store.runtime.display().to_string();
    let sops = counting.display().to_string();
    store
        .fixture
        .run_env(
            &["install", path.to_str().unwrap()],
            None,
            &[("XDG_RUNTIME_DIR", &runtime), ("SAFIX_SOPS", &sops)],
        )
        .expect_success("the install");

    let ran = std::fs::read_to_string(&ledger).unwrap_or_default();
    assert_eq!(
        ran.lines().count(),
        1,
        "one document, one subprocess, whatever the entry count: {ran}"
    );
}

#[test]
fn publication_preserves_existing_non_symlink_destinations() {
    let store = Store::new("coexistence");
    let path = store.write("coexistence", &store.manifest());

    let root = store.store_root();
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("left-behind"), b"retained").unwrap();

    store
        .install(&["install", path.to_str().unwrap()])
        .expect_refusal("publication cannot replace a directory the installer does not own");
    assert_eq!(
        std::fs::read(root.join("left-behind")).unwrap(),
        b"retained"
    );
    assert!(std::fs::symlink_metadata(&root).unwrap().is_dir());
}

/// The committed manifest fixture, carried inside the compiled suite rather
/// than read off the source tree: the per-claim checks run this binary in a
/// derivation that has no source tree at all.
const FIXTURE_MANIFEST: &str = include_str!("support/install-manifest.json");

#[test]
fn the_hand_written_fixture_manifest_is_accepted_and_a_mutation_of_it_is_not() {
    let store = Store::new("fixture");
    let fixture: Value = serde_json::from_str(FIXTURE_MANIFEST).expect("the fixture is json");
    let fixture_path = store.write("fixture", &fixture);

    store
        .install(&[
            "install",
            "--check-mode=manifest",
            fixture_path.to_str().unwrap(),
        ])
        .expect_success("the committed manifest fixture is this schema");

    let mut mutated = fixture;
    mutated["secrets"][0]["mode"] = json!("0o400");
    let path = store.write("mutated", &mutated);
    store
        .install(&["install", "--check-mode=manifest", path.to_str().unwrap()])
        .expect_refusal("a mode that is not octal is refused naming the entry")
        .says("alice-alone");
}

#[test]
fn store_refuses_a_symlinked_mount_without_weakening_its_target() {
    let store = Store::new("linked-mount");
    let protected = store.runtime.join("protected");
    std::fs::create_dir(&protected).unwrap();
    std::fs::set_permissions(&protected, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(protected.join("keep"), b"retained").unwrap();
    std::os::unix::fs::symlink(&protected, store.runtime.join("safix.d")).unwrap();
    let path = store.write("linked-mount", &store.manifest());

    store
        .install(&["install", path.to_str().unwrap()])
        .expect_refusal("a mount path must not redirect into another directory");
    assert_eq!(mode_of(&protected), 0o700);
    assert_eq!(std::fs::read(protected.join("keep")).unwrap(), b"retained");
    assert!(!store.store_root().exists());
}

#[test]
fn store_refuses_an_exhausted_counter_without_replacing_live_data() {
    let store = Store::new("exhausted-counter");
    let generation = store.runtime.join("safix.d").join(u64::MAX.to_string());
    std::fs::create_dir_all(&generation).unwrap();
    std::fs::write(generation.join("alice-alone"), b"retained").unwrap();
    std::os::unix::fs::symlink(&generation, store.store_root()).unwrap();
    let path = store.write("exhausted-counter", &store.manifest());

    store
        .install(&["install", path.to_str().unwrap()])
        .expect_refusal("generation exhaustion must not reuse the live directory");
    assert_eq!(std::fs::read_link(store.store_root()).unwrap(), generation);
    assert_eq!(
        std::fs::read(store.store_root().join("alice-alone")).unwrap(),
        b"retained"
    );
}
