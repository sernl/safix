//! The `nix` the integration suite drives the runtime against.
//!
//! A flake evaluation is the one thing a hermetic test cannot do, so it is the
//! one thing stubbed. Everything else the suite drives — sops, age, git — is the
//! real program, because a stub standing in for a backend is what lets a check
//! stay green over a command calling something the tree no longer contains.
//!
//! This stub is not a convenience: it asserts. It refuses an attribute it was
//! not asked for, a flake reference naming something other than the fixture
//! repository, and a read mode other than the one the runtime uses for that
//! attribute. Renaming `flake.safix.lib.placements`, or reading it `--raw` where
//! it was `--json`, fails here rather than at an operator's terminal.
//!
//! It answers out of files the harness writes, named by environment:
//! `SAFIX_FIXTURE_PLACEMENTS`, `_AUDIENCES`, `_GOVERNED`, `_RECIPIENTS`,
//! `_GENPLAN`, `_BRIDGE`, `_KEEPASSXC`, `_HOOK`, `_ENROLL_HOOK` and `_RULES`. `safix.lib.policyText` is the exception and
//! is computed here from what git tracks, because that is the property
//! `adduser`'s staging order turns on: an evaluation reads the files git tracks,
//! so a command that regenerates the policy before staging its scaffold writes a
//! policy missing the person it has just declared. A stub emitting a fixed
//! document passes either order and notices nothing.

use std::path::Path;
use std::process::Command;

/// Every attribute of the nix half this runtime reads, and how.
///
/// The read mode is part of the assertion: `--raw` is a string taken verbatim
/// and `--json` is a document, and a runtime that swapped them would be reading
/// a quoted string as a pattern.
const ATTRIBUTES: [(&str, Mode, Source); 11] = [
    (
        "safix.lib.placements",
        Mode::Json,
        Source::Fixture("SAFIX_FIXTURE_PLACEMENTS"),
    ),
    (
        "safix.lib.audiences",
        Mode::Json,
        Source::Fixture("SAFIX_FIXTURE_AUDIENCES"),
    ),
    (
        "safix.lib.governedFiles",
        Mode::Json,
        Source::Fixture("SAFIX_FIXTURE_GOVERNED"),
    ),
    (
        "safix.lib.recipients",
        Mode::Json,
        Source::Fixture("SAFIX_FIXTURE_RECIPIENTS"),
    ),
    (
        "safix.lib.generatorPlan",
        Mode::Json,
        Source::Fixture("SAFIX_FIXTURE_GENPLAN"),
    ),
    (
        "safix.lib.bridge",
        Mode::Json,
        Source::Fixture("SAFIX_FIXTURE_BRIDGE"),
    ),
    (
        "safix.lib.keepassxc",
        Mode::Json,
        Source::Fixture("SAFIX_FIXTURE_KEEPASSXC"),
    ),
    (
        "safix.onboardingHook",
        Mode::Json,
        Source::Fixture("SAFIX_FIXTURE_HOOK"),
    ),
    (
        "safix.enrollHook",
        Mode::Json,
        Source::Fixture("SAFIX_FIXTURE_ENROLL_HOOK"),
    ),
    ("safix.lib.nameRegex", Mode::Raw, Source::NameRegex),
    ("safix.lib.policyText", Mode::Raw, Source::PolicyText),
];

/// The alphabet a declared name is drawn from, as `modules/flake/safix`
/// publishes it.
const NAME_REGEX: &str = "[a-z0-9][a-z0-9_-]*";

/// How an attribute is read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// `nix eval --json`.
    Json,
    /// `nix eval --raw`.
    Raw,
}

impl Mode {
    /// The flag this mode arrives as.
    const fn flag(self) -> &'static str {
        match self {
            Self::Json => "--json",
            Self::Raw => "--raw",
        }
    }
}

/// Where an attribute's answer comes from.
#[derive(Clone, Copy)]
enum Source {
    /// A file the harness wrote, named by this environment variable.
    Fixture(&'static str),
    /// The name alphabet, which is a constant of the nix half.
    NameRegex,
    /// The policy the declarations git tracks imply, computed here.
    PolicyText,
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.split_first() {
        Some((verb, rest)) if verb == "eval" => eval(rest),
        Some((verb, rest)) if verb == "shell" => shell(rest),
        _ => refuse(&format!("unexpected invocation: {}", arguments.join(" "))),
    }
}

/// Answer one attribute, having asserted which one was asked for and how.
fn eval(arguments: &[String]) -> ! {
    let [mode, reference] = arguments else {
        refuse(&format!(
            "expected `eval <--json|--raw> <flake>#<attribute>`, got: {}",
            arguments.join(" ")
        ));
    };
    let Some((root, attribute)) = reference.split_once('#') else {
        refuse(&format!(
            "'{reference}' is not a <flake>#<attribute> reference"
        ));
    };
    let expected_root = environment("SAFIX_REPO_ROOT");
    if root != expected_root {
        refuse(&format!(
            "the evaluation names '{root}' rather than the fixture repository '{expected_root}'"
        ));
    }

    let Some((_, expected_mode, source)) = ATTRIBUTES
        .into_iter()
        .find(|(name, _, _)| *name == attribute)
    else {
        refuse(&format!(
            "unexpected attribute: {attribute}. The suite answers only what the runtime declares in safix_core::nix::Attribute, so a rename arrives here."
        ));
    };
    if mode != expected_mode.flag() {
        refuse(&format!(
            "{attribute} was read {mode}, and the runtime reads it {}",
            expected_mode.flag()
        ));
    }

    match source {
        Source::Fixture(variable) => print!("{}", read(Path::new(&environment(variable)))),
        Source::NameRegex => print!("{NAME_REGEX}"),
        Source::PolicyText => print!("{}", policy_text(Path::new(root))),
    }
    std::process::exit(0)
}

/// The policy the tracked declarations imply.
///
/// The anchors follow `git ls-files`, which is what an evaluation sees; the
/// rules half comes from the fixture, because rendering it is `policy.nix`'s
/// claim and this stands in for an evaluation rather than for the renderer.
///
/// Both halves of a person's keys are projected: their `recipient`, and every
/// entry of their `recoveryRecipients`. That second one is what makes an
/// enrollment's edit observably move `.sops.yaml` — the anchor for the card
/// appears in the regenerated policy and is committed with it — rather than
/// something a fixture asserted about itself.
///
/// Where this stops short of `policy.nix` is the rules, and deliberately: the real
/// renderer grants a recovery recipient in the creation rule too, and a rule
/// naming an `age1yubikey1…` key would send the real `sops updatekeys` to the age
/// plugin, which needs the card. So the projection is the anchors, the sandbox has
/// no card, and the wrap itself is the one thing these checks leave to the
/// operator's first real run.
fn policy_text(root: &Path) -> String {
    let mut policy = String::from("keys:\n");
    for tracked in capture(root, &["ls-files", "--", "safix/users"]).lines() {
        let Some(user) = tracked
            .strip_prefix("safix/users/")
            .and_then(|rest| rest.strip_suffix(".nix"))
        else {
            continue;
        };
        let declaration = read(&root.join(tracked));
        let recipient = declaration.lines().find_map(|line| {
            line.trim()
                .strip_prefix("recipient = \"")
                .and_then(|rest| rest.strip_suffix("\";"))
        });
        if let Some(recipient) = recipient {
            policy.push_str(&anchor(user, recipient));
        }
        for (nth, recovery) in recovery_recipients(&declaration).iter().enumerate() {
            policy.push_str(&anchor(&format!("{user}_recovery_{nth}"), recovery));
        }
    }
    policy.push_str(&read(Path::new(&environment("SAFIX_FIXTURE_RULES"))));
    policy
}

/// One anchor line, in the shape `policy.nix` renders one.
///
/// `crates/safix/tests/harness/mod.rs` writes the fixture's own committed policy
/// and renders anchors the same way; a change to the shape here without one there
/// is a `check` that reports drift against a policy nothing moved.
fn anchor(name: &str, key: &str) -> String {
    format!("  - &{name} {key}\n")
}

/// Every key a declaration's `recoveryRecipients` names.
///
/// The anchor-attrset form the option types and `safix enroll` writes:
/// `"<anchor>".key = "<key>";`, one per line. Anything else is not read here
/// rather than half-read, so a shape the writer stopped producing stops being
/// parsed instead of being guessed at.
fn recovery_recipients(declaration: &str) -> Vec<String> {
    declaration
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('"') && line.contains(".key ="))
        .filter_map(|line| line.split('"').nth(3))
        .map(str::to_owned)
        .collect()
}

/// Run a generator's script with its declared tools nominally on `PATH`.
///
/// The shape is asserted rather than realised: the flake the inputs are
/// resolved from, every spec being an attribute of that flake's nixpkgs, and the
/// `-c`. What this cannot assert is that the packages exist; `safix-generator-tools`
/// is the check that does that. The remainder is run with this process's
/// descriptors inherited, because the descriptors a generator's input travels
/// down are the subject of `generate-isolation`.
///
/// `SAFIX_TEST_UNRESOLVABLE` names one attribute this stub refuses to resolve,
/// which is how a drill takes the sandbox backend out of the toolset. A real
/// `nix shell` exits non-zero for an attribute it cannot resolve and runs
/// nothing, and that is what this reproduces — the runtime meets a resolution
/// that failed rather than a tool that is missing from a path.
fn shell(arguments: &[String]) -> ! {
    let mut rest = arguments;
    match rest.split_first() {
        Some((flag, tail)) if flag == "--inputs-from" => rest = tail,
        _ => refuse("expected `--inputs-from` first in a `nix shell` invocation"),
    }
    match rest.split_first() {
        Some((root, tail)) if *root == environment("SAFIX_REPO_ROOT") => rest = tail,
        _ => refuse("`--inputs-from` does not name the fixture repository"),
    }
    let withheld = std::env::var("SAFIX_TEST_UNRESOLVABLE").ok();
    while let Some((first, tail)) = rest.split_first() {
        if first == "-c" {
            rest = tail;
            break;
        }
        let Some(attribute) = first.strip_prefix("nixpkgs#") else {
            refuse(&format!("'{first}' is not a nixpkgs#<attribute> spec"));
        };
        if withheld.as_deref() == Some(attribute) {
            refuse(&format!(
                "error: flake 'nixpkgs' does not provide attribute '{attribute}'"
            ));
        }
        rest = tail;
    }
    let Some((program, tail)) = rest.split_first() else {
        refuse("no command after `-c`");
    };

    let status = Command::new(program).args(tail).status();
    match status {
        Ok(status) => std::process::exit(status.code().unwrap_or(128)),
        Err(cause) => refuse(&format!("could not run '{program}': {cause}")),
    }
}

/// One environment variable the harness is required to have set.
fn environment(variable: &str) -> String {
    match std::env::var(variable) {
        Ok(value) => value,
        Err(_) => refuse(&format!("{variable} is unset; the harness sets it")),
    }
}

/// One file the harness wrote.
fn read(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(cause) => refuse(&format!("{} is unreadable: {cause}", path.display())),
    }
}

/// One git question, answered in the fixture repository.
fn capture(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output();
    match output {
        Ok(output) => String::from_utf8_lossy(&output.stdout).into_owned(),
        Err(cause) => refuse(&format!("git {} failed: {cause}", arguments.join(" "))),
    }
}

/// Refuse, on standard error, with the status a `nix` that cannot answer exits
/// with.
fn refuse(reason: &str) -> ! {
    eprintln!("stub nix: {reason}");
    std::process::exit(1)
}
