//! Recovery must cover private components, not merely certificate fingerprints.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use std::io::Write as _;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use harness::Fixture;

struct Keyring(PathBuf);

impl Keyring {
    fn new(fixture: &Fixture, name: &str) -> Self {
        let path = fixture.scratch_dir(name);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }

    fn gpg(&self, arguments: &[&str], input: &[u8]) -> Vec<u8> {
        let mut child = Command::new("gpg")
            .args(["--no-options", "--homedir"])
            .arg(&self.0)
            .args(["--batch", "--pinentry-mode", "loopback", "--passphrase", ""])
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "GPG fixture operation failed");
        output.stdout
    }

    fn generate(&self, uid: &str, encryption_grip: Option<&str>) -> String {
        let reuse = encryption_grip
            .map(|grip| format!("Subkey-grip: {grip}\n"))
            .unwrap_or_default();
        let parameters = format!(
            "Key-Type: eddsa\nKey-Curve: Ed25519\nKey-Usage: cert\n\
             Subkey-Type: ecdh\nSubkey-Curve: Curve25519\nSubkey-Usage: encrypt\n\
             {reuse}Name-Real: Disposable Custody Fixture\nName-Email: {uid}\n\
             Expire-Date: 2y\n%no-protection\n%commit\n"
        );
        self.gpg(&["--generate-key"], parameters.as_bytes());
        self.fields(uid, "fpr").into_iter().next().unwrap()
    }

    fn fields(&self, fingerprint: &str, kind: &str) -> Vec<String> {
        let listing = self.gpg(
            &[
                "--with-colons",
                "--with-keygrip",
                "--list-secret-keys",
                fingerprint,
            ],
            &[],
        );
        String::from_utf8(listing)
            .unwrap()
            .lines()
            .filter_map(|line| {
                let mut fields = line.split(':');
                (fields.next() == Some(kind)).then(|| fields.nth(8).unwrap().to_owned())
            })
            .collect()
    }

    fn component(&self, grip: &str) -> PathBuf {
        self.0.join("private-keys-v1.d").join(format!("{grip}.key"))
    }

    fn copy_component(&self, from: &Path, grip: &str) -> PathBuf {
        let directory = self.0.join("private-keys-v1.d");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let destination = self.component(grip);
        std::fs::copy(from, &destination).unwrap();
        destination
    }

    fn command(&self, fixture: &Fixture, arguments: &[&str]) -> Command {
        let mut command = fixture.command(arguments);
        command
            .env("SAFIX_GNUPGHOME", &self.0)
            .env("SAFIX_GPG", "gpg")
            .env("SAFIX_GPGCONF", "gpgconf")
            .env("SOPS_GPG_EXEC", "gpg");
        command
    }
}

impl Drop for Keyring {
    fn drop(&mut self) {
        let _ = Command::new("gpgconf")
            .arg("--homedir")
            .arg(&self.0)
            .args(["--kill", "gpg-agent"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn recovery_key(fixture: &Fixture) -> PathBuf {
    let command = fixture.command(&[]);
    command
        .get_envs()
        .find(|(name, _)| *name == "SOPS_AGE_KEY_FILE")
        .and_then(|(_, value)| value)
        .map(PathBuf::from)
        .unwrap()
}

fn backup(source: &Keyring, fixture: &Fixture, destination: &Path) -> Output {
    source
        .command(fixture, &["identity", "backup", "pgp"])
        .arg(destination)
        .args(["--recipient", &fixture.alice, "--age-key-file"])
        .arg(recovery_key(fixture))
        .output()
        .unwrap()
}

#[test]
fn an_unlisted_private_component_cannot_be_lost_in_backup_or_overwritten_by_restore() {
    let fixture = Fixture::new();
    let source = Keyring::new(&fixture, "source-keyring");
    let owner = source.generate("owner@example.invalid", None);
    let complete = fixture.scratch("complete-backup.json");
    let taken = backup(&source, &fixture, &complete);
    assert!(
        taken.status.success(),
        "a complete keyring was not backed up: {}",
        String::from_utf8_lossy(&taken.stderr)
    );

    let other = Keyring::new(&fixture, "other-keyring");
    let other_fingerprint = other.generate("other@example.invalid", None);
    let orphan_grip = other.fields(&other_fingerprint, "grp").pop().unwrap();
    source.copy_component(&other.component(&orphan_grip), &orphan_grip);
    let incomplete = fixture.scratch("incomplete-backup.json");
    assert!(!backup(&source, &fixture, &incomplete).status.success());
    assert!(!incomplete.exists());

    let destination = Keyring::new(&fixture, "restore-keyring");
    let owner_grip = source.fields(&owner, "grp").pop().unwrap();
    let existing = destination.copy_component(&source.component(&owner_grip), &owner_grip);
    let before = std::fs::read(&existing).unwrap();
    let metadata = existing.metadata().unwrap();
    let result = destination
        .command(&fixture, &["identity", "restore"])
        .arg(&complete)
        .arg("--age-key-file")
        .arg(recovery_key(&fixture))
        .output()
        .unwrap();
    assert!(!result.status.success());
    let unchanged = std::fs::read(&existing).unwrap() == before;
    assert!(unchanged, "restore changed an existing private component");
    assert_eq!(existing.metadata().unwrap().ino(), metadata.ino());
    assert_eq!(existing.metadata().unwrap().mode(), metadata.mode());
}

#[test]
fn a_different_certificate_sharing_the_encryption_key_is_not_independent_recovery() {
    let fixture = Fixture::new();
    let source = Keyring::new(&fixture, "source-keyring");
    let owner = source.generate("owner@example.invalid", None);
    let encryption_grip = source.fields(&owner, "grp").pop().unwrap();
    let builder = Keyring::new(&fixture, "alias-builder");
    builder.gpg(
        &["--import"],
        &source.gpg(&["--export-secret-keys", &owner], &[]),
    );
    let alias = builder.generate("alias@example.invalid", Some(&encryption_grip));
    assert_ne!(owner, alias);
    let recovery = Keyring::new(&fixture, "recovery-keyring");
    recovery.gpg(
        &["--import"],
        &builder.gpg(&["--export-secret-keys", &alias], &[]),
    );
    assert!(recovery.fields(&alias, "grp").contains(&encryption_grip));

    let destination = fixture.scratch("circular-backup.json");
    let result = source
        .command(&fixture, &["identity", "backup", "pgp"])
        .arg(&destination)
        .args(["--recipient", &format!("pgp:{alias}"), "--gnupg-home"])
        .arg(&recovery.0)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!destination.exists());
}

/// A mode-700 directory holding the identities age-keygen mints into it, in
/// order, as one identity file; returns the file and each identity's recipient.
fn age_identities(fixture: &Fixture, name: &str, hybrid: &[bool]) -> (PathBuf, Vec<String>) {
    let directory = fixture.scratch_dir(name);
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    let file = directory.join("keys.txt");
    let mut joined = Vec::new();
    let mut recipients = Vec::new();
    for (index, pq) in hybrid.iter().enumerate() {
        let one = directory.join(format!("minted-{index}.txt"));
        let mut command = Command::new("age-keygen");
        if *pq {
            command.arg("-pq");
        }
        assert!(command.arg("-o").arg(&one).status().unwrap().success());
        let derived = Command::new("age-keygen")
            .arg("-y")
            .arg(&one)
            .output()
            .unwrap();
        recipients.push(String::from_utf8(derived.stdout).unwrap().trim().to_owned());
        joined.extend(std::fs::read(&one).unwrap());
        std::fs::remove_file(&one).unwrap();
    }
    std::fs::write(&file, joined).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    (file, recipients)
}

fn sops(arguments: &[&str], key_file: &Path) -> Output {
    Command::new(harness::real_sops())
        .args(arguments)
        .env("SOPS_AGE_KEY_FILE", key_file)
        .output()
        .unwrap()
}

#[test]
fn an_identity_file_holding_a_hybrid_identity_is_backed_up_and_restored() {
    let fixture = Fixture::new();
    let (source, own) = age_identities(&fixture, "source-identities", &[false, true]);
    let [classic, hybrid]: [String; 2] = own.try_into().unwrap();
    assert!(classic.starts_with("age1") && !classic.starts_with("age1pq1"));
    assert!(hybrid.starts_with("age1pq1"));
    let (recovery, recovery_recipients) = age_identities(&fixture, "recovery-identity", &[true]);
    let [recovery_recipient]: [String; 1] = recovery_recipients.try_into().unwrap();
    assert!(recovery_recipient.starts_with("age1pq1"));

    let backup = fixture.scratch("hybrid-backup.sops");
    let taken = fixture
        .command(&["identity", "backup", "age"])
        .arg(&backup)
        .args(["--recipient", &recovery_recipient, "--age-key-file"])
        .arg(&recovery)
        .env("SAFIX_AGE_KEY_FILE", &source)
        .output()
        .unwrap();
    assert!(
        taken.status.success(),
        "an identity file holding a hybrid identity was not backed up: {}",
        String::from_utf8_lossy(&taken.stderr)
    );

    let opened = sops(
        &[
            "decrypt",
            "--input-type",
            "binary",
            "--output-type",
            "binary",
            backup.to_str().unwrap(),
        ],
        &recovery,
    );
    assert!(
        opened.status.success(),
        "the recovery identity cannot open the backup"
    );
    let package: serde_json::Value = serde_json::from_slice(&opened.stdout).unwrap();
    let mut roster: Vec<String> = package
        .get("publicRecipients")
        .and_then(serde_json::Value::as_array)
        .expect("the backup names its recipients")
        .iter()
        .map(|recipient| recipient.as_str().unwrap().to_owned())
        .collect();
    roster.sort();
    let mut expected = vec![classic, hybrid.clone()];
    expected.sort();
    assert_eq!(
        roster, expected,
        "the backup's roster is not both identities"
    );

    let empty = fixture.scratch_dir("restored-identities");
    std::fs::set_permissions(&empty, std::fs::Permissions::from_mode(0o700)).unwrap();
    let restored = empty.join("keys.txt");
    let result = fixture
        .command(&["identity", "restore"])
        .arg(&backup)
        .arg("--age-key-file")
        .arg(&recovery)
        .env("SAFIX_AGE_KEY_FILE", &restored)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "the hybrid backup was not restored: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let plaintext = fixture.scratch("hybrid-only.plain.yaml");
    std::fs::write(&plaintext, "value: CANARY-hybrid\n").unwrap();
    let wrapped = sops(
        &[
            "--encrypt",
            "--config",
            "/dev/null",
            "--age",
            &hybrid,
            "--input-type",
            "yaml",
            "--output-type",
            "yaml",
            plaintext.to_str().unwrap(),
        ],
        &recovery,
    );
    assert!(
        wrapped.status.success(),
        "sops could not wrap to the hybrid recipient: {}",
        String::from_utf8_lossy(&wrapped.stderr)
    );
    let ciphertext = fixture.scratch("hybrid-only.yaml");
    std::fs::write(&ciphertext, wrapped.stdout).unwrap();
    let read = sops(
        &[
            "decrypt",
            "--input-type",
            "yaml",
            "--output-type",
            "yaml",
            ciphertext.to_str().unwrap(),
        ],
        &restored,
    );
    assert!(
        read.status.success(),
        "the restored identity file cannot open a file wrapped to the hybrid recipient alone"
    );
    assert_eq!(
        String::from_utf8(read.stdout).unwrap(),
        "value: CANARY-hybrid\n"
    );
}
