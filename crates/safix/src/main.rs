#![forbid(unsafe_code)]

//! The safix command.
//!
//! This binary is the thin edge of [`safix_core`]: argument parsing, operator
//! interaction, and the rendering of refusals. No decision about custody,
//! drift, ordering or writing is made here.
//!
//! # What this binary does today
//!
//! The read paths — `list`, `get` and `check` — and it refuses the rest. The
//! runtime is being ported from `modules/flake/safix/safix.sh` one subcommand
//! at a time, and a subcommand appears here only once a differential harness
//! has compared it against the shell runtime on standard output, standard
//! error, exit code and effect on the repository. Until every subcommand has
//! passed, the flake's `safix` package builds the shell script and this binary
//! ships beside it as `safix-rs`.
//!
//! # Exit codes
//!
//! Zero on success, one on a refusal, and — for `get` alone — whatever sops
//! exited with, because the shell runtime ends `get` with the sops invocation
//! itself and its status is therefore the command's.

mod render;
mod reporter;
mod table;
mod usage;

use std::io::Write;
use std::process::ExitCode;

use safix_core::{Error, Workspace, check};

use reporter::Refusal;

/// The subcommands the shell runtime has and this binary has not reached.
const NOT_PORTED: [&str; 5] = ["set", "generate", "fix", "keygen", "adduser"];

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match run(&arguments) {
        Ok(code) => code,
        Err(refusal) => {
            reporter::report(refusal);
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
        "check" => check_command(rest),
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
        "get" => usage::GET,
        "list" => usage::LIST,
        "check" => usage::CHECK,
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

    Ok(ExitCode::from(u8::try_from(decrypted.status).unwrap_or(1)))
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
