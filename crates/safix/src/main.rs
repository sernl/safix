#![forbid(unsafe_code)]

//! The safix command.
//!
//! This binary is the thin edge of [`safix_core`]: argument parsing, operator
//! interaction, and the rendering of refusals. No decision about custody,
//! drift, ordering or writing is made here.
//!
//! # What this binary does today
//!
//! It reports its version and refuses everything else. The runtime is being
//! ported from `modules/flake/safix/safix.sh` one subcommand at a time, and a
//! subcommand appears here only once a differential harness has compared it
//! against the shell runtime on stdout, standard error, exit code and effect on
//! the repository. Until every subcommand has passed, the flake's `safix`
//! package builds the shell script and this binary ships beside it as
//! `safix-rs`.

use miette::{Diagnostic, Result};
use thiserror::Error;

/// The refusal every invocation but `--version` receives, for as long as the
/// port is in progress.
#[derive(Debug, Error, Diagnostic)]
#[error("the rust runtime implements no subcommand yet")]
#[diagnostic(
    code(safix::runtime_is_scaffold),
    help(
        "the shell runtime is the one that ships: run the flake's `safix` package. This binary is `safix-rs`, and it takes over one subcommand at a time as each passes the differential harness."
    )
)]
struct RuntimeIsScaffold;

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);

    match (arguments.next(), arguments.next()) {
        (Some(only), None) if only == "--version" => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => Err(RuntimeIsScaffold.into()),
    }
}
