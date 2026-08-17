#![forbid(unsafe_code)]

//! The safix command.
//!
//! This binary is the thin edge of [`safix_core`]: argument parsing, operator
//! interaction, and the rendering of refusals. No decision about custody,
//! drift, ordering or writing is made here.
//!
//! # What this binary does
//!
//! Every subcommand the retired shell runtime had: the read paths `list`, `get`
//! and `check`; the write paths `set` and `fix`; the generator graph behind
//! `generate`; and the two that touch custody itself, `keygen` and `adduser`.
//! Each appeared here only once the differential harness had compared it against
//! that runtime on standard output, standard error, exit code and effect on the
//! repository. What each subcommand does now is asserted against literals by
//! `crates/safix/tests/`, driven per mode from `modules/flake/checks/cli.nix`;
//! `CHANGELOG.md` records the retirement and what the comparison never
//! covered.
//!
//! # Exit codes
//!
//! Zero on success and one on a refusal. For `get`, `set` and `fix`, whatever
//! sops exited with when sops is what refused: sops's own standard error has
//! already said why, so nothing of ours is printed over it, and the retired
//! shell runtime let sops's status be the command's on all three as well.
//!
//! git is the exception to that, and deliberately: a git that refuses is a
//! refusal like any other here — exit 1, and a line naming the command that
//! refused — where the shell runtime, running under `set -e`, exited with git's
//! own status and said nothing of its own. git's message names a lock file or a
//! hook rather than the subcommand safix ran, so the line is what makes the
//! failure actionable. `CHANGELOG.md` records that divergence as a decision.
//!
//! An interrupted run exits 130 and a terminated one 143, having swept up
//! whatever it had written but not yet moved into place; see [`abort`].

mod abort;
mod prompt;
mod render;
mod reporter;
mod stream;
mod table;
mod usage;

use std::io::Write;
use std::process::ExitCode;

use safix_core::{
    Error, Progress, Workspace, adduser, audit, bridge, check, edit, enroll, fix, generate, keygen,
    set,
};

use reporter::Refusal;

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

/// One subcommand: the word that selects it, the text `-h` prints for it, and
/// what it runs.
///
/// One table rather than three lists. Dispatch reads it, `-h` reads it, and the
/// refusal for a word that is in none of it names what is in it — so a
/// subcommand cannot be dispatchable and absent from the refusal, or listed in
/// the refusal and unreachable, which is exactly the pair that had drifted:
/// `edit` shipped, dispatched, and was missing from the sentence telling an
/// operator what they could have typed.
struct Verb {
    /// The word that selects it.
    name: &'static str,
    /// What `safix <name> -h` prints.
    help: &'static str,
    /// What it does with the arguments after its own name.
    run: fn(&[String]) -> Result<ExitCode, Refusal>,
}

/// Every subcommand, in the order [`usage::SCAFFOLD`] lists them.
///
/// The order is the operator-facing one — write, read, converge, bridge,
/// custody — and it is the order the unknown-subcommand refusal names them in,
/// because a list in a different order from the help would be a second answer
/// to the question the help already answers.
const VERBS: &[Verb] = &[
    Verb {
        name: "set",
        help: usage::SET,
        run: set_command,
    },
    Verb {
        name: "edit",
        help: usage::EDIT,
        run: edit_command,
    },
    Verb {
        name: "get",
        help: usage::GET,
        run: get,
    },
    Verb {
        name: "list",
        help: usage::LIST,
        run: list,
    },
    Verb {
        name: "generate",
        help: usage::GENERATE,
        run: generate_command,
    },
    Verb {
        name: "check",
        help: usage::CHECK,
        run: check_command,
    },
    Verb {
        name: "fix",
        help: usage::FIX,
        run: fix_command,
    },
    Verb {
        name: "import",
        help: usage::IMPORT,
        run: import_command,
    },
    Verb {
        name: "export",
        help: usage::EXPORT,
        run: export_command,
    },
    Verb {
        name: "audit",
        help: usage::AUDIT,
        run: audit_command,
    },
    Verb {
        name: "keygen",
        help: usage::KEYGEN,
        run: keygen_command,
    },
    Verb {
        name: "adduser",
        help: usage::ADDUSER,
        run: adduser_command,
    },
    Verb {
        name: "enroll",
        help: usage::ENROLL,
        run: enroll_command,
    },
];

/// The subcommand this word selects, if it selects one.
fn verb(name: &str) -> Option<&'static Verb> {
    VERBS.iter().find(|verb| verb.name == name)
}

/// Every subcommand, as the unknown-subcommand refusal names them.
///
/// Derived from [`VERBS`] rather than written out, which is the whole point:
/// the sentence cannot name a subcommand this binary does not have, and cannot
/// omit one it does. The snapshot holding this refusal is therefore a snapshot
/// of the table, and adding a subcommand fails it until the new wording is
/// accepted.
pub(crate) fn expected_verbs() -> String {
    let names: Vec<&str> = VERBS.iter().map(|verb| verb.name).collect();
    match names.split_last() {
        Some((last, [])) => (*last).to_owned(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => String::new(),
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
            return Ok(ExitCode::SUCCESS);
        }
        "--version" => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }

    match verb(subcommand) {
        Some(verb) => (verb.run)(rest),
        None => Err(Refusal::UnknownSubcommand {
            subcommand: subcommand.clone(),
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
    Some(verb(subcommand).map_or(usage::SCAFFOLD, |verb| verb.help))
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

/// One value, typed twice or piped once, written and committed.
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

    let mut source = value_source();
    let status = set::run(&workspace, &Terminal, source.as_mut(), &user, &name)?;
    Ok(abort::exit_code(status))
}

/// Where `set` reads its value from: the person when one is typing, the stream
/// when one is not.
///
/// The fork is the terminal test on standard input and nothing else, which is the
/// branch `clan vars set` takes, so one piece of calling code scripts both
/// commands. Neither side changes the other: a terminal still gets the hidden
/// double prompt, and a pipe gets the bytes it sent with no prompt and no
/// confirmation — see [`stream`] for why dropping the confirmation there is the
/// point rather than a concession.
fn value_source() -> Box<dyn set::ValueSource> {
    if stream::stdin_is_a_terminal() {
        Box::new(prompt::Prompted)
    } else {
        Box::new(stream::Piped)
    }
}

/// One value, opened in the operator's editor, written and committed.
fn edit_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    const FORM: &str = "edit [--allow-disk-staging] [<user>] <name>";
    let mut options = edit::Options::default();
    let mut positional: Vec<String> = Vec::new();
    let mut rest = arguments;

    while let Some((first, tail)) = rest.split_first() {
        match first.as_str() {
            flag if flag == safix_core::staging::ACKNOWLEDGEMENT => {
                options.allow_disk_staging = true;
            }
            option if option.starts_with('-') => {
                return Err(Refusal::UnknownOption {
                    option: option.to_owned(),
                });
            }
            _ => positional.push(first.clone()),
        }
        rest = tail;
    }

    let workspace = Workspace::discover()?;
    let (user, name) = match positional.as_slice() {
        [name] => (workspace.default_user()?, name.clone()),
        [user, name] => (user.clone(), name.clone()),
        _ => return Err(Refusal::Usage { form: FORM }),
    };

    let status = edit::run(&workspace, &Terminal, &user, &name, options)?;
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

/// Every generator with something to mint, or the one that writes a name.
///
/// Both flags are read before the positional arguments and in either order,
/// because `--yes` answers a question `--regenerate` is what raises.
///
/// A flag this verb does not take is refused with the form rather than read as a
/// secret's name, and that matters here more than it reads: `--no-sandbox` is
/// the flag clan offers and safix does not, so an operator reaching for it gets
/// the usage line rather than a refusal about a name nobody declared — and there
/// is nothing it could be spelled as that would suspend the envelope.
fn generate_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    const FORM: &str = "generate [--regenerate] [--yes] [--allow-disk-staging] [<user>] [<name>]";
    let mut options = generate::Options::default();
    let mut rest = arguments;
    while let Some((first, tail)) = rest.split_first() {
        match first.as_str() {
            "--regenerate" => options.regenerate = true,
            "--yes" => options.assume_yes = true,
            flag if flag == safix_core::staging::ACKNOWLEDGEMENT => {
                options.allow_disk_staging = true;
            }
            flag if flag.starts_with("--") => return Err(Refusal::Usage { form: FORM }),
            _ => break,
        }
        rest = tail;
    }

    let workspace = Workspace::discover()?;
    let (user, name) = match rest {
        [] => (workspace.default_user()?, None),
        // The one argument is a user when it names one, and a secret otherwise.
        // A secret whose name is also a person's is reachable by naming both:
        // this is the only subcommand whose single optional argument could be
        // either, because it is the only one that means something with no secret
        // named at all.
        [only] if workspace.placements()?.declares(only) => (only.clone(), None),
        [only] => (workspace.default_user()?, Some(only.clone())),
        [user, name] => (user.clone(), Some(name.clone())),
        _ => return Err(Refusal::Usage { form: FORM }),
    };

    let status = generate::run(
        &workspace,
        &Terminal,
        &mut prompt::Prompted,
        &user,
        name.as_deref(),
        options,
    )?;
    Ok(abort::exit_code(status))
}

/// Declared clan-to-safix mappings, moved into safix.
fn import_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    transfer(arguments, "import [<mapping>]", bridge::import)
}

/// Declared safix-to-clan mappings, moved into clan.
fn export_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    transfer(arguments, "export [<mapping>]", bridge::export)
}

/// The two transfer verbs, which differ in nothing but the direction they act
/// on.
///
/// No mapping named means every mapping of that direction, rather than an
/// `--all` flag. The flag would be the only way to spell "do the thing this verb
/// is for", and a verb whose bare form does nothing is a verb an operator has to
/// remember a flag for; the narrowing case is the one that takes an argument.
fn transfer(
    arguments: &[String],
    form: &'static str,
    act: fn(&Workspace, &dyn Progress, Option<&str>) -> safix_core::Result<bridge::Run>,
) -> Result<ExitCode, Refusal> {
    let only = match arguments {
        [] => None,
        [option] if option.starts_with('-') => {
            return Err(Refusal::UnknownOption {
                option: option.clone(),
            });
        }
        [mapping] => Some(mapping.clone()),
        _ => return Err(Refusal::Usage { form }),
    };

    let workspace = Workspace::discover()?;
    let outcome = act(&workspace, &Terminal, only.as_deref())?;
    eprint!("{}", render::transfer(&outcome));

    Ok(if outcome.refused() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// The bridge report.
///
/// A verb rather than rows in `check`, and the exit codes are `check`'s: zero
/// when every declared mapping's two sides agree, one when any of them does
/// not. The narrowing argument is the transfer verbs' own, so an operator who
/// can name a mapping to `export` can name it here — and to either direction's,
/// because comparing a mapping is the same act whichever way its value moves.
fn audit_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    let only = match arguments {
        [] => None,
        [option] if option.starts_with('-') => {
            return Err(Refusal::UnknownOption {
                option: option.clone(),
            });
        }
        [mapping] => Some(mapping.clone()),
        _ => {
            return Err(Refusal::Usage {
                form: "audit [<mapping>]",
            });
        }
    };

    let workspace = Workspace::discover()?;
    let report = audit::run(&workspace, only.as_deref())?;
    eprint!("{}", render::audit(&report));

    Ok(if report.is_clean() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

/// An age identity for a person who has none.
fn keygen_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    const FORM: &str = "keygen [--for-someone-else] [<user>]";
    let (for_someone_else, rest) = match arguments.split_first() {
        Some((first, tail)) if first == "--for-someone-else" => (true, tail),
        _ => (false, arguments),
    };

    let workspace = Workspace::discover()?;
    let user = match rest {
        [] => workspace.default_user()?,
        [user] => user.clone(),
        _ => return Err(Refusal::Usage { form: FORM }),
    };

    keygen::run(&workspace, &Terminal, &user, for_someone_else)?;
    Ok(ExitCode::SUCCESS)
}

/// Declare a person who holds nothing yet.
///
/// Flags are read in any order and around the two positionals, because `--host`
/// is repeatable and a caller adding a second one should not have to know where
/// the name and the recipient sit.
fn adduser_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    const FORM: &str = "adduser <name> <age-recipient> [--host <hostname>]... [--yes]";
    let mut hosts = Vec::new();
    let mut assume_yes = false;
    let mut positional: Vec<String> = Vec::new();
    let mut rest = arguments;

    while let Some((first, tail)) = rest.split_first() {
        match first.as_str() {
            "--host" => match tail.split_first() {
                Some((host, after)) => {
                    hosts.push(host.clone());
                    rest = after;
                }
                None => return Err(Refusal::HostNeedsHostname),
            },
            "--yes" => {
                assume_yes = true;
                rest = tail;
            }
            "--" => {
                positional.extend(tail.iter().cloned());
                rest = &[];
            }
            option if option.starts_with('-') => {
                return Err(Refusal::UnknownOption {
                    option: option.to_owned(),
                });
            }
            _ => {
                positional.push(first.clone());
                rest = tail;
            }
        }
    }

    let [name, recipient] = positional.as_slice() else {
        return Err(Refusal::Usage { form: FORM });
    };

    let workspace = Workspace::discover()?;
    adduser::run(
        &workspace,
        &Terminal,
        &mut prompt::Prompted,
        &adduser::Request {
            name: name.clone(),
            recipient: recipient.clone(),
            hosts,
            assume_yes,
        },
    )?;
    Ok(ExitCode::SUCCESS)
}

/// One hardware key, from a blank card to a proven recovery identity.
///
/// The flags are read in any order and around the one positional, which is what
/// `adduser` does and for the same reason: a run that has to remember where the
/// person's name sits among six options is a run an operator gets wrong.
///
/// Anything naming an OTP slot is refused by name here rather than as an unknown
/// option, because the operator asking has a hazard to be told about — see
/// [`Error::OtpRefused`]. The list is the spellings somebody would plausibly
/// reach for, and it is a refusal rather than a silent ignore.
fn enroll_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    const FORM: &str = "enroll [<user>] [--serial <n>] [--slot <n>] [--no-store-pin] \
                        [--mirror-to-store] [--store-database <path>] [--pin-policy <p>] \
                        [--touch-policy <p>] [--allow-disk-staging]";
    const OTP_SPELLINGS: [&str; 5] = [
        "--otp",
        "--otp-slot",
        "--challenge-response",
        "--program-otp",
        "--hmac-sha1",
    ];

    let mut options = enroll::Options::default();
    let mut positional: Vec<String> = Vec::new();
    let mut rest = arguments;

    while let Some((first, tail)) = rest.split_first() {
        let mut valued = |sink: &mut String| match tail.split_first() {
            Some((value, after)) => {
                sink.clone_from(value);
                Ok(after)
            }
            None => Err(Refusal::OptionNeedsValue {
                option: first.clone(),
            }),
        };
        match first.as_str() {
            otp if OTP_SPELLINGS.contains(&otp) => return Err(Error::OtpRefused.into()),
            "--serial" => {
                let mut held = String::new();
                rest = valued(&mut held)?;
                options.serial = Some(held);
            }
            "--slot" => {
                let mut held = String::new();
                rest = valued(&mut held)?;
                options.slot = Some(held);
            }
            "--pin-policy" => rest = valued(&mut options.pin_policy)?,
            "--touch-policy" => rest = valued(&mut options.touch_policy)?,
            "--store-database" => {
                let mut held = String::new();
                rest = valued(&mut held)?;
                options.mirror.database = Some(std::path::PathBuf::from(held));
            }
            "--no-store-pin" => {
                options.store_pin = false;
                rest = tail;
            }
            "--mirror-to-store" => {
                options.mirror.mirror = true;
                rest = tail;
            }
            flag if flag == safix_core::staging::ACKNOWLEDGEMENT => {
                options.allow_disk_staging = true;
                rest = tail;
            }
            "--" => {
                positional.extend(tail.iter().cloned());
                rest = &[];
            }
            option if option.starts_with('-') => {
                return Err(Refusal::UnknownOption {
                    option: option.to_owned(),
                });
            }
            _ => {
                positional.push(first.clone());
                rest = tail;
            }
        }
    }

    let workspace = Workspace::discover()?;
    let user = match positional.as_slice() {
        [] => workspace.default_user()?,
        [user] => user.clone(),
        _ => return Err(Refusal::Usage { form: FORM }),
    };

    let outcome = enroll::run(
        &workspace,
        &Terminal,
        &mut prompt::Prompted,
        &user,
        &options,
    )?;
    // Non-zero for an enrollment that did not end, which is what an outstanding
    // proof is. Nothing was undone and the report says so, so this is a status a
    // script can act on rather than a failure to clean up after.
    Ok(if outcome.proven {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

#[cfg(test)]
mod tests {
    use super::{VERBS, usage};

    /// Every subcommand is in the scaffold, in the order the table declares them.
    ///
    /// [`VERBS`] says this in a doc comment and nothing held it to it. A verb
    /// added to the table alone is dispatchable and is named by the
    /// unknown-subcommand refusal — [`expected_verbs`](super::expected_verbs)
    /// derives that sentence from the table — while being absent from the one
    /// page an operator is shown. No snapshot reads both, so every snapshot in
    /// the tree stays green over exactly that drift.
    ///
    /// The order is asserted for the reason the table's own doc gives: a list in
    /// a different order from the help is a second answer to the question the
    /// help already answers.
    #[test]
    fn every_verb_is_in_the_scaffold_in_the_order_the_table_declares_them() {
        let mut previous: Option<(usize, &str)> = None;
        for verb in VERBS {
            let name = verb.name;
            // The listing line rather than the bare word: `fix` and `set` are
            // English, and the scaffold's prose says both before it is done.
            let listing = format!("safix {name}");
            let found = usage::SCAFFOLD.find(&listing);
            assert!(
                found.is_some(),
                "`{name}` is a subcommand and the usage scaffold never lists it"
            );
            let at = found.unwrap();
            if let Some((earlier, earlier_name)) = previous {
                assert!(
                    at > earlier,
                    "the scaffold lists `{name}` before `{earlier_name}` and the table \
                     declares them the other way round"
                );
            }
            previous = Some((at, name));
        }
    }
}
