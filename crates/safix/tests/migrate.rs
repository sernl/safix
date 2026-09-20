//! The migrate verb, over real identities and real ciphertext.
//!
//! Every plan here is built from age identities minted inside the fixture's own
//! scratch directory and from sources encrypted by the real `age` and the real
//! `sops`. Nothing is stubbed: what these tests drive is the tool an operator
//! runs, and what they read back they read with `age` and `sops` rather than
//! through the runtime that wrote it.
//!
//! # How the interrupted states are built
//!
//! The runtime has no halt hook and gains none for a test. A partially
//! published migration is instead constructed by hand, exactly as the journal
//! contract describes one: a destination encrypted to the target recipient, and
//! a journal beside the receipt recording that destination's path, file
//! identity and digest. Both the plan digest and the byte digests are taken
//! with `sha256sum`, so the oracle the runtime is held to is coreutils' and not
//! the runtime's own.

// A test's failure is the point; see the note at the head of `harness/mod.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod harness;

use std::io::Write as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use harness::{Fixture, mint_identity, real_sops, recipient_of, shim};
use serde_json::{Value, json};

/// How one entry of a plan stores its source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceKind {
    /// A whole raw age file.
    Age,
    /// One key of a SOPS YAML document.
    SopsYaml,
}

/// How one entry of a plan stores its destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DestinationKind {
    /// A whole raw age file, which `age --decrypt` reads without safix.
    Age,
    /// One key of a SOPS YAML document, which is what the `sops` shim sees.
    SopsYaml,
}

/// One entry, as the fixture knows it.
struct Entry {
    name: String,
    value: String,
    source: PathBuf,
    destination: PathBuf,
    /// Whose recipient the destination is encrypted to: the target identity
    /// unless a test wants a candidate the target cannot open.
    recipient: String,
    destination_kind: DestinationKind,
}

/// A migration's whole world: two isolated identities, sources encrypted to
/// one, destinations to be encrypted to the other, and the plan naming them.
struct Migration {
    root: PathBuf,
    out: PathBuf,
    source_identity: PathBuf,
    target_identity: PathBuf,
    target_recipient: String,
    entries: Vec<Entry>,
    plan: PathBuf,
    receipt: PathBuf,
    journal: PathBuf,
    declarations: PathBuf,
}

/// The key a SOPS YAML source or destination stores its value under.
const KEY: &str = "value";

impl Migration {
    /// A plan whose entries are all whole raw age documents on both sides.
    fn new(fixture: &Fixture, label: &str, entries: &[(&str, &str)]) -> Self {
        let shaped: Vec<(&str, &str, SourceKind, DestinationKind)> = entries
            .iter()
            .map(|(name, value)| (*name, *value, SourceKind::Age, DestinationKind::Age))
            .collect();
        Self::shaped(fixture, label, &shaped)
    }

    /// A plan whose entries name their own source and destination containers.
    fn shaped(
        fixture: &Fixture,
        label: &str,
        entries: &[(&str, &str, SourceKind, DestinationKind)],
    ) -> Self {
        let root = fixture.scratch(label);
        let sources = root.join("sources");
        let out = root.join("out");
        std::fs::create_dir_all(&sources).unwrap();
        std::fs::create_dir_all(&out).unwrap();

        let source_identity = root.join("source-identity.txt");
        let target_identity = root.join("target-identity.txt");
        mint_identity(&source_identity);
        mint_identity(&target_identity);
        let source_recipient = recipient_of(&source_identity);
        let target_recipient = recipient_of(&target_identity);

        let built = entries
            .iter()
            .map(|(name, value, source_kind, destination_kind)| {
                let source = match source_kind {
                    SourceKind::Age => {
                        let path = sources.join(format!("{name}.age"));
                        age_encrypt(&source_recipient, value.as_bytes(), &path);
                        path
                    }
                    SourceKind::SopsYaml => {
                        let path = sources.join(format!("{name}.yaml"));
                        sops_encrypt_yaml(&source_recipient, value, &path);
                        path
                    }
                };
                Entry {
                    name: (*name).to_owned(),
                    value: (*value).to_owned(),
                    source,
                    destination: out.join(match destination_kind {
                        DestinationKind::Age => format!("{name}.age"),
                        DestinationKind::SopsYaml => format!("{name}.yaml"),
                    }),
                    recipient: target_recipient.clone(),
                    destination_kind: *destination_kind,
                }
            })
            .collect();

        let migration = Self {
            plan: root.join("plan.json"),
            receipt: out.join("receipt.json"),
            journal: out.join("receipt.json.journal"),
            declarations: out.join("secrets.nix"),
            root,
            out,
            source_identity,
            target_identity,
            target_recipient,
            entries: built,
        };
        migration.write_plan();
        migration
    }

    /// Point one entry's destination at a recipient the target identity is not,
    /// which is a candidate the target cannot open.
    fn encrypt_one_to_a_stranger(&mut self, name: &str) {
        let stranger = self.root.join("stranger-identity.txt");
        mint_identity(&stranger);
        let recipient = recipient_of(&stranger);
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.name == name)
            .expect("the plan names that entry");
        entry.recipient = recipient;
        self.write_plan();
    }

    fn write_plan(&self) {
        let entries: Vec<Value> = self
            .entries
            .iter()
            .map(|entry| {
                let (destination_format, destination_key) = match entry.destination_kind {
                    DestinationKind::Age => ("age", ""),
                    DestinationKind::SopsYaml => ("yaml", KEY),
                };
                let source_is_yaml = entry
                    .source
                    .extension()
                    .is_some_and(|extension| extension == "yaml");
                json!({
                    "name": entry.name,
                    "source": {
                        "path": entry.source,
                        "format": if source_is_yaml { "yaml" } else { "age" },
                        "key": if source_is_yaml { KEY } else { "" },
                    },
                    "destination": {
                        "path": entry.destination,
                        "format": destination_format,
                        "key": destination_key,
                    },
                    "recipients": [entry.recipient],
                    "deployment": {
                        "path": format!("/run/secrets/{}", entry.name),
                        "mode": "0400",
                        "owner": "root",
                    },
                })
            })
            .collect();
        let plan = json!({
            "version": 1,
            "sourceIdentities": { "ageKeyFile": self.source_identity },
            "targetIdentities": { "ageKeyFile": self.target_identity },
            "entries": entries,
            "receipt": self.receipt,
            "deploymentOutput": self.declarations,
            "deploymentTarget": "safix",
        });
        std::fs::write(&self.plan, serde_json::to_string_pretty(&plan).unwrap()).unwrap();
    }

    /// The plan as the runtime names it: the argument a run is given.
    fn argument(&self) -> String {
        self.plan.display().to_string()
    }

    /// Every source's bytes, for the claim that a run changed none of them.
    fn source_bytes(&self) -> Vec<Vec<u8>> {
        self.entries
            .iter()
            .map(|entry| std::fs::read(&entry.source).unwrap())
            .collect()
    }

    /// The staging directories left under the destination directory, which a
    /// finished or rolled-back run leaves none of.
    fn staging_directories(&self) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = std::fs::read_dir(&self.out)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(".safix-migrate-"))
            })
            .collect();
        found.sort();
        found
    }

    /// Publish one entry's destination by hand, as an interrupted run would
    /// have left it: the source's own value, encrypted to the target recipient.
    fn publish_by_hand(&self, name: &str) -> &Entry {
        let entry = self.entry(name);
        assert_eq!(
            entry.destination_kind,
            DestinationKind::Age,
            "a hand-published destination is read back with age"
        );
        age_encrypt(
            &self.target_recipient,
            entry.value.as_bytes(),
            &entry.destination,
        );
        entry
    }

    /// The journal an interrupted run would have left, recording the named
    /// destinations and staging directories.
    fn write_journal(&self, published: &[&str], staging: &[PathBuf]) {
        self.write_journal_for(&self.plan, published, staging);
    }

    /// The same, naming another plan — which is what a journal from a different
    /// migration looks like from here.
    fn write_journal_for(&self, plan: &Path, published: &[&str], staging: &[PathBuf]) {
        let outputs: Vec<Value> = published
            .iter()
            .map(|name| {
                let path = &self.entry(name).destination;
                let metadata = std::fs::metadata(path).unwrap();
                json!({
                    "path": path,
                    "kind": "ciphertext",
                    "device": metadata.dev(),
                    "inode": metadata.ino(),
                    "sha256": sha256_of(path),
                })
            })
            .collect();
        let journal = json!({
            "version": 1,
            "planDigest": plan_digest(plan),
            "plan": std::fs::canonicalize(plan).unwrap(),
            "staging": staging,
            "outputs": outputs,
        });
        std::fs::write(
            &self.journal,
            serde_json::to_string_pretty(&journal).unwrap(),
        )
        .unwrap();
    }

    /// The journal a run killed between a record and its hard link leaves: the
    /// output is named, with the identity and digest the link would have given
    /// it, and nothing is at the path.
    fn write_journal_naming_an_unlinked_output(&self, name: &str) {
        let path = &self.entry(name).destination;
        // Written, measured and removed, so the record carries a real identity
        // and a real digest for a path that holds nothing.
        age_encrypt(
            &self.target_recipient,
            self.entry(name).value.as_bytes(),
            path,
        );
        let metadata = std::fs::metadata(path).unwrap();
        let record = json!({
            "path": path,
            "kind": "ciphertext",
            "device": metadata.dev(),
            "inode": metadata.ino(),
            "sha256": sha256_of(path),
        });
        std::fs::remove_file(path).unwrap();
        let journal = json!({
            "version": 1,
            "planDigest": plan_digest(&self.plan),
            "plan": std::fs::canonicalize(&self.plan).unwrap(),
            "staging": Vec::<PathBuf>::new(),
            "outputs": [record],
        });
        std::fs::write(
            &self.journal,
            serde_json::to_string_pretty(&journal).unwrap(),
        )
        .unwrap();
    }

    fn entry(&self, name: &str) -> &Entry {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .expect("the plan names that entry")
    }

    /// What the target identity reads out of one published destination.
    fn published_value(&self, name: &str) -> String {
        age_decrypt(&self.target_identity, &self.entry(name).destination)
    }
}

/// One value, encrypted to one recipient by the real `age`.
fn age_encrypt(recipient: &str, value: &[u8], destination: &Path) {
    let mut child = Command::new("age")
        .arg("--encrypt")
        .arg("--recipient")
        .arg(recipient)
        .arg("--output")
        .arg(destination)
        .stdin(Stdio::piped())
        .spawn()
        .expect("age does not run");
    child
        .stdin
        .take()
        .expect("age takes no input")
        .write_all(value)
        .unwrap();
    assert!(
        child.wait().unwrap().success(),
        "age could not encrypt {}",
        destination.display()
    );
}

/// What one age file holds, read with `age` and one identity.
fn age_decrypt(identity: &Path, path: &Path) -> String {
    let output = Command::new("age")
        .arg("--decrypt")
        .arg("-i")
        .arg(identity)
        .arg(path)
        .output()
        .expect("age does not run");
    assert!(
        output.status.success(),
        "age could not decrypt {}",
        path.display()
    );
    String::from_utf8(output.stdout).expect("the fixture values are text")
}

/// One value under [`KEY`], in a SOPS YAML document encrypted to one recipient.
fn sops_encrypt_yaml(recipient: &str, value: &str, destination: &Path) {
    let plaintext = destination.with_extension("plain.yaml");
    std::fs::write(&plaintext, format!("{KEY}: {value}\n")).unwrap();
    let output = Command::new(real_sops())
        .arg("--encrypt")
        .arg("--config")
        .arg("/dev/null")
        .arg("--age")
        .arg(recipient)
        .arg("--encrypted-regex")
        .arg(".*")
        .arg("--input-type")
        .arg("yaml")
        .arg("--output-type")
        .arg("yaml")
        .arg(&plaintext)
        .output()
        .expect("sops does not run");
    assert!(
        output.status.success(),
        "sops could not encrypt {}: {}",
        destination.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::write(destination, output.stdout).unwrap();
    std::fs::remove_file(&plaintext).unwrap();
}

/// The digest of a file's bytes, taken with coreutils rather than with the
/// implementation under test.
fn sha256_of(path: &Path) -> String {
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .expect("sha256sum does not run");
    assert!(output.status.success(), "sha256sum failed");
    String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .expect("sha256sum prints a digest")
        .to_owned()
}

/// The digest the journal names a plan by: its canonical directory, a NUL, and
/// the plan's bytes — hashed by coreutils.
fn plan_digest(plan: &Path) -> String {
    let canonical = std::fs::canonicalize(plan).unwrap();
    let mut material = Vec::new();
    material.extend_from_slice(
        canonical
            .parent()
            .unwrap()
            .to_str()
            .expect("the fixture's paths are text")
            .as_bytes(),
    );
    material.push(0);
    material.extend_from_slice(&std::fs::read(&canonical).unwrap());

    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("sha256sum does not run");
    child
        .stdin
        .take()
        .expect("sha256sum takes no input")
        .write_all(&material)
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "sha256sum failed");
    String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .expect("sha256sum prints a digest")
        .to_owned()
}

/// A directory shaped like one a migration stages in, holding one file, for the
/// abandonment that has to sweep it.
fn staging_directory(out: &Path, name: &str) -> PathBuf {
    let directory = out.join(format!(".safix-migrate-{name}"));
    std::fs::create_dir(&directory).unwrap();
    std::fs::set_permissions(
        &directory,
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )
    .unwrap();
    std::fs::write(directory.join("candidate"), b"leftover").unwrap();
    directory
}

/// The straight run: three entries, two source containers, and everything the
/// plan names published exactly once.
#[test]
fn a_plan_publishes_its_ciphertext_declarations_and_receipt() {
    let fixture = Fixture::new();
    let migration = Migration::shaped(
        &fixture,
        "straight",
        &[
            (
                "api-token",
                "CANARY-api-token",
                SourceKind::Age,
                DestinationKind::Age,
            ),
            (
                "db-password",
                "CANARY-db-password",
                SourceKind::SopsYaml,
                DestinationKind::Age,
            ),
        ],
    );
    let sources = migration.source_bytes();

    fixture
        .run(&["migrate", &migration.argument()])
        .expect_success("a plan whose candidates all verify");

    for entry in &migration.entries {
        assert!(
            entry.destination.is_file(),
            "{} was not published",
            entry.destination.display()
        );
        assert_eq!(
            migration.published_value(&entry.name),
            entry.value,
            "{} does not hold its source's value",
            entry.destination.display()
        );
    }
    assert!(migration.declarations.is_file(), "no declarations");
    assert!(migration.receipt.is_file(), "no receipt");
    assert!(
        !migration.journal.exists(),
        "a completed migration left a journal"
    );
    assert_eq!(
        migration.staging_directories(),
        Vec::<PathBuf>::new(),
        "a completed migration left a staging directory"
    );
    assert_eq!(
        migration.source_bytes(),
        sources,
        "a migration changed a source"
    );
}

/// A completed migration's outputs are not resumable and not abandonable: the
/// second run refuses them the way it always did, and `--abandon` refuses
/// without a journal rather than removing what a finished run published.
#[test]
fn a_completed_migration_leaves_nothing_to_resume_or_abandon() {
    let fixture = Fixture::new();
    let migration = Migration::new(&fixture, "completed", &[("api-token", "CANARY-token")]);
    fixture
        .run(&["migrate", &migration.argument()])
        .expect_success("the first run");
    let published = std::fs::read(&migration.entry("api-token").destination).unwrap();

    fixture
        .run(&["migrate", &migration.argument()])
        .expect_refusal("a rerun of a completed migration")
        .says("already exists");
    fixture
        .run(&["migrate", "--abandon", &migration.argument()])
        .expect_refusal("abandoning a migration that completed")
        .says("no migration journal");

    assert_eq!(
        std::fs::read(&migration.entry("api-token").destination).unwrap(),
        published,
        "a refused run changed a published output"
    );
    assert!(
        migration.receipt.is_file(),
        "a refused run removed the receipt"
    );
}

/// The rollback scenario: a later candidate the target identity cannot open.
#[test]
fn a_candidate_the_target_cannot_open_publishes_nothing() {
    let fixture = Fixture::new();
    let mut migration = Migration::new(
        &fixture,
        "stranger",
        &[
            ("api-token", "CANARY-api-token"),
            ("db-password", "CANARY-db-password"),
            ("service-key", "CANARY-service-key"),
        ],
    );
    migration.encrypt_one_to_a_stranger("db-password");
    let sources = migration.source_bytes();

    fixture
        .run(&["migrate", &migration.argument()])
        .expect_refusal("a candidate outside the target audience")
        .says("independent target decryption failed");

    for entry in &migration.entries {
        assert!(
            !entry.destination.exists(),
            "{} was published despite a failed verification",
            entry.destination.display()
        );
    }
    assert!(
        !migration.declarations.exists(),
        "declarations were published"
    );
    assert!(!migration.receipt.exists(), "a receipt was published");
    assert!(!migration.journal.exists(), "a journal was left");
    assert_eq!(
        migration.staging_directories(),
        Vec::<PathBuf>::new(),
        "a rolled-back run left a staging directory"
    );
    assert_eq!(
        migration.source_bytes(),
        sources,
        "a refused migration changed a source"
    );
}

/// A rerun finishes what an interruption left, keeping what the journal proves.
#[test]
fn a_rerun_finishes_what_the_crash_left() {
    let fixture = Fixture::new();
    let migration = Migration::new(
        &fixture,
        "resume",
        &[
            ("api-token", "CANARY-api-token"),
            ("db-password", "CANARY-db-password"),
        ],
    );
    let kept = migration.publish_by_hand("api-token");
    let kept_identity = std::fs::metadata(&kept.destination).unwrap().ino();
    let kept_bytes = std::fs::read(&kept.destination).unwrap();
    migration.write_journal(&["api-token"], &[]);
    let sources = migration.source_bytes();

    fixture
        .run(&["migrate", &migration.argument()])
        .expect_success("a rerun of an interrupted plan")
        .says("Resuming an interrupted migration");

    assert_eq!(
        std::fs::metadata(&kept.destination).unwrap().ino(),
        kept_identity,
        "the kept output was republished rather than kept"
    );
    assert_eq!(
        std::fs::read(&kept.destination).unwrap(),
        kept_bytes,
        "the kept output's bytes changed"
    );
    for entry in &migration.entries {
        assert_eq!(
            migration.published_value(&entry.name),
            entry.value,
            "{} does not hold its source's value",
            entry.destination.display()
        );
    }
    assert!(migration.declarations.is_file(), "no declarations");
    assert!(migration.receipt.is_file(), "no receipt");
    assert!(
        !migration.journal.exists(),
        "the finished rerun left its journal"
    );
    assert_eq!(
        migration.source_bytes(),
        sources,
        "a rerun changed a source"
    );
}

/// A record whose hard link never landed is published rather than refused.
///
/// This is the window a real `kill -9` lands in: the journal is written before
/// each link, so a process killed between the two leaves a record naming a
/// path that holds nothing. The rerun has to publish it — a refusal here would
/// leave the operator with a migration that can neither finish nor be
/// abandoned, which is the state this whole change exists to remove.
#[test]
fn a_rerun_publishes_a_recorded_output_whose_link_never_landed() {
    let fixture = Fixture::new();
    let migration = Migration::new(
        &fixture,
        "unlinked",
        &[
            ("api-token", "CANARY-api-token"),
            ("db-password", "CANARY-db-password"),
        ],
    );
    migration.write_journal_naming_an_unlinked_output("api-token");
    assert!(
        !migration.entry("api-token").destination.exists(),
        "the fixture left the output it records as unlinked"
    );

    fixture
        .run(&["migrate", &migration.argument()])
        .expect_success("a rerun over a record whose link never landed");

    for entry in &migration.entries {
        assert_eq!(
            migration.published_value(&entry.name),
            entry.value,
            "{} does not hold its source's value",
            entry.destination.display()
        );
    }
    assert!(migration.receipt.is_file(), "no receipt");
    assert!(!migration.journal.exists(), "the rerun left its journal");
}

/// A recorded output somebody replaced stops the rerun before it publishes.
#[test]
fn a_rerun_refuses_an_output_someone_replaced() {
    let fixture = Fixture::new();
    let migration = Migration::new(
        &fixture,
        "replaced",
        &[
            ("api-token", "CANARY-api-token"),
            ("db-password", "CANARY-db-password"),
        ],
    );
    let replaced = migration.publish_by_hand("api-token");
    migration.write_journal(&["api-token"], &[]);
    std::fs::write(&replaced.destination, b"not what the journal recorded").unwrap();
    let journal_before = std::fs::read(&migration.journal).unwrap();

    fixture
        .run(&["migrate", &migration.argument()])
        .expect_refusal("a rerun over a replaced output")
        .says(&replaced.destination.display().to_string())
        .says("no longer matches the migration journal");

    assert_eq!(
        std::fs::read(&replaced.destination).unwrap(),
        b"not what the journal recorded",
        "the refused rerun touched the replaced file"
    );
    assert_eq!(
        std::fs::read(&migration.journal).unwrap(),
        journal_before,
        "the refused rerun rewrote the journal"
    );
    assert!(
        !migration.entry("db-password").destination.exists(),
        "the refused rerun published an output"
    );
    assert!(
        !migration.receipt.exists(),
        "the refused rerun wrote a receipt"
    );
}

/// A journal another plan wrote is refused, naming both plans.
#[test]
fn a_journal_from_another_plan_is_not_resumed() {
    let fixture = Fixture::new();
    let migration = Migration::new(
        &fixture,
        "other-plan",
        &[
            ("api-token", "CANARY-api-token"),
            ("db-password", "CANARY-db-password"),
        ],
    );
    let other = migration.root.join("other-plan.json");
    std::fs::copy(&migration.plan, &other).unwrap();
    // The same JSON at another path is another plan, which is the property the
    // digest's directory component exists for; a changed byte makes it plainer.
    let mut bytes = std::fs::read(&other).unwrap();
    bytes.push(b'\n');
    std::fs::write(&other, bytes).unwrap();

    migration.publish_by_hand("api-token");
    migration.write_journal_for(&other, &["api-token"], &[]);
    let journal_before = std::fs::read(&migration.journal).unwrap();

    let run = fixture
        .run(&["migrate", &migration.argument()])
        .expect_refusal("a journal from another plan");
    run.says(&other.display().to_string());
    run.says(
        &std::fs::canonicalize(&migration.plan)
            .unwrap()
            .display()
            .to_string(),
    );

    assert_eq!(
        std::fs::read(&migration.journal).unwrap(),
        journal_before,
        "the refused run rewrote the journal"
    );
    assert!(
        !migration.entry("db-password").destination.exists(),
        "the refused run published an output"
    );
    assert!(
        migration.entry("api-token").destination.is_file(),
        "the refused run removed a recorded output"
    );
}

/// Abandonment removes what the journal proves, and leaves everything else.
#[test]
fn abandoning_removes_only_what_the_journal_proves() {
    let fixture = Fixture::new();
    let migration = Migration::new(
        &fixture,
        "abandon",
        &[
            ("api-token", "CANARY-api-token"),
            ("db-password", "CANARY-db-password"),
        ],
    );
    let abandoned = migration.publish_by_hand("api-token");
    let staging = staging_directory(&migration.out, "fixture-1");
    let bystander = migration.out.join("an-operator-put-this-here");
    std::fs::write(&bystander, b"not the tool's to remove").unwrap();
    migration.write_journal(&["api-token"], std::slice::from_ref(&staging));
    let sources = migration.source_bytes();

    fixture
        .run(&["migrate", "--abandon", &migration.argument()])
        .expect_success("abandoning an interrupted migration")
        .says("Abandoned the interrupted migration");

    assert!(
        !abandoned.destination.exists(),
        "the recorded output survived abandonment"
    );
    assert!(!staging.exists(), "the recorded staging directory survived");
    assert!(!migration.journal.exists(), "the journal survived");
    assert!(
        bystander.is_file(),
        "abandonment removed an unrecorded file"
    );
    assert_eq!(
        migration.source_bytes(),
        sources,
        "abandonment changed a source"
    );
}

/// Abandonment refuses a recorded output that no longer matches, and removes
/// nothing at all.
#[test]
fn abandoning_refuses_a_replaced_output() {
    let fixture = Fixture::new();
    let migration = Migration::new(
        &fixture,
        "abandon-replaced",
        &[
            ("api-token", "CANARY-api-token"),
            ("db-password", "CANARY-db-password"),
        ],
    );
    let first = migration.publish_by_hand("api-token");
    let second = migration.publish_by_hand("db-password");
    migration.write_journal(&["api-token", "db-password"], &[]);
    std::fs::write(&second.destination, b"replaced after the journal").unwrap();

    fixture
        .run(&["migrate", "--abandon", &migration.argument()])
        .expect_refusal("abandoning over a replaced output")
        .says(&second.destination.display().to_string())
        .says("no longer matches the migration journal");

    assert!(
        first.destination.is_file(),
        "a refused abandonment removed a recorded output"
    );
    assert_eq!(
        std::fs::read(&second.destination).unwrap(),
        b"replaced after the journal",
        "a refused abandonment touched the replaced file"
    );
    assert!(
        migration.journal.is_file(),
        "a refused abandonment removed the journal"
    );
}

/// A signal delivered while the candidate is being encrypted stops the run,
/// rolls it back and leaves neither outputs nor a journal.
///
/// The signal comes from inside `sops` rather than from a timer, so the window
/// drilled is the one where the encryption is in flight; see the head of
/// `tests/support/shim.rs`.
#[test]
fn a_signal_during_encryption_publishes_nothing_and_leaves_no_journal() {
    let fixture = Fixture::new();
    let migration = Migration::shaped(
        &fixture,
        "interrupted",
        &[
            (
                "api-token",
                "CANARY-api-token",
                SourceKind::Age,
                DestinationKind::SopsYaml,
            ),
            (
                "db-password",
                "CANARY-db-password",
                SourceKind::Age,
                DestinationKind::SopsYaml,
            ),
        ],
    );
    let sources = migration.source_bytes();
    let sops = real_sops();

    let run = fixture.run_env(
        &["migrate", &migration.argument()],
        None,
        &[
            ("SAFIX_SOPS", shim()),
            ("SAFIX_SHIM_ROLE", "interrupt"),
            ("SAFIX_SHIM_SOPS", &sops),
            // Only in front of the encryption, which is the one interruptible
            // step before publication.
            ("SAFIX_SHIM_HOLD", "--encrypt"),
        ],
    );

    assert_eq!(
        run.code,
        Some(130),
        "an interrupted migration exits 130; stderr: {}",
        run.stderr
    );
    for entry in &migration.entries {
        assert!(
            !entry.destination.exists(),
            "{} survived an interrupted run",
            entry.destination.display()
        );
    }
    assert!(
        !migration.declarations.exists(),
        "declarations were published"
    );
    assert!(!migration.receipt.exists(), "a receipt was published");
    assert!(
        !migration.journal.exists(),
        "an interrupted run left a journal"
    );
    assert_eq!(
        migration.staging_directories(),
        Vec::<PathBuf>::new(),
        "an interrupted run left a staging directory"
    );
    assert_eq!(
        migration.source_bytes(),
        sources,
        "an interrupted run changed a source"
    );
}

/// The help states the recovery contract rather than advising an inspection by
/// hand.
#[test]
fn the_help_states_the_recovery_contract() {
    let fixture = Fixture::new();
    let run = fixture
        .run(&["migrate", "--help"])
        .expect_success("the migrate help");
    run.says("journal");
    run.says("--abandon");
    run.says("Rerunning the same plan while its journal exists resumes");
    run.says("Sources are retained on every path, abandonment included.");
    run.silent_about("inspect");
}

/// `--abandon` takes exactly one plan.
#[test]
fn abandon_takes_exactly_one_plan() {
    let fixture = Fixture::new();
    let migration = Migration::new(&fixture, "arity", &[("api-token", "CANARY-token")]);

    fixture
        .run(&["migrate", "--abandon"])
        .expect_refusal("--abandon with no plan")
        .says("migrate --abandon <plan.json>");
    fixture
        .run(&["migrate", "--abandon", &migration.argument(), "extra.json"])
        .expect_refusal("--abandon with two plans")
        .says("migrate --abandon <plan.json>");
}
