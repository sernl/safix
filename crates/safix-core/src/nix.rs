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
    /// The recipient policy these declarations imply, as text.
    PolicyText,
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
            Self::PolicyText => "safix.lib.policyText",
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
            Self::PolicyText => "flake.safix.lib.policyText",
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
}

impl Default for Nix {
    fn default() -> Self {
        Self::from_environment()
    }
}

impl Nix {
    /// The binary `SAFIX_NIX` names, or `nix`.
    #[must_use]
    pub fn from_environment() -> Self {
        Self {
            program: std::env::var_os("SAFIX_NIX")
                .map_or_else(|| PathBuf::from("nix"), PathBuf::from),
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

    /// Standard error is inherited: nix's own diagnosis of a broken
    /// declaration is the useful half of the failure, and a refusal of ours
    /// that swallowed it would leave the operator with "could not evaluate".
    fn eval(&self, root: &Path, attribute: Attribute, format: &str) -> Result<Vec<u8>> {
        let mut target = OsString::from(root);
        target.push("#");
        target.push(attribute.as_str());

        let output = Command::new(&self.program)
            .arg("eval")
            .arg(format)
            .arg(&target)
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
