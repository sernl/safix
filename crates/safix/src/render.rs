//! The prose a drift report is printed as.
//!
//! [`safix_core::check`] answers the questions and returns findings; this turns
//! each finding into the paragraph the shell runtime prints for it, word for
//! word, because that prose is the tested contract and the differential harness
//! compares it.
//!
//! The shape is the shell's two functions. A finding is a blank line, then its
//! headline; detail lines under it are indented two spaces, and the items they
//! list four; every remedy is indented four. The whole report goes to standard
//! error — `check` writes nothing to standard output, which is what lets it be
//! run with output redirected and still be read.

use safix_core::check::{Finding, Mint};

use crate::reporter::PROGRAM;

/// The whole report, findings in order, and the closing line.
///
/// A report with no findings is one line and no blank line before it. A report
/// with findings ends with a blank line and the count, and the caller exits
/// non-zero.
#[must_use]
pub fn report(findings: &[Finding]) -> String {
    let mut out = String::new();
    for finding in findings {
        push_finding(&mut out, finding);
    }
    if findings.is_empty() {
        out.push_str(
            "safix: no drift. The policy, the recipients and the values all agree with the declarations.\n",
        );
    } else {
        let closing = format!("\n{PROGRAM}: {} finding(s).\n", findings.len());
        out.push_str(&closing);
    }
    out
}

fn headline(out: &mut String, text: &str) {
    out.push('\n');
    out.push_str(text);
    out.push('\n');
}

fn detail(out: &mut String, text: &str) {
    out.push_str("  ");
    out.push_str(text);
    out.push('\n');
}

fn item(out: &mut String, text: &str) {
    out.push_str("    - ");
    out.push_str(text);
    out.push('\n');
}

fn remedy(out: &mut String, text: &str) {
    out.push_str("    ");
    out.push_str(text);
    out.push('\n');
}

/// The command that mints the value a shared-copy finding asks for.
fn mint_command(mint: &Mint) -> String {
    let Mint {
        carrier,
        name,
        generated,
    } = mint;
    if *generated {
        format!("    {PROGRAM} generate --regenerate {carrier} {name}")
    } else {
        format!("    {PROGRAM} set {carrier} {name}")
    }
}

/// Dispatch to the family a finding belongs to.
///
/// Split by family rather than written as one arm per variant, because the four
/// families are the four questions `check` asks and each one's prose is a
/// paragraph that reads as a whole.
fn push_finding(out: &mut String, finding: &Finding) {
    match finding {
        Finding::PolicyMissing | Finding::PolicyDiffers | Finding::UngovernableExtra { .. } => {
            push_policy(out, finding);
        }
        Finding::RecipientDrift { .. } => push_recipients(out, finding),
        Finding::SharedStrayMigration { .. } | Finding::SharedStrayRevocation { .. } => {
            push_shared(out, finding);
        }
        Finding::ValuelessName { .. } | Finding::UnclaimedValue { .. } => push_values(out, finding),
        _ => {}
    }
}

/// The policy artifact, and the files no rule can reach.
fn push_policy(out: &mut String, finding: &Finding) {
    match finding {
        Finding::PolicyMissing => {
            headline(
                out,
                ".sops.yaml does not exist, so no creation rule covers any file.",
            );
            remedy(out, &format!("{PROGRAM} fix"));
        }

        Finding::PolicyDiffers => {
            headline(
                out,
                ".sops.yaml differs from the policy flake.safix.users implies.",
            );
            remedy(out, &format!("{PROGRAM} fix"));
            remedy(out, "git diff .sops.yaml");
        }

        Finding::UngovernableExtra { file } => {
            headline(
                out,
                &format!(
                    "{file} is named in flake.safix.extraGovernedFiles and no creation rule's \
                     directory covers it, so nothing declares who should be able to open it and \
                     `{PROGRAM} fix` cannot re-wrap it."
                ),
            );
            remedy(
                out,
                "move it beside the secrets of the audience it belongs to, or drop it from flake.safix.extraGovernedFiles",
            );
        }

        _ => {}
    }
}

/// A governed file's stanzas against the audience declared for it.
fn push_recipients(out: &mut String, finding: &Finding) {
    if let Finding::RecipientDrift {
        file,
        extra,
        missing,
    } = finding
    {
        headline(
            out,
            &format!("{file} is not encrypted to the audience declared for it."),
        );
        if !extra.is_empty() {
            detail(out, "can open it and is not in its audience:");
            for key in extra {
                item(out, key);
            }
        }
        if !missing.is_empty() {
            detail(out, "is in its audience and cannot open it:");
            for key in missing {
                item(out, key);
            }
        }
        remedy(out, &format!("{PROGRAM} fix"));
        remedy(out, &format!("git diff -- {file}"));
    }
}

/// A shared name with a copy outside the file its audience reads.
///
/// The two kinds are rendered apart because they are different events with
/// different remedies, and the report says which it is in its first sentence.
fn push_shared(out: &mut String, finding: &Finding) {
    match finding {
        Finding::SharedStrayMigration { .. } => push_migration(out, finding),
        Finding::SharedStrayRevocation { .. } => push_revocation(out, finding),
        _ => {}
    }
}

/// Every reader of the copy is still in the audience, so no value has escaped.
fn push_migration(out: &mut String, finding: &Finding) {
    if let Finding::SharedStrayMigration {
        name,
        audience_file,
        stray_file,
        key,
        mint,
    } = finding
    {
        headline(
            out,
            &format!(
                "flake.safix.catalogue.{name} is shared, so one value in {audience_file} \
                 serves every carrier, but {stray_file} holds a value under '{key}' of its own."
            ),
        );
        detail(
            out,
            "Everyone who can open that copy is still in the audience, so this is a",
        );
        detail(
            out,
            "migration rather than a disclosure: the value the audience holds in common",
        );
        detail(
            out,
            &format!(
                "has not been minted into {audience_file} yet, and the copies left behind can disagree"
            ),
        );
        detail(
            out,
            "with each other. Which one should win is yours to say, not this tool's.",
        );
        remedy(out, "mint the value the audience is to share:");
        remedy(out, &mint_command(mint));
        remedy(
            out,
            &format!("then delete the superseded key:  sops {stray_file}"),
        );
        remedy(
            out,
            &format!("then converge the policy:        {PROGRAM} fix"),
        );
    }
}

/// Someone outside the audience can open the copy, so a value has escaped and
/// only a new one revokes it.
fn push_revocation(out: &mut String, finding: &Finding) {
    if let Finding::SharedStrayRevocation {
        name,
        audience_file,
        stray_file,
        key,
        named,
        orphaned,
        mint,
    } = finding
    {
        headline(
            out,
            &format!(
                "flake.safix.catalogue.{name} is shared and its audience reads \
                 {audience_file}, but {stray_file} still holds a value under '{key}' that \
                 someone outside that audience can open. This is a revocation."
            ),
        );
        if !named.is_empty() {
            detail(
                out,
                &format!("can open the copy in {stray_file} and is no longer a carrier:"),
            );
            for person in named {
                item(out, person);
            }
        }
        if !orphaned.is_empty() {
            detail(out, "can open it and answers to no declared user:");
            for key in orphaned {
                item(out, key);
            }
        }
        detail(
            out,
            "They have held the data key that copy is wrapped under, so re-wrapping it",
        );
        detail(
            out,
            &format!(
                "does not unread what they have already read. {PROGRAM} fix is not the remedy"
            ),
        );
        detail(
            out,
            "here and will not be: revoking means a value they never saw.",
        );
        remedy(out, "mint a new value for the audience that remains:");
        remedy(out, &mint_command(mint));
        remedy(
            out,
            &format!("then delete the revoked copy:    sops {stray_file}"),
        );
        remedy(
            out,
            &format!("then converge the policy:        {PROGRAM} fix"),
        );
    }
}

/// Declared names with no value, and values no declaration claims.
fn push_values(out: &mut String, finding: &Finding) {
    match finding {
        Finding::ValuelessName {
            user,
            name,
            file,
            generated,
        } => {
            let clause = if *generated {
                "It has a generator."
            } else {
                "It has no generator."
            };
            headline(
                out,
                &format!(
                    "flake.safix.users.{user} declares '{name}' and {file} holds no value for it. {clause}"
                ),
            );
            if *generated {
                remedy(out, &format!("{PROGRAM} generate {user} {name}"));
            } else {
                remedy(out, &format!("{PROGRAM} set {user} {name}"));
            }
        }

        Finding::UnclaimedValue { file, key } => {
            headline(
                out,
                &format!("{file} holds a value under '{key}' and no declaration claims it."),
            );
            remedy(
                out,
                &format!("declare it in flake.safix.users, or remove it with: sops {file}"),
            );
        }

        _ => {}
    }
}

/// One user's held names, as the rows `list` aligns.
///
/// The header is a row like any other, which is what makes the column widths
/// account for it.
#[must_use]
pub fn listing(
    held: &std::collections::BTreeMap<String, safix_core::model::Placement>,
) -> Vec<Vec<String>> {
    let mut rows = vec![
        ["NAME", "ORIGIN", "SHARED", "GENERATOR", "KEY", "FILE"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<String>>(),
    ];
    for (name, placement) in held {
        rows.push(vec![
            name.clone(),
            placement.origin.as_str().to_owned(),
            if placement.shared { "yes" } else { "-" }.to_owned(),
            placement.generator.as_ref().map_or_else(
                || "-".to_owned(),
                |generator| {
                    generator
                        .description
                        .clone()
                        .unwrap_or_else(|| "yes".to_owned())
                },
            ),
            placement.key.clone(),
            placement.file.clone(),
        ]);
    }
    rows
}
