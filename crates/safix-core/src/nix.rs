//! The nix driver.
//!
//! The nix half of safix is the consumer-facing option surface, and the
//! runtime's only view of it is `nix eval`. Nothing here interprets a
//! declaration: it evaluates one attribute of `flake.safix.lib` and hands the
//! bytes to serde, so the resolver's answers stay the resolver's.
//!
//! The attribute name is part of the contract with the nix half and with the
//! hermetic checks, whose stubbed `nix` dispatches on it. Naming each one in
//! [`Attribute`] rather than passing a string means a rename fails to compile
//! here and fails a check there, rather than at an operator's terminal.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::de::DeserializeOwned;

use crate::error::{Error, Result};
use crate::sandbox::{self, Confinement};

/// An attribute of `flake.safix.lib` the runtime reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribute {
    /// `user -> name -> placement`.
    Placements,
    /// `file -> who can open it`.
    Audiences,
    /// Which files the recipient policy governs.
    GovernedFiles,
    /// `user -> every age key that user can open a file with`.
    Recipients,
    /// Who may scaffold for whom, and over which groups.
    Delegation,
    /// The recipient policy these declarations imply, as text.
    PolicyText,
    /// `user -> what may run, in which order, reading and writing what`.
    GeneratorPlan,
    /// The alphabet a declared name is drawn from, as an unanchored pattern.
    NameRegex,
    /// The declared bridge: the clan flake, and every mapping under it.
    Bridge,
    /// The declared mirror: the database, the group, and every mapping under it.
    Keepassxc,
    /// The consumer's onboarding invocation, or null when none is configured.
    OnboardingHook,
    /// The consumer's enrollment invocation, or null when none is configured.
    EnrollHook,
}

impl Attribute {
    /// The attribute path under the flake, as `nix eval` takes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Placements => "safix.lib.placements",
            Self::Audiences => "safix.lib.audiences",
            Self::GovernedFiles => "safix.lib.governedFiles",
            Self::Recipients => "safix.lib.recipients",
            Self::Delegation => "safix.lib.delegation",
            Self::PolicyText => "safix.lib.policyText",
            Self::GeneratorPlan => "safix.lib.generatorPlan",
            Self::NameRegex => "safix.lib.nameRegex",
            Self::Bridge => "safix.lib.bridge",
            Self::Keepassxc => "safix.lib.keepassxc",
            Self::OnboardingHook => "safix.onboardingHook",
            Self::EnrollHook => "safix.enrollHook",
        }
    }

    /// How a refusal names this attribute, which is how a consumer declares it.
    #[must_use]
    pub const fn declared_as(self) -> &'static str {
        match self {
            Self::Placements => "flake.safix.lib.placements",
            Self::Audiences => "flake.safix.lib.audiences",
            Self::GovernedFiles => "flake.safix.lib.governedFiles",
            Self::Recipients => "flake.safix.lib.recipients",
            Self::Delegation => "flake.safix.lib.delegation",
            Self::PolicyText => "flake.safix.lib.policyText",
            Self::GeneratorPlan => "flake.safix.lib.generatorPlan",
            Self::NameRegex => "flake.safix.lib.nameRegex",
            Self::Bridge => "flake.safix.lib.bridge",
            Self::Keepassxc => "flake.safix.lib.keepassxc",
            Self::OnboardingHook => "flake.safix.onboardingHook",
            Self::EnrollHook => "flake.safix.enrollHook",
        }
    }
}

/// The nix binary, and how it is reached.
///
/// `SAFIX_NIX` overrides the program so that a hermetic check can drive the
/// runtime against a fixture evaluation: a flake evaluation is the one thing a
/// build sandbox cannot do.
#[derive(Debug, Clone)]
pub struct Nix {
    program: PathBuf,
    /// The plain file `--entry` or `SAFIX_ENTRY` named, when either did.
    ///
    /// Set, every evaluation targets `--file <entry> <attribute>` instead of
    /// `<root>#<attribute>` — see [`Nix::target`]. This changes only how the
    /// target is built; [`Attribute::as_str`] supplies the same string either
    /// way.
    entry: Option<PathBuf>,
    /// The flake reference `--nixpkgs` or `SAFIX_NIXPKGS` declared, when
    /// either did.
    ///
    /// Read by [`Nix::shell`], and only when [`entry`](Self::entry) is also
    /// set: flake mode already resolves the sandbox's tools through
    /// `--inputs-from`, so a declared reference with no `--entry` is ignored
    /// rather than consulted.
    nixpkgs: Option<String>,
}

impl Default for Nix {
    fn default() -> Self {
        Self::from_environment()
    }
}

impl Nix {
    /// The binary `SAFIX_NIX` names, or `nix`; the entry file `SAFIX_ENTRY`
    /// names, if any; and the nixpkgs reference `SAFIX_NIXPKGS` names, if any.
    #[must_use]
    pub fn from_environment() -> Self {
        Self {
            program: std::env::var_os("SAFIX_NIX")
                .map_or_else(|| PathBuf::from("nix"), PathBuf::from),
            entry: std::env::var_os("SAFIX_ENTRY").map(PathBuf::from),
            nixpkgs: std::env::var("SAFIX_NIXPKGS").ok(),
        }
    }

    /// Point every evaluation at a plain file instead of `<root>#<attribute>`.
    ///
    /// Takes the value rather than reading `--entry` itself, because the
    /// command parses it after already building a `Nix` from the
    /// environment; calling this is what lets `--entry` override
    /// `SAFIX_ENTRY` rather than merely duplicate it.
    #[must_use]
    pub fn with_entry(mut self, entry: PathBuf) -> Self {
        self.entry = Some(entry);
        self
    }

    /// Declare the nixpkgs reference `generate`'s sandbox resolves its tools
    /// against, overriding `SAFIX_NIXPKGS` the same way [`Nix::with_entry`]
    /// overrides `SAFIX_ENTRY`.
    #[must_use]
    pub fn with_nixpkgs(mut self, nixpkgs: String) -> Self {
        self.nixpkgs = Some(nixpkgs);
        self
    }

    /// Whether `--entry` or `SAFIX_ENTRY` named a plain file to evaluate.
    #[must_use]
    pub fn entry(&self) -> Option<&Path> {
        self.entry.as_deref()
    }

    /// The nixpkgs reference `--nixpkgs` or `SAFIX_NIXPKGS` declared, if any.
    #[must_use]
    pub fn nixpkgs(&self) -> Option<&str> {
        self.nixpkgs.as_deref()
    }

    /// The arguments naming what to evaluate: a plain file when `--entry` or
    /// `SAFIX_ENTRY` named one, the repository's flake output otherwise.
    ///
    /// Two arguments where a flake target is one — `--file <entry>` ahead of
    /// the attribute — but [`Attribute::as_str`] supplies the same string in
    /// the last position either way, so the twelve attribute spellings this
    /// runtime reads are unchanged by which form this returns.
    fn target(&self, root: &Path, attribute: Attribute) -> Vec<OsString> {
        if let Some(entry) = &self.entry {
            vec![
                OsString::from("--file"),
                entry.as_os_str().to_owned(),
                OsString::from(attribute.as_str()),
            ]
        } else {
            let mut target = OsString::from(root);
            target.push("#");
            target.push(attribute.as_str());
            vec![target]
        }
    }

    /// Evaluate one attribute and deserialize its JSON.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] when nix cannot be run or exits non-zero, and
    /// [`Error::NixSchemaMismatch`] when what it printed is not the shape this
    /// runtime reads. The two are separate because they have different
    /// remedies: one is a broken evaluation, the other a nix half that has
    /// moved without this one.
    pub fn eval_json<T: DeserializeOwned>(&self, root: &Path, attribute: Attribute) -> Result<T> {
        let output = self.eval(root, attribute, "--json")?;
        serde_json::from_slice(&output).map_err(|cause| Error::NixSchemaMismatch {
            attribute: attribute.declared_as(),
            cause: cause.to_string(),
        })
    }

    /// Evaluate one attribute whose value is a string, taking it verbatim.
    ///
    /// # Errors
    ///
    /// [`Error::NixEvalFailed`] when nix cannot be run or exits non-zero, and
    /// [`Error::NixSchemaMismatch`] when the bytes are not text.
    pub fn eval_raw(&self, root: &Path, attribute: Attribute) -> Result<String> {
        let output = self.eval(root, attribute, "--raw")?;
        String::from_utf8(output).map_err(|cause| Error::NixSchemaMismatch {
            attribute: attribute.declared_as(),
            cause: cause.to_string(),
        })
    }

    /// Evaluate one attribute whose value is a string, straight into a file.
    ///
    /// The file is created and truncated before nix runs and is left behind when
    /// nix fails, which is what the shell runtime's `>` redirection does. The
    /// caller renames it into place on success, so a failed evaluation never
    /// half-writes the file it was going to replace.
    ///
    /// # Errors
    ///
    /// [`Error::FileUnwritable`] when the destination cannot be created, and
    /// [`Error::NixEvalFailed`] when nix cannot be run or exits non-zero.
    pub fn eval_raw_to(&self, root: &Path, attribute: Attribute, destination: &Path) -> Result<()> {
        let out = std::fs::File::create(destination).map_err(|cause| Error::FileUnwritable {
            path: destination.display().to_string(),
            cause,
        })?;

        let status = Command::new(&self.program)
            .arg("eval")
            .arg("--raw")
            .args(self.target(root, attribute))
            .stdin(Stdio::null())
            .stdout(Stdio::from(out))
            .stderr(Stdio::inherit())
            .status()
            .map_err(|cause| Error::NixEvalFailed {
                attribute: attribute.declared_as(),
                root: root.display().to_string(),
                cause: Some(cause),
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(Error::NixEvalFailed {
                attribute: attribute.declared_as(),
                root: root.display().to_string(),
                cause: None,
            })
        }
    }

    /// The command that runs one program with nixpkgs attributes on `PATH`, up
    /// to and including the `-c` the program follows.
    ///
    /// `--inputs-from` resolves each `nixpkgs#<attribute>` against this flake's
    /// own locked nixpkgs, which is what makes a generator mint the same value
    /// from the same declaration on every machine. When `--entry` and
    /// `--nixpkgs` are both set, there is no flake to resolve `--inputs-from`
    /// against, so each attribute is instead resolved directly against the
    /// declared reference — the refusal in `generate.rs` is what keeps this
    /// branch from being reached with `--entry` set and no reference declared.
    pub(crate) fn shell(&self, root: &Path, attributes: &[&str]) -> Command {
        let mut command = Command::new(&self.program);
        command.arg("shell");
        let reference = if let (Some(_), Some(nixpkgs)) = (&self.entry, &self.nixpkgs) {
            nixpkgs.clone()
        } else {
            command.arg("--inputs-from").arg(root);
            "nixpkgs".to_owned()
        };
        for attribute in attributes {
            command.arg(format!("{reference}#{attribute}"));
        }
        command.arg("-c");
        command
    }

    /// The command that runs one shell fragment inside the envelope, with a
    /// generator's declared tools on `PATH`.
    ///
    /// The tools are *prepended* to the caller's `PATH` rather than replacing
    /// it, which is why `runtimeInputs` has to name every tool the fragment
    /// runs: what the envelope leaves reachable is the store, so a tool the
    /// caller's `PATH` names through anything else — `/usr/bin`, a profile's
    /// symlink tree — is not there. The envelope's own words come first and the
    /// fragment's shell is the one they resolve, so the confinement is
    /// established before the fragment's first byte runs.
    ///
    /// Handed back rather than run, because how the three streams are connected
    /// differs between minting a value and judging one, and neither belongs to
    /// this driver.
    #[must_use]
    pub fn generator_shell(
        &self,
        root: &Path,
        runtime_inputs: &[String],
        script: &str,
        confinement: &Confinement,
    ) -> Command {
        let attributes: Vec<&str> = confinement
            .tools
            .iter()
            .copied()
            .chain(runtime_inputs.iter().map(String::as_str))
            .collect();
        let mut command = self.shell(root, &attributes);
        for word in &confinement.words {
            command.arg(word);
        }
        command
            .arg(sandbox::SHELL)
            .arg("-euo")
            .arg("pipefail")
            .arg("-c")
            .arg(script);
        command
    }

    /// Whether nix can parse this file at all.
    ///
    /// `nix-instantiate --parse` rather than an evaluation: the question is
    /// whether the bytes are a nix expression, and a scaffold that is not one
    /// would be committed beside a regenerated policy and found at the next
    /// evaluation, with the recipient policy already moved.
    ///
    /// Not routed through `SAFIX_NIX`, because that variable names the `nix`
    /// binary and this is a different one; a nix that cannot be found is a file
    /// that cannot be shown to parse, which is the same answer.
    #[must_use]
    pub fn parses(&self, path: &Path) -> bool {
        Command::new("nix-instantiate")
            .arg("--parse")
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    /// Standard error is inherited: nix's own diagnosis of a broken
    /// declaration is the useful half of the failure, and a refusal of ours
    /// that swallowed it would leave the operator with "could not evaluate".
    fn eval(&self, root: &Path, attribute: Attribute, format: &str) -> Result<Vec<u8>> {
        let output = Command::new(&self.program)
            .arg("eval")
            .arg(format)
            .args(self.target(root, attribute))
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|cause| Error::NixEvalFailed {
                attribute: attribute.declared_as(),
                root: root.display().to_string(),
                cause: Some(cause),
            })?;

        if !output.status.success() {
            return Err(Error::NixEvalFailed {
                attribute: attribute.declared_as(),
                root: root.display().to_string(),
                cause: None,
            });
        }
        Ok(output.stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D4: `--entry` changes only how the target is built, never the twelve
    /// attribute spellings — [`Attribute::as_str`] supplies the identical
    /// string either way.
    #[test]
    fn target_reads_a_plain_file_when_entry_is_set() {
        let nix = Nix::from_environment().with_entry(PathBuf::from("/fixture/entry.nix"));
        assert_eq!(
            nix.target(Path::new("/repo"), Attribute::GeneratorPlan),
            vec![
                OsString::from("--file"),
                OsString::from("/fixture/entry.nix"),
                OsString::from(Attribute::GeneratorPlan.as_str()),
            ]
        );
    }

    #[test]
    fn target_reads_the_flake_output_when_entry_is_not_set() {
        let nix = Nix {
            program: PathBuf::from("nix"),
            entry: None,
            nixpkgs: None,
        };
        assert_eq!(
            nix.target(Path::new("/repo"), Attribute::GeneratorPlan),
            vec![OsString::from(format!(
                "/repo#{}",
                Attribute::GeneratorPlan.as_str()
            ))]
        );
    }

    /// `--entry` overrides `SAFIX_ENTRY`: [`Nix::with_entry`] is applied after
    /// [`Nix::from_environment`] already read the variable, so the value it
    /// sets is the one [`Nix::target`] reads.
    #[test]
    fn with_entry_overrides_whatever_from_environment_already_read() {
        let nix = Nix {
            program: PathBuf::from("nix"),
            entry: Some(PathBuf::from("/env/entry.nix")),
            nixpkgs: None,
        }
        .with_entry(PathBuf::from("/cli/entry.nix"));
        assert_eq!(nix.entry(), Some(Path::new("/cli/entry.nix")));
    }

    /// D5's other half: `shell()` resolves directly against the declared
    /// reference only when both `entry` and `nixpkgs` are set; `nixpkgs` alone
    /// is ignored, which is `generate`'s "flake mode is unaffected" scenario.
    #[test]
    fn shell_ignores_a_declared_nixpkgs_reference_without_entry() {
        let nix = Nix {
            program: PathBuf::from("nix"),
            entry: None,
            nixpkgs: Some("github:example/nixpkgs".to_owned()),
        };
        let command = nix.shell(Path::new("/repo"), &["coreutils"]);
        let rendered = format!("{command:?}");
        assert!(rendered.contains("--inputs-from"));
        assert!(rendered.contains("nixpkgs#coreutils"));
        assert!(!rendered.contains("github:example/nixpkgs"));
    }

    #[test]
    fn shell_resolves_against_the_declared_reference_when_entry_and_nixpkgs_are_both_set() {
        let nix = Nix {
            program: PathBuf::from("nix"),
            entry: Some(PathBuf::from("/fixture/entry.nix")),
            nixpkgs: Some("github:example/nixpkgs".to_owned()),
        };
        let command = nix.shell(Path::new("/repo"), &["coreutils"]);
        let rendered = format!("{command:?}");
        assert!(!rendered.contains("--inputs-from"));
        assert!(rendered.contains("github:example/nixpkgs#coreutils"));
    }
}
