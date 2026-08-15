#![forbid(unsafe_code)]

//! The safix command.
//!
//! This binary is the thin edge of [`safix_core`]: argument parsing, operator
//! interaction, and the rendering of refusals. No decision about custody,
//! drift, ordering or writing is made here.
//!
//! # What this binary does today
//!
//! The read paths — `list`, `get` and `check` — and the two write paths that
//! change no declaration: `set`, which writes one value, and `fix`, which
//! converges the policy and the ciphertext onto the declarations. The runtime
//! is being ported from `modules/flake/safix/safix.sh` one subcommand at a
//! time, and a subcommand appears here only once a differential harness has
//! compared it against the shell runtime on standard output, standard error,
//! exit code and effect on the repository. Until every subcommand has passed,
//! the flake's `safix` package builds the shell script and this binary ships
//! beside it as `safix-rs`.
//!
//! # Exit codes
//!
//! Zero on success and one on a refusal. For `get`, `set` and `fix`, whatever
//! sops exited with when sops is what refused: the shell runtime lets sops's
//! status be the command's on all three, and sops's own standard error has
//! already said why, so nothing of ours is printed over it.
//!
//! An interrupted run exits 130 and a terminated one 143, having swept up
//! whatever it had written but not yet moved into place; see [`abort`].

mod abort;
mod prompt;
mod render;
mod reporter;
mod table;
mod usage;

use std::io::Write;
use std::process::ExitCode;

use safix_core::{Error, Progress, Workspace, check, fix, set};

use reporter::Refusal;

/// The subcommands the shell runtime has and this binary has not reached.
const NOT_PORTED: [&str; 3] = ["generate", "keygen", "adduser"];

/// Where a run's commentary and a subprocess's output go.
///
/// The commentary is standard error, which is where the shell runtime puts it:
/// standard output is `get`'s value and `list`'s table, and a progress line
/// mixed into either would break a pipeline.
struct Terminal;

impl Progress for Terminal {
    fn write(&self, text: &str) {
        eprint!("{text}");
    }

    fn write_output(&self, bytes: &[u8]) {
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(bytes);
        let _ = out.flush();
    }
}

fn main() -> ExitCode {
    abort::catch_signals();
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match run(&arguments) {
        Ok(code) => code,
        Err(refusal) => {
            reporter::report(&refusal);
            ExitCode::from(1)
        }
    }
}

fn run(arguments: &[String]) -> Result<ExitCode, Refusal> {
    let Some((subcommand, rest)) = arguments.split_first() else {
        eprint!("{}", usage::SCAFFOLD);
        return Ok(ExitCode::from(1));
    };

    if let Some(text) = help_requested(subcommand, rest) {
        eprint!("{text}");
        return Ok(ExitCode::SUCCESS);
    }

    match subcommand.as_str() {
        "-h" | "--help" | "help" => {
            eprint!("{}", usage::SCAFFOLD);
            Ok(ExitCode::SUCCESS)
        }
        "--version" => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        "list" => list(rest),
        "get" => get(rest),
        "set" => set_command(rest),
        "check" => check_command(rest),
        "fix" => fix_command(rest),
        other if NOT_PORTED.contains(&other) => Err(Refusal::NotPorted {
            subcommand: other.to_owned(),
        }),
        other => Err(Refusal::UnknownSubcommand {
            subcommand: other.to_owned(),
        }),
    }
}

/// Which help text an invocation asks for, if any.
///
/// The whole argument list is scanned rather than only its head, because the
/// shell runtime scans it: `safix list ana -h` explains `list` rather than
/// listing ana's secrets, and an operator who appends `-h` to a command they
/// have already typed gets the explanation.
fn help_requested(subcommand: &str, rest: &[String]) -> Option<&'static str> {
    if !rest
        .iter()
        .any(|argument| argument == "-h" || argument == "--help")
    {
        return None;
    }
    Some(match subcommand {
        "set" => usage::SET,
        "get" => usage::GET,
        "list" => usage::LIST,
        "check" => usage::CHECK,
        "fix" => usage::FIX,
        _ => usage::SCAFFOLD,
    })
}

/// Every name a user holds, and what serves it.
fn list(arguments: &[String]) -> Result<ExitCode, Refusal> {
    let workspace = Workspace::discover()?;
    let user = match arguments {
        [] => workspace.default_user()?,
        [user] => user.clone(),
        _ => {
            return Err(Refusal::Usage {
                form: "list [<user>]",
            });
        }
    };
    let placements = workspace.placements()?;
    let held = placements
        .held_by(&user)
        .ok_or_else(|| Error::UnknownUser {
            user: user.clone(),
            declared: placements.users().map(str::to_owned).collect(),
        })?;

    if held.is_empty() {
        println!("flake.safix.users.{user} holds no secret.");
    } else {
        print!("{}", table::aligned(&render::listing(held)));
    }
    Ok(ExitCode::SUCCESS)
}

/// One key, decrypted to standard output.
///
/// The output is plaintext by design: it is what makes `get` pipeable, and it
/// is why the value travels as a [`safix_core::Secret`] right up to the write —
/// it is zeroed when this returns whether the write succeeded or not.
fn get(arguments: &[String]) -> Result<ExitCode, Refusal> {
    let workspace = Workspace::discover()?;
    let (user, name) = match arguments {
        [name] => (workspace.default_user()?, name.clone()),
        [user, name] => (user.clone(), name.clone()),
        _ => {
            return Err(Refusal::Usage {
                form: "get [<user>] <name>",
            });
        }
    };

    let placement = workspace.resolve(&user, &name)?;
    let path = workspace.absolute(&placement.file);
    if !path.exists() {
        return Err(Error::NoValueYet {
            file: placement.file.clone(),
            name,
            user,
        }
        .into());
    }

    let decrypted = workspace.sops().decrypt_key(&path, &placement.key)?;
    let mut stdout = std::io::stdout().lock();
    decrypted
        .value
        .write_to(&mut stdout)
        .and_then(|()| stdout.flush())
        .map_err(|cause| Error::SecretRead { cause })?;

    Ok(abort::exit_code(decrypted.status))
}

/// One value, typed twice, written and committed.
fn set_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    let workspace = Workspace::discover()?;
    let (user, name) = match arguments {
        [name] => (workspace.default_user()?, name.clone()),
        [user, name] => (user.clone(), name.clone()),
        _ => {
            return Err(Refusal::Usage {
                form: "set [<user>] <name>",
            });
        }
    };

    let status = set::run(&workspace, &Terminal, &mut prompt::Prompted, &user, &name)?;
    Ok(abort::exit_code(status))
}

/// The policy regenerated, and every governed file re-wrapped to it.
fn fix_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    let assume_yes = match arguments {
        [] => false,
        [flag] if flag == "--yes" => true,
        _ => {
            return Err(Refusal::Usage {
                form: "fix [--yes]",
            });
        }
    };
    let workspace = Workspace::discover()?;
    let status = fix::run(&workspace, &Terminal, assume_yes)?;
    Ok(abort::exit_code(status))
}

/// The drift report.
fn check_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    let workspace = Workspace::discover()?;
    let only = match arguments {
        [] => None,
        [user] => Some(user.clone()),
        _ => {
            return Err(Refusal::Usage {
                form: "check [<user>]",
            });
        }
    };
    if let Some(user) = &only {
        workspace.require_user(user)?;
    }

    let findings = check::run(&workspace, only.as_deref())?;
    eprint!("{}", render::report(&findings));
    Ok(if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}
