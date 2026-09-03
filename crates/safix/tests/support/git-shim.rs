//! The git the commit-ordering drills point safix at, to refuse one commit.
//!
//! `SAFIX_SHIM_GIT` names the real git; `SAFIX_GIT_SHIM_REFUSE_ROOT` names one
//! repository's working tree whose `commit` invocation this refuses. Every
//! other invocation — `add`, `status`, `rev-parse`, `var`, and `commit` at any
//! other root — passes straight through to the real git, so the preflight and
//! the vault-root commit a half-landed-state drill depends on are unaffected.
//!
//! Canonicalized before comparison, because `-C` is handed whatever path the
//! runtime resolved a root to, which need not be spelled identically to the
//! path the drill named when it set the variable.

use std::path::PathBuf;
use std::process::Command;

fn main() -> ! {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let real = std::env::var("SAFIX_SHIM_GIT").unwrap_or_else(|_| "git".to_owned());

    if refuses(&arguments) {
        eprintln!("safix-git-shim: refusing this commit for the drill");
        std::process::exit(1);
    }

    let status = Command::new(real)
        .args(&arguments)
        .status()
        .unwrap_or_else(|cause| {
            eprintln!("safix-git-shim: could not run the real git: {cause}");
            std::process::exit(1);
        });
    std::process::exit(status.code().unwrap_or(1));
}

/// Whether this invocation is a `commit` at the root the drill named.
fn refuses(arguments: &[String]) -> bool {
    let Ok(refuse_root) = std::env::var("SAFIX_GIT_SHIM_REFUSE_ROOT") else {
        return false;
    };
    if !arguments.iter().any(|argument| argument == "commit") {
        return false;
    }
    let Some(named) = named_root(arguments) else {
        return false;
    };
    canonicalized(&named) == canonicalized(&PathBuf::from(refuse_root))
}

/// The path following this invocation's `-C`, if it named one.
fn named_root(arguments: &[String]) -> Option<PathBuf> {
    let position = arguments.iter().position(|argument| argument == "-C")?;
    arguments.get(position.checked_add(1)?).map(PathBuf::from)
}

fn canonicalized(path: &std::path::Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
