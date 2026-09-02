#![forbid(unsafe_code)]

//! The safix command.
//!
//! This binary is the thin edge of [`safix_core`]: argument parsing, operator
//! interaction, and the rendering of refusals. No decision about custody,
//! drift, ordering or writing is made here.
//!
//! # What this binary does
//!
//! Every subcommand this binary implements: the read paths `list`, `get`,
//! `check` and `audit`; the write paths `set`, `edit` and `fix`; the generator
//! graph behind `generate`; `sync`, converging the clan bridge and the
//! password-database mirror; and the three that touch custody itself,
//! `keygen`, `adduser` and `enroll`; plus `group`, for editing a group's
//! declared membership.
//! What each subcommand does is asserted against literals by
//! `crates/safix/tests/`, driven per mode from `modules/flake/checks/cli.nix`;
//! the earliest of them were also compared against the retired shell runtime
//! by a differential harness before that runtime was deleted, and
//! `CHANGELOG.md` records that retirement and what the comparison never
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
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::OnceLock;

use safix_core::{
    Error, Progress, Workspace, adduser, audit, bridge, check, edit, enroll, fix, generate, group,
    keygen, model::Direction, nix::Nix, set, sync, upload,
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
        name: "audit",
        help: usage::AUDIT,
        run: audit_command,
    },
    Verb {
        name: "sync",
        help: usage::SYNC,
        run: sync_command,
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
    Verb {
        name: "group",
        help: usage::GROUP,
        run: group_command,
    },
    Verb {
        name: "upload",
        help: usage::UPLOAD,
        run: upload_command,
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
    let (nix, arguments) = parse_globals(arguments)?;
    let _ = NIX.set(nix);

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

/// The nix driver `--entry`, `SAFIX_ENTRY`, `--nixpkgs` and `SAFIX_NIXPKGS`
/// build, and the arguments after the leading global options.
///
/// Read as a prefix rather than scanned for throughout, because these options
/// change how every subcommand evaluates and are not a subcommand's own
/// concern: `safix --entry ./entry.nix list alice` reads naturally where
/// `safix list --entry ./entry.nix alice` would leave `list` guessing whether
/// `--entry` is its own flag. `--entry` overrides `SAFIX_ENTRY` and
/// `--nixpkgs` overrides `SAFIX_NIXPKGS`: [`Nix::from_environment`] reads the
/// environment first, and the builders below apply on top of it only when the
/// flag was actually given.
fn parse_globals(arguments: &[String]) -> Result<(Nix, &[String]), Refusal> {
    let mut nix = Nix::from_environment();
    let mut rest = arguments;
    loop {
        match rest {
            [option, value, tail @ ..] if option == "--entry" => {
                nix = nix.with_entry(PathBuf::from(value));
                rest = tail;
            }
            [option, value, tail @ ..] if option == "--nixpkgs" => {
                nix = nix.with_nixpkgs(value.clone());
                rest = tail;
            }
            [option] if option == "--entry" || option == "--nixpkgs" => {
                return Err(Refusal::OptionNeedsValue {
                    option: option.clone(),
                });
            }
            _ => return Ok((nix, rest)),
        }
    }
}

/// The nix driver [`parse_globals`] built in [`run`], read by every
/// subcommand through [`workspace`] rather than each rebuilding its own.
static NIX: OnceLock<Nix> = OnceLock::new();

/// The workspace `--entry`, `SAFIX_ENTRY`, `--nixpkgs` and `SAFIX_NIXPKGS`
/// name, in place of [`Workspace::discover`]'s environment-only form.
///
/// Every subcommand function calls this rather than [`Workspace::discover`]
/// directly, so `--entry`'s precedence over `SAFIX_ENTRY` is applied
/// uniformly rather than by each subcommand remembering to read it.
fn workspace() -> Result<Workspace, Refusal> {
    Ok(Workspace::discover_with(
        NIX.get().cloned().unwrap_or_default(),
    )?)
}

/// Which help text an invocation asks for, if any.
///
/// The whole argument list is scanned rather than only its head, because the
/// shell runtime scans it: `safix list alice -h` explains `list` rather than
/// listing alice's secrets, and an operator who appends `-h` to a command they
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
    let workspace = workspace()?;
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
    let workspace = workspace()?;
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
    let workspace = workspace()?;
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

    let workspace = workspace()?;
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
    let workspace = workspace()?;
    let status = fix::run(&workspace, &Terminal, assume_yes)?;
    Ok(abort::exit_code(status))
}

/// The drift report.
fn check_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    let workspace = workspace()?;
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

    let workspace = workspace()?;
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

/// The target, mapping names and `--direction` filter `sync` and `audit`
/// share one grammar for.
struct Dispatch {
    /// `clan`, `keepassxc`, or absent for every target.
    target: Option<bridge::Target>,
    /// Every mapping name given after the target, in the order given.
    names: Vec<String>,
    /// `--direction`'s value, when given.
    direction: Option<Direction>,
}

/// Parse `sync`'s and `audit`'s shared dispatch grammar:
/// `[<target>] [<mapping>...] [--direction <value>]`.
///
/// `verb` names which of the two is asking, for the "a mapping name needs a
/// target" refusal's own remedy, which names both forms; `form` is the usage
/// line an unrecognised option or a malformed `--direction` value is refused
/// with.
fn parse_dispatch(
    verb: &'static str,
    form: &'static str,
    arguments: &[String],
) -> Result<Dispatch, Refusal> {
    let mut rest = arguments;
    let target = match rest.first().map(String::as_str) {
        Some("clan") => Some(bridge::Target::Clan),
        Some("keepassxc") => Some(bridge::Target::Keepassxc),
        _ => None,
    };
    if target.is_some()
        && let Some((_, tail)) = rest.split_first()
    {
        rest = tail;
    }

    let mut names = Vec::new();
    let mut direction = None;
    while let Some((first, tail)) = rest.split_first() {
        match first.as_str() {
            "--direction" => {
                let Some((value, after)) = tail.split_first() else {
                    return Err(Refusal::OptionNeedsValue {
                        option: "--direction".to_owned(),
                    });
                };
                direction = Some(match value.as_str() {
                    "clan-to-safix" => Direction::ClanToSafix,
                    "safix-to-clan" => Direction::SafixToClan,
                    _ => return Err(Refusal::Usage { form }),
                });
                rest = after;
            }
            option if option.starts_with('-') => {
                return Err(Refusal::UnknownOption {
                    option: option.to_owned(),
                });
            }
            name if target.is_none() => {
                return Err(Error::MappingNameNeedsTarget {
                    verb,
                    name: name.to_owned(),
                }
                .into());
            }
            name if bridge::RESERVED_MAPPING_WORDS.contains(&name) => {
                return Err(Error::ReservedMappingWord {
                    word: name.to_owned(),
                }
                .into());
            }
            name => {
                names.push(name.to_owned());
                rest = tail;
            }
        }
    }

    if direction.is_some() && target != Some(bridge::Target::Clan) {
        return Err(Error::DirectionOnWrongTarget {
            target: match target {
                Some(bridge::Target::Keepassxc) => "the keepassxc target",
                _ => "every target, with none named",
            },
        }
        .into());
    }

    Ok(Dispatch {
        target,
        names,
        direction,
    })
}

/// The bridge report, over the named target or both, narrowed by mapping
/// names and, for the clan target, `--direction`.
///
/// A verb rather than rows in `check`, and the exit codes are `check`'s: zero
/// when every compared mapping's two sides agree, one when any of them does
/// not. `lingering` entries never move the exit status, on either target.
fn audit_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    const FORM: &str = "audit [clan|keepassxc] [<mapping>...] [--direction <value>]";
    let dispatch = parse_dispatch("audit", FORM, arguments)?;

    let workspace = workspace()?;
    let report = audit::run(
        &workspace,
        &mut prompt::Prompted,
        dispatch.target,
        dispatch.direction,
        &dispatch.names,
    )?;
    eprint!("{}", render::audit(&report));

    Ok(if report.is_clean() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

/// The two targets `sync` converges, over the named one or both, narrowed by
/// mapping names and, for the clan target, `--direction`.
///
/// The exit code is zero when every mapping on every target the run scoped to
/// converged without a conflict, a refusal, or an unjudgeable side.
fn sync_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    const FORM: &str = "sync [clan|keepassxc] [<mapping>...] [--direction <value>]";
    let dispatch = parse_dispatch("sync", FORM, arguments)?;

    let workspace = workspace()?;
    let mut out = String::new();
    let mut refused = false;

    if matches!(dispatch.target, None | Some(bridge::Target::Clan)) {
        let run = bridge::sync(&workspace, &Terminal, dispatch.direction, &dispatch.names)?;
        refused |= run.refused();
        out.push_str(&render::transfer(&run));
    }
    if matches!(dispatch.target, None | Some(bridge::Target::Keepassxc)) {
        let report = sync::run(
            &workspace,
            &Terminal,
            &mut prompt::Prompted,
            &dispatch.names,
        )?;
        refused |= !report.is_clean();
        out.push_str(&render::sync(&report));
    }

    eprint!("{out}");
    Ok(if refused {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// An age identity for a person who has none, or their own public recipient
/// with `--show`.
fn keygen_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    const FORM: &str = "keygen [--for-someone-else] [<user>] | keygen --show";

    if let [first, rest @ ..] = arguments
        && first == "--show"
    {
        if !rest.is_empty() {
            return Err(Refusal::Usage { form: FORM });
        }
        keygen::show(&Terminal)?;
        return Ok(ExitCode::SUCCESS);
    }

    let (for_someone_else, rest) = match arguments.split_first() {
        Some((first, tail)) if first == "--for-someone-else" => (true, tail),
        _ => (false, arguments),
    };

    let workspace = workspace()?;
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

    let workspace = workspace()?;
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
        let valued = |sink: &mut String| match tail.split_first() {
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

    let workspace = workspace()?;
    // D4 of unlock-keepassxc-composite-key's design.md: `--store-database`
    // names a database independently of `flake.safix.keepassxc.database`, on
    // the same file by a different route, so the declared composite-key
    // factors are read off `workspace.keepassxc()` and applied here rather
    // than through a second declaration on this flag.
    if options.mirror.database.is_some() {
        let mirror = workspace.keepassxc()?;
        options.mirror.yubikey.clone_from(&mirror.yubikey);
        options.mirror.key_file.clone_from(&mirror.key_file);
    }
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

/// One subject into or out of one group's declared membership.
///
/// Two positionals in one order and no flags, because the two acts are the whole
/// of the verb: an operator who has to remember which of `add` and `remove` takes
/// a flag is an operator who gets it wrong on the one that narrows an audience.
fn group_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    const FORM: &str = "group add|remove <group> <subject>";
    let act = match arguments.split_first() {
        Some((word, _)) if word == "add" => group::Act::Add,
        Some((word, _)) if word == "remove" => group::Act::Remove,
        _ => return Err(Refusal::Usage { form: FORM }),
    };
    let [_, group, subject] = arguments else {
        return Err(Refusal::Usage { form: FORM });
    };

    let workspace = workspace()?;
    group::run(&workspace, &Terminal, act, group, subject)?;
    Ok(ExitCode::SUCCESS)
}

/// A machine's own ed25519 host key, seeded before its first activation:
/// written straight to `--directory DIR`, or probed and then written over
/// ssh to `--to ADDRESS`.
///
/// Flags are read in any order and around the one positional, the way
/// `enroll`'s are — `--directory` and `--to` name the two write modes and
/// are mutually exclusive, and naming neither or both is the same usage
/// refusal.
fn upload_command(arguments: &[String]) -> Result<ExitCode, Refusal> {
    const FORM: &str = "upload <machine> --directory DIR --identity PATH | \
                        upload <machine> --to ADDRESS [--identity PATH] [--force]";

    let mut directory: Option<String> = None;
    let mut identity: Option<String> = None;
    let mut to: Option<String> = None;
    let mut force = false;
    let mut positional: Vec<String> = Vec::new();
    let mut rest = arguments;

    while let Some((first, tail)) = rest.split_first() {
        let valued = |sink: &mut Option<String>| match tail.split_first() {
            Some((value, after)) => {
                *sink = Some(value.clone());
                Ok(after)
            }
            None => Err(Refusal::OptionNeedsValue {
                option: first.clone(),
            }),
        };
        match first.as_str() {
            "--directory" => rest = valued(&mut directory)?,
            "--identity" => rest = valued(&mut identity)?,
            "--to" => rest = valued(&mut to)?,
            "--force" => {
                force = true;
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

    let [machine] = positional.as_slice() else {
        return Err(Refusal::Usage { form: FORM });
    };
    let target = match (directory, to) {
        (Some(dir), None) => upload::Target::Directory(PathBuf::from(dir)),
        (None, Some(address)) => upload::Target::Remote(address),
        _ => return Err(Refusal::Usage { form: FORM }),
    };

    let workspace = workspace()?;
    upload::run(
        &workspace,
        &Terminal,
        machine,
        &target,
        &upload::Options {
            identity: identity.map(PathBuf::from),
            force,
        },
    )?;
    Ok(ExitCode::SUCCESS)
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

    /// The boundary sentence is one string, and the help text carries that one.
    ///
    /// `safix_core::delegation::BOUNDARY` is where it is written, and every refusal
    /// in that family ends with it. This help text cannot interpolate a `const`
    /// into a `const`, so the words are pasted — and pasted words drift, which is
    /// what this reads.
    #[test]
    fn the_group_help_carries_the_boundary_sentence_word_for_word() {
        assert!(
            usage::GROUP.contains(safix_core::delegation::BOUNDARY),
            "the help text's boundary paragraph has drifted from the one the \
             refusals carry:\n{}",
            safix_core::delegation::BOUNDARY
        );
    }
}
