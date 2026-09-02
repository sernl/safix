//! The clan command, and the whole of how safix reaches clan's store.
//!
//! clan owns its store. Nothing here reads, writes, encrypts, decrypts or
//! parses a file clan placed: every value crosses the boundary through clan's
//! own command, on a pipe, in both directions. The consumer's backend — `age`,
//! `sops`, `password-store`, or whatever clan adds next — is a choice clan
//! owns, and a driver that reached past the command would silently support one
//! of them and quietly corrupt the rest.
//!
//! # The two contracts
//!
//! Read is `clan vars get <machine> <generator>/<file>`, which writes the value
//! to its standard output. It writes the raw bytes when that output is not a
//! terminal and a *printable* rendering when it is, so the pipe below is
//! load-bearing rather than incidental: a `get` inheriting a terminal would
//! hand back a rendering of the value in place of the value. The pipe is
//! established by the fixture rather than assumed — the stub records whether
//! its standard output was a terminal, and the test asserts it was not.
//!
//! Write is `clan vars set <machine> <generator>/<file>`, which reads the value
//! from its standard input. It prompts when that input is a terminal, so the
//! pipe is load-bearing there too: a `set` inheriting a terminal would hang
//! waiting for a person.
//!
//! Both contracts were read out of clan-cli rather than out of its
//! documentation: `clan_cli/vars/get.py` for the `isatty` branch on output,
//! `clan_lib/vars/set.py` for the `isatty` branch on input.
//!
//! # What travels where
//!
//! The machine, the generator and the file travel in argv, which is public and
//! is what clan's own command line takes. The value travels on a pipe and
//! appears in no argument vector and no environment, in either direction.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::secret::Secret;

/// The line clan writes when a var it knows about holds no value yet.
///
/// Matched as a substring of clan's own standard error, which is a coupling to
/// its wording and is deliberate. The alternative is treating an ungenerated
/// var as a failure, and it is not one: a clan var that has not been generated
/// is a normal state during bootstrap, and a bridge run then should say so and
/// carry on rather than stop.
const NOT_GENERATED: &str = "has not been generated yet";

/// The line clan writes when the var id names nothing it has.
///
/// Matched for the same reason and with the same coupling: this one has a
/// remedy the others do not, which is that the triple in the declaration is
/// wrong and the refusal can say which three names were tried.
const NO_SUCH_VAR: &str = "Couldn't find var";

/// The line clan writes about a generator whose recorded validation no longer
/// matches its definition.
///
/// Matched as a substring for the reason the two above are, and with one more:
/// clan's exit status alone cannot answer this question. `clan vars check`
/// exits non-zero for a missing var and for a secret needing re-encryption as
/// well as for a stale generator, and the first of those is the ordinary state
/// of a var about to be exported into for the first time. Treating the status
/// as the answer would refuse every first export.
///
/// The sentence is emitted per generator by `clan_lib/vars/check.py`, at the
/// default log level, on standard error.
const OUTDATED_VALIDATION: &str = "outdated invalidation hash";

/// The environment variable that replaces the program, for checks.
///
/// Mirrors `SAFIX_SOPS`. A hermetic check drives the runtime against a stub
/// without a real clan on `PATH` meaning something different from the one the
/// package pins.
pub const PROGRAM_OVERRIDE: &str = "SAFIX_CLAN";

/// The clan command, and the flake it is pointed at.
#[derive(Debug, Clone)]
pub struct Clan {
    program: PathBuf,
    flake: String,
}

/// What a read of the clan side found.
///
/// No `Debug`: the present arm holds a value, and deriving it would print that
/// value through the field.
pub enum Reading {
    /// clan had a value, and this is it.
    Present(Secret),
    /// clan knows the var and it holds nothing yet. An outcome rather than a
    /// failure — see `NOT_GENERATED`.
    AbsentAtSource,
}

impl Clan {
    /// The command as this run will invoke it.
    #[must_use]
    pub fn new(flake: String) -> Self {
        let program = std::env::var_os(PROGRAM_OVERRIDE)
            .unwrap_or_else(|| OsString::from("clan"))
            .into();
        Self { program, flake }
    }

    /// The program name, for a refusal that has to name it.
    #[must_use]
    pub fn program(&self) -> String {
        self.program.display().to_string()
    }

    /// Establish that clan can be run at all, before anything is transferred.
    ///
    /// Called once per run rather than per mapping, and before the first
    /// mapping is touched. A run that discovered the absence partway through
    /// would have already reported "unchanged" for the mappings it had not
    /// reached, and a report that says "unchanged" about a side it never looked
    /// at is worse than no report.
    ///
    /// # Errors
    ///
    /// [`Error::ClanUnavailable`] when the binary cannot be run.
    pub fn probe(&self) -> Result<()> {
        Command::new(&self.program)
            .arg("--help")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|cause| Error::ClanUnavailable {
                program: self.program(),
                cause,
            })?;
        Ok(())
    }

    /// The var id as clan's own command line spells it.
    #[must_use]
    pub fn var_id(generator: &str, file: &str) -> String {
        format!("{generator}/{file}")
    }

    /// One clan value, read onto a pipe.
    ///
    /// # Errors
    ///
    /// [`Error::ClanUnavailable`] when the binary cannot be run,
    /// [`Error::ClanPipeMissing`] when it was started with a pipe that was not
    /// there to read, [`Error::ClanVarUnknown`] when clan has no such var, and
    /// [`Error::ClanCommandFailed`] carrying clan's own message for every other
    /// refusal.
    pub fn read(
        &self,
        mapping: &str,
        machine: &str,
        generator: &str,
        file: &str,
    ) -> Result<Reading> {
        let id = Self::var_id(generator, file);
        let mut child = Command::new(&self.program)
            .arg("vars")
            .arg("get")
            .arg("--flake")
            .arg(&self.flake)
            .arg(machine)
            .arg(&id)
            .stdin(Stdio::null())
            // Piped rather than inherited, and that is the contract: clan
            // writes a printable rendering in place of the value when this is a
            // terminal.
            .stdout(Stdio::piped())
            // Captured rather than inherited, because two lines of it are
            // outcomes this runtime distinguishes. The rest is carried into the
            // refusal verbatim, so clan's own message reaches the operator.
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|cause| Error::ClanUnavailable {
                program: self.program(),
                cause,
            })?;

        let value = {
            let mut stdout = child.stdout.take().ok_or(Error::ClanPipeMissing)?;
            Secret::read_from(&mut stdout)?
        };

        let finished = child
            .wait_with_output()
            .map_err(|cause| Error::ClanUnavailable {
                program: self.program(),
                cause,
            })?;

        if finished.status.success() {
            return Ok(Reading::Present(value));
        }

        let complaint = String::from_utf8_lossy(&finished.stderr);
        if complaint.contains(NOT_GENERATED) {
            return Ok(Reading::AbsentAtSource);
        }
        if complaint.contains(NO_SUCH_VAR) {
            return Err(Error::ClanVarUnknown {
                mapping: mapping.to_owned(),
                machine: machine.to_owned(),
                generator: generator.to_owned(),
                file: file.to_owned(),
            });
        }
        Err(Error::ClanCommandFailed {
            mapping: mapping.to_owned(),
            machine: machine.to_owned(),
            var_id: id,
            output: trimmed(&complaint),
        })
    }

    /// Register one age recipient with clan, as one person's key.
    ///
    /// Two commands rather than one, and the second is reached by outcome rather
    /// than by reading clan's wording: `users add` declares a person clan does not
    /// have, `users add-key` adds a key to one it does, and which of those applies
    /// is a fact about clan's store that only clan knows. So the first is tried
    /// and the second follows it when it refuses, and only both refusing is a
    /// refusal here.
    ///
    /// Nothing on this path reads or writes a file clan placed. A card's
    /// recipient is a public string and clan's own command is what puts it in
    /// clan's store, which is this module's whole rule applied to a key instead of
    /// to a value.
    ///
    /// # Errors
    ///
    /// [`Error::ClanUnavailable`] when the binary cannot be run, and
    /// [`Error::ClanUserRegistrationFailed`] carrying clan's own message when both
    /// commands refuse.
    pub fn register_user(&self, user: &str, recipient: &str) -> Result<()> {
        let mut said = String::new();
        for verb in ["add", "add-key"] {
            let finished = Command::new(&self.program)
                .arg("secrets")
                .arg("users")
                .arg(verb)
                .arg("--flake")
                .arg(&self.flake)
                .arg(user)
                .arg("--age-key")
                .arg(recipient)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .map_err(|cause| Error::ClanUnavailable {
                    program: self.program(),
                    cause,
                })?;
            if finished.status.success() {
                return Ok(());
            }
            said.push_str(&String::from_utf8_lossy(&finished.stderr));
        }
        Err(Error::ClanUserRegistrationFailed {
            user: user.to_owned(),
            output: trimmed(&said),
        })
    }

    /// Whether clan considers this generator's recorded validation stale.
    ///
    /// clan records a validation hash per generator and regenerates when the
    /// recorded one no longer matches the definition's. An export into a
    /// generator in that state writes a value clan's next routine generation
    /// replaces, silently, so `export` refuses rather than writes.
    ///
    /// The comparison is clan's own and is asked for rather than made here.
    /// Nothing in this runtime reads the recorded hash, computes one, or writes
    /// one: the recorded hash is a file in clan's store and reading it is the
    /// thing this module exists not to do, and the hash it would be compared
    /// against is a function of clan's definition, which safix would then be
    /// evaluating clan's nix to obtain.
    ///
    /// A clan that cannot answer — because the machine does not exist, because
    /// the generator does not, or because it refused for any other reason — is
    /// reported as not stale rather than as stale. That is deliberate: the
    /// caller's next act is a `get` or a `set` against the same triple, and
    /// those produce clan's own refusal naming what is wrong. Answering "stale"
    /// here would replace an accurate refusal with a misleading one.
    ///
    /// # Errors
    ///
    /// [`Error::ClanUnavailable`] when the binary cannot be run.
    pub fn generator_stale(&self, machine: &str, generator: &str) -> Result<bool> {
        let finished = Command::new(&self.program)
            .arg("vars")
            .arg("check")
            .arg("--flake")
            .arg(&self.flake)
            .arg(machine)
            .arg("--generator")
            .arg(generator)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|cause| Error::ClanUnavailable {
                program: self.program(),
                cause,
            })?;

        if finished.status.success() {
            return Ok(false);
        }
        Ok(String::from_utf8_lossy(&finished.stderr).contains(OUTDATED_VALIDATION))
    }

    /// Every machine name clan's own fleet declares, sorted.
    ///
    /// Used only to discover the machine that addresses a shared mapping: a
    /// caller tries each name in turn against [`Self::read`]/[`Self::write`]
    /// until one resolves, rather than building a second copy of clan's own
    /// registry.
    ///
    /// # Errors
    ///
    /// [`Error::ClanUnavailable`] when the binary cannot be run, and
    /// [`Error::ClanMachinesListFailed`] carrying clan's own message when it
    /// refuses.
    pub fn machines(&self) -> Result<Vec<String>> {
        let finished = Command::new(&self.program)
            .arg("machines")
            .arg("list")
            .arg("--flake")
            .arg(&self.flake)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|cause| Error::ClanUnavailable {
                program: self.program(),
                cause,
            })?;

        if !finished.status.success() {
            return Err(Error::ClanMachinesListFailed {
                output: trimmed(&String::from_utf8_lossy(&finished.stderr)),
            });
        }

        Ok(String::from_utf8_lossy(&finished.stdout)
            .lines()
            .map(str::to_owned)
            .collect())
    }

    /// One value written into clan, on a pipe.
    ///
    /// # Errors
    ///
    /// [`Error::ClanUnavailable`] when the binary cannot be run,
    /// [`Error::ClanPipeMissing`] when its standard input was not there to
    /// write, and [`Error::ClanCommandFailed`] carrying clan's own message when
    /// it refuses.
    pub fn write(
        &self,
        mapping: &str,
        machine: &str,
        generator: &str,
        file: &str,
        value: &Secret,
    ) -> Result<()> {
        let id = Self::var_id(generator, file);
        let mut child = Command::new(&self.program)
            .arg("vars")
            .arg("set")
            .arg("--flake")
            .arg(&self.flake)
            .arg(machine)
            .arg(&id)
            // Piped rather than inherited, and that is the contract: clan
            // prompts a person when this is a terminal.
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|cause| Error::ClanUnavailable {
                program: self.program(),
                cause,
            })?;

        {
            let mut stdin = child.stdin.take().ok_or(Error::ClanPipeMissing)?;
            value
                .write_to(&mut stdin)
                .and_then(|()| stdin.flush())
                .map_err(|cause| Error::SecretRead { cause })?;
        }

        let finished = child
            .wait_with_output()
            .map_err(|cause| Error::ClanUnavailable {
                program: self.program(),
                cause,
            })?;

        if finished.status.success() {
            return Ok(());
        }

        let complaint = String::from_utf8_lossy(&finished.stderr);
        if complaint.contains(NO_SUCH_VAR) {
            return Err(Error::ClanVarUnknown {
                mapping: mapping.to_owned(),
                machine: machine.to_owned(),
                generator: generator.to_owned(),
                file: file.to_owned(),
            });
        }
        Err(Error::ClanCommandFailed {
            mapping: mapping.to_owned(),
            machine: machine.to_owned(),
            var_id: id,
            output: trimmed(&complaint),
        })
    }
}

fn trimmed(complaint: &str) -> String {
    complaint.strip_suffix('\n').unwrap_or(complaint).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_var_id_is_the_pair_clans_command_line_takes() {
        assert_eq!(Clan::var_id("ntfy", "token"), "ntfy/token");
    }

    #[test]
    fn an_absent_command_is_refused_by_name() {
        let clan = Clan {
            program: PathBuf::from("safix-no-such-clan-command"),
            flake: ".".into(),
        };
        let refusal = clan.probe().expect_err("no such program exists");
        assert!(matches!(refusal, Error::ClanUnavailable { .. }));
    }
}
