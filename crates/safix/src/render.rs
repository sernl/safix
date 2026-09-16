//! The prose a drift report is printed as.
//!
//! [`safix_core::check`] answers the questions and returns findings; this turns
//! each finding into the paragraph the retired shell runtime printed for it,
//! word for word. That prose is the tested contract: it was held to the shell
//! runtime's byte for byte while both existed, and is now held to the literals
//! the integration suite asserts.
//!
//! The shape is the shell's two functions. A finding is a blank line, then its
//! headline; detail lines under it are indented two spaces, and the items they
//! list four; every remedy is indented four. The whole report goes to standard
//! error — `check` writes nothing to standard output, which is what lets it be
//! run with output redirected and still be read.

use safix_core::audit::{self, Disagreement, Side};
use safix_core::check::{Finding, Mint};
use safix_core::model::{Direction, Holders};

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
/// Split by family rather than written as one arm per variant, because the five
/// families are the five questions `check` asks and each one's prose is a
/// paragraph that reads as a whole.
fn push_finding(out: &mut String, finding: &Finding) {
    match finding {
        Finding::PolicyMissing
        | Finding::PolicyDiffers
        | Finding::UngovernableExtra { .. }
        | Finding::VaultGitignoreMissing
        | Finding::VaultRelocationPending { .. } => {
            push_policy(out, finding);
        }
        Finding::RecipientDrift { .. } => push_recipients(out, finding),
        Finding::SharedStrayMigration { .. } | Finding::SharedStrayRevocation { .. } => {
            push_shared(out, finding);
        }
        Finding::ValuelessName { .. } | Finding::UnclaimedValue { .. } => push_values(out, finding),
        Finding::DefinitionDrift { .. } => push_definition(out, finding),
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

        Finding::VaultGitignoreMissing => {
            headline(
                out,
                "the vault's .gitignore does not cover .sops-vault-rules.yaml, so a scratch \
                 rendering left behind by an interrupted run could be staged and committed.",
            );
            remedy(out, "add .sops-vault-rules.yaml to the vault's .gitignore");
        }

        Finding::VaultRelocationPending { file } => {
            headline(
                out,
                &format!(
                    "{file} is still at the declaration root, and a vault is declared: \
                     `{PROGRAM} fix` has not yet moved it into the vault."
                ),
            );
            remedy(out, &format!("{PROGRAM} fix"));
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
        narrowed,
        mints,
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
        push_narrowing(out, narrowed);
        remedy(out, &format!("{PROGRAM} fix"));
        remedy(out, &format!("git diff -- {file}"));
        if !extra.is_empty() && !mints.is_empty() {
            remedy(out, "then, to revoke rather than align, mint new values:");
            for mint in mints {
                remedy(out, &mint_command(mint));
            }
        }
    }
}

/// Whose custody the keys outside the audience are, and what a re-wrap of them
/// is not.
///
/// A key on a governed file that its declared audience does not name is what
/// every narrowing looks like from here — a grant dropped, a member removed from
/// a group, a machine changed hands — and `fix` is the right way to align the
/// ciphertext with the policy afterwards. It is not the remedy for the narrowing
/// itself, and saying so here is the whole point: whoever held that data key has
/// read what the file holds, and no re-wrap unreads it.
fn push_narrowing(out: &mut String, narrowed: &Holders) {
    let Holders { named, orphaned } = narrowed;
    if named.is_empty() && orphaned.is_empty() {
        return;
    }
    if !named.is_empty() {
        detail(out, "those keys are the custody of:");
        for subject in named {
            item(out, subject);
        }
    }
    if !orphaned.is_empty() {
        detail(out, "and of no declared subject:");
        for key in orphaned {
            item(out, key);
        }
    }
    detail(
        out,
        "A key the declared audience does not name is a narrowing — a grant dropped, a",
    );
    detail(
        out,
        "member removed from a group, a machine changed hands — so this is a revocation.",
    );
    detail(
        out,
        "Re-wrapping is not how one happens: whoever held that data key has read every",
    );
    detail(
        out,
        "value in the file, and no re-wrap unreads it. Only a new value revokes.",
    );
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

/// A generated value whose declaration has changed since it was minted.
///
/// The paragraph names both remedies and recommends neither, because the tree
/// holds a value and a declaration that disagree about how the value comes to be
/// and nothing here knows which of the two the operator meant. Regenerating adopts
/// the declaration; reverting the edit adopts the value.
///
/// No value appears, and none could: the finding is derived from a digest of the
/// declaration, and `check` never opened the file the value is in.
///
/// This is the one paragraph on this page with no shell antecedent — the record it
/// reads did not exist then — so it is held to the literals `crates/safix/tests/`
/// asserts and to nothing else.
fn push_definition(out: &mut String, finding: &Finding) {
    if let Finding::DefinitionDrift {
        user,
        name,
        generator,
        record,
    } = finding
    {
        headline(
            out,
            &format!(
                "flake.safix.users.{user} holds '{name}', minted by the generator on \
                 '{generator}', and that declaration is not the one it was minted under."
            ),
        );
        detail(
            out,
            &format!("{record} records the definition the value was minted under, and the"),
        );
        detail(
            out,
            "declaration no longer produces it. The value in the tree is a function of a",
        );
        detail(
            out,
            "generator that no longer exists, and reads exactly like one the current",
        );
        detail(out, "declaration would produce.");
        detail(
            out,
            "Which of the two is right is yours to say, not this tool's.",
        );
        remedy(out, "adopt the declaration by minting a new value:");
        remedy(
            out,
            &format!("    {PROGRAM} generate --regenerate {user} {name}"),
        );
        remedy(
            out,
            &format!("or adopt the value by reverting the edit to the '{generator}' generator"),
        );
    }
}

/// The eight facts `list` reports about one entry, in column order.
///
/// Named once because two tables show them: `list`'s own output and the
/// picker's candidate rows. Two tables claiming to show the same eight facts,
/// built by two pieces of code, drift on the first column that gains a rule —
/// the `GENERATOR` column's description/`yes`/`-` fallback below is already
/// such a rule, and the two stamp columns are two more.
///
/// `FILE` is last because it is the column the picker keeps behind tab: where a
/// value is served from answers a question about one entry rather than about a
/// list, so hiding it is hiding the end of the row rather than a hole in the
/// middle of one.
const LISTING_COLUMNS: [&str; 8] = [
    "NAME",
    "ORIGIN",
    "SHARED",
    "GENERATOR",
    "KEY",
    "CREATED",
    "UPDATED",
    "FILE",
];

/// The header row `list` and the picker both align their columns against.
#[must_use]
pub fn listing_header() -> Vec<String> {
    LISTING_COLUMNS.into_iter().map(str::to_owned).collect()
}

/// One held name, as the row `list` aligns and the picker offers.
///
/// The stamps arrive rather than being read here, because the two callers reach
/// them differently — `list` walks a whole user's entries and the picker holds
/// one record per candidate — and because a row builder that read files would
/// be a row builder a test could not call.
#[must_use]
pub fn listing_row(
    name: &str,
    placement: &safix_core::model::Placement,
    stamps: Option<safix_core::stamps::Stamps>,
) -> Vec<String> {
    vec![
        name.to_owned(),
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
        crate::picker::stamp(stamps.map(|stamps| stamps.created)),
        crate::picker::stamp(stamps.map(|stamps| stamps.updated)),
        placement.file.clone(),
    ]
}

/// One user's held names, as the rows `list` aligns.
///
/// The header is a row like any other, which is what makes the column widths
/// account for it.
///
/// `stamps` is asked for each entry by name rather than read here: `list` hands
/// it what it read out of the records and a test hands it a literal, which is
/// what keeps the two stamp columns assertable without a repository.
#[must_use]
pub fn listing(
    held: &std::collections::BTreeMap<String, safix_core::model::Placement>,
    stamps: impl Fn(&str) -> Option<safix_core::stamps::Stamps>,
) -> Vec<Vec<String>> {
    let mut rows = vec![listing_header()];
    rows.extend(
        held.iter()
            .map(|(name, placement)| listing_row(name, placement, stamps(name))),
    );
    rows
}

/// What a transfer run did, one line per mapping and a closing count.
///
/// Each line names the mapping and the outcome. An `updated` mapping's line
/// names its direction as an arrow instead of the outcome word, because the
/// direction is the fact a reader of a mixed run needs and the word alone
/// does not carry it; `unchanged`, `absent at source` and `refused` render as
/// a bare outcome word, because none of the three is a write a reader needs
/// an arrow to understand.
///
/// A refused mapping's reason is printed under its line rather than only
/// counted, because a run that says "refused" and not why is a run the operator
/// has to reproduce one mapping at a time to understand.
#[must_use]
pub fn transfer(run: &safix_core::bridge::Run) -> String {
    let mut out = String::new();
    if run.transferred.is_empty() {
        out.push_str(PROGRAM);
        out.push_str(": no mapping is declared.\n");
        return out;
    }

    for entry in &run.transferred {
        let line = if matches!(entry.outcome, safix_core::bridge::Outcome::Updated) {
            let arrow = match entry.direction {
                safix_core::model::Direction::ClanToSafix => {
                    format!("pulled {} \u{2190} clan", entry.mapping)
                }
                safix_core::model::Direction::SafixToClan => {
                    format!("pushed {} \u{2192} clan", entry.mapping)
                }
                // Never reached: `bridge::sync` reports a two-way mapping's
                // outcome through `bridge::bridge_sync::converge` and its own
                // `Run`, never through this one. Kept exhaustive rather than
                // guarded, so a future caller that did put one here would get
                // sensible text instead of a compile error pointing nowhere.
                safix_core::model::Direction::TwoWay => format!("converged {}", entry.mapping),
            };
            format!("{PROGRAM}: {arrow}\n")
        } else {
            let (from, to) = match entry.direction {
                safix_core::model::Direction::SafixToClan => (&entry.safix, &entry.clan),
                safix_core::model::Direction::ClanToSafix
                | safix_core::model::Direction::TwoWay => (&entry.clan, &entry.safix),
            };
            format!(
                "{PROGRAM}: {mapping}  {from} -> {to}  {outcome}\n",
                mapping = entry.mapping,
                outcome = entry.outcome.as_str(),
            )
        };
        out.push_str(&line);
        if let safix_core::bridge::Outcome::Refused(reason) = &entry.outcome {
            out.push('\n');
            for line in reason.to_string().lines() {
                out.push_str("  ");
                out.push_str(line);
                out.push('\n');
            }
            out.push('\n');
        }
    }

    let tally = run.tally();
    let closing = format!(
        "{PROGRAM}: {total} mapping(s): {} updated, {} unchanged, {} absent at source, {} refused.\n",
        tally.updated,
        tally.unchanged,
        tally.absent,
        tally.refused,
        total = run.transferred.len(),
    );
    out.push_str(&closing);
    out
}

/// Entries under a declared group that no declared mapping accounts for, in
/// the shape both `sync`'s and `audit`'s keepassxc reports give it.
fn push_lingering(out: &mut String, entries: &[String]) {
    for entry in entries {
        out.push('\n');
        if safix_core::store::is_companion(entry) {
            detail(
                out,
                &format!("{entry} is safix's own record of a two-way agreement, and the"),
            );
            detail(
                out,
                "mapping it belonged to is no longer declared. It holds no value \u{2014} only a",
            );
            detail(out, "digest of one \u{2014} and removing it is safe.");
        } else {
            detail(
                out,
                &format!("{entry} is in the group and no mapping declares it."),
            );
            detail(
                out,
                "A mapping removed after this entry was created leaves it looking",
            );
            detail(
                out,
                "exactly like this, and so does an entry a person placed in the",
            );
            detail(
                out,
                "group by hand \u{2014} the declarations cannot tell the two apart.",
            );
        }
        detail(out, "Nothing here will remove it; a person does that.");
    }
}

/// What an audit found, over whichever target or targets the run scoped to.
///
/// The shape is [`report`]'s, because this is the same kind of report over a
/// different question: a finding is a blank line and a headline, the lines
/// explaining it are indented two spaces, and the command that converges it is
/// indented four. Every headline names the mapping and its two endpoints, and
/// none of them names a value. The two target sections print one after the
/// other, each with its own closing line, because [`audit::Report`] carries
/// them as two independent sections rather than one merged list.
#[must_use]
pub fn audit(report: &audit::Report) -> String {
    let mut out = String::new();
    if let Some(clan) = &report.clan {
        push_clan_audit(&mut out, clan);
    }
    if let Some(keepassxc) = &report.keepassxc {
        push_keepassxc_audit(&mut out, keepassxc);
    }
    if let Some(pass) = &report.pass {
        push_pass_audit(&mut out, pass);
    }
    if let Some(bitwarden) = &report.bitwarden {
        push_bitwarden_audit(&mut out, bitwarden);
    }
    if let Some(onepassword) = &report.onepassword {
        push_onepassword_audit(&mut out, onepassword);
    }
    out
}

/// The clan target's section of an audit report.
fn push_clan_audit(out: &mut String, report: &audit::ClanReport) {
    for finding in &report.findings {
        push_disagreement(out, finding);
    }
    push_clan_lingering(out, &report.lingering);
    if report.findings.is_empty() {
        out.push_str(&agreed(report.examined));
    } else {
        let closing = format!(
            "\n{PROGRAM}: {} finding(s) over {} mapping(s).\n",
            report.findings.len(),
            report.examined,
        );
        out.push_str(&closing);
    }
}

/// Clan vars no currently declared mapping's clan side accounts for.
///
/// Placed the same way [`push_lingering`] is: after the section's own
/// findings and before its closing line. Unlike a keepassxc entry, a clan var
/// has no companion counterpart to distinguish, so every entry renders the
/// same way.
fn push_clan_lingering(out: &mut String, entries: &[String]) {
    for entry in entries {
        out.push('\n');
        detail(
            out,
            &format!("{entry} is a clan var and no declared mapping accounts for it."),
        );
        detail(
            out,
            "Nothing here reads, moves, or deletes it. A mapping removed after this var",
        );
        detail(
            out,
            "was created leaves it looking exactly like this, and so does a var that",
        );
        detail(
            out,
            "only clan's own services use \u{2014} the declarations cannot tell the two apart.",
        );
        detail(
            out,
            "If a person decides it is the former, clan's own command removes it.",
        );
    }
}

/// The keepassxc target's section of an audit report.
///
/// One line per compared mapping — agreeing included, the way [`sync`]'s own
/// report lists every mapping rather than only the ones that need a person —
/// because a keepassxc mapping's outcome is one of exactly four words rather
/// than the clan target's open-ended disagreement, and a report that named
/// all four is as short as one that named only two of them.
fn push_keepassxc_audit(out: &mut String, report: &audit::KeepassxcReport) {
    use audit::KeepassxcOutcome;
    use safix_core::model::Mode;

    if report.compared.is_empty() {
        out.push_str(PROGRAM);
        out.push_str(": no mapping is declared.\n");
        return;
    }

    for entry in &report.compared {
        let line = format!(
            "{PROGRAM}: {mapping}  {safix} <-> {kdbx}  {mode}  {outcome}\n",
            mapping = entry.mapping,
            safix = entry.safix,
            kdbx = entry.kdbx,
            mode = entry.mode,
            outcome = entry.outcome.as_str(),
        );
        out.push_str(&line);
        match &entry.outcome {
            KeepassxcOutcome::Diverged => {
                remedy(out, &format!("{PROGRAM} sync keepassxc {}", entry.mapping));
            }
            KeepassxcOutcome::FieldsDiverged(fields) => {
                out.push('\n');
                detail(
                    out,
                    &format!(
                        "The two sides agree on the value; these declared fields differ: {}.",
                        named_fields(fields),
                    ),
                );
                match entry.mode {
                    Mode::KeepassxcToSafix => {
                        detail(
                            out,
                            "A field has one author, and it is the declaration. Edit the \
                             declaration to",
                        );
                        detail(
                            out,
                            "say what the entry holds, or declare a mode that writes the database.",
                        );
                    }
                    Mode::Backup => {
                        detail(
                            out,
                            "backup never overwrites an existing entry, so nothing here would \
                             write them.",
                        );
                    }
                    Mode::SafixToKeepassxc | Mode::TwoWay => {
                        remedy(out, &format!("{PROGRAM} sync keepassxc {}", entry.mapping));
                    }
                }
                out.push('\n');
            }
            KeepassxcOutcome::Unjudgeable(reason) => {
                out.push('\n');
                for line in reason.to_string().lines() {
                    out.push_str("  ");
                    out.push_str(line);
                    out.push('\n');
                }
                out.push('\n');
            }
            KeepassxcOutcome::Agreeing => {}
        }
    }

    push_lingering(out, &report.lingering);

    let agreeing = report
        .compared
        .iter()
        .filter(|entry| matches!(entry.outcome, KeepassxcOutcome::Agreeing))
        .count();
    let diverged = report
        .compared
        .iter()
        .filter(|entry| matches!(entry.outcome, KeepassxcOutcome::Diverged))
        .count();
    let fields_diverged = report
        .compared
        .iter()
        .filter(|entry| matches!(entry.outcome, KeepassxcOutcome::FieldsDiverged(_)))
        .count();
    let unjudgeable = report
        .compared
        .iter()
        .filter(|entry| matches!(entry.outcome, KeepassxcOutcome::Unjudgeable(_)))
        .count();
    // The field count is printed only when there is one, so a report over
    // mappings declaring no field reads exactly as it read before fields
    // existed.
    let fields = counted("fields diverged", fields_diverged);
    let closing = format!(
        "{PROGRAM}: {total} mapping(s) against {database}: {agreeing} agreeing, {diverged} \
         diverged, {unjudgeable} unjudgeable{fields}.\n",
        total = report.compared.len(),
        database = report.database,
    );
    out.push_str(&closing);
}

/// The pass target's section of an audit report.
///
/// One line per compared mapping — agreeing included, the way
/// [`push_keepassxc_audit`] lists every mapping and for its reason.
fn push_pass_audit(out: &mut String, report: &audit::PassReport) {
    use audit::PassOutcome;
    use safix_core::model::PassMode;

    if report.compared.is_empty() {
        out.push_str(PROGRAM);
        out.push_str(": no mapping is declared.\n");
        return;
    }

    for entry in &report.compared {
        let line = format!(
            "{PROGRAM}: {mapping}  {safix} <-> {store} {mode}  {outcome}\n",
            mapping = entry.mapping,
            safix = entry.safix,
            store = entry.entry,
            mode = entry.mode,
            outcome = entry.outcome.as_str(),
        );
        out.push_str(&line);
        match &entry.outcome {
            PassOutcome::Diverged => {
                remedy(out, &format!("{PROGRAM} sync pass {}", entry.mapping));
            }
            PassOutcome::FieldsDiverged(fields) => {
                out.push('\n');
                detail(
                    out,
                    &format!(
                        "The two sides agree on the value; these declared fields differ: {}.",
                        named_fields(fields),
                    ),
                );
                match entry.mode {
                    PassMode::PassToSafix => {
                        detail(
                            out,
                            "A field has one author, and it is the declaration. Edit the \
                             declaration to",
                        );
                        detail(
                            out,
                            "say what the record holds, or declare a mode that writes the store.",
                        );
                    }
                    PassMode::Backup => {
                        detail(
                            out,
                            "backup never overwrites an existing entry, so nothing here would \
                             write them.",
                        );
                    }
                    PassMode::SafixToPass | PassMode::TwoWay => {
                        remedy(out, &format!("{PROGRAM} sync pass {}", entry.mapping));
                    }
                }
                out.push('\n');
            }
            PassOutcome::Unjudgeable(reason) => {
                out.push('\n');
                for line in reason.to_string().lines() {
                    out.push_str("  ");
                    out.push_str(line);
                    out.push('\n');
                }
                out.push('\n');
            }
            PassOutcome::Agreeing => {}
        }
    }

    push_pass_lingering(out, &report.lingering);

    let counting = |wanted: &str| {
        report
            .compared
            .iter()
            .filter(|entry| entry.outcome.as_str() == wanted)
            .count()
    };
    // The field count is printed only when there is one, so a report over
    // mappings declaring no field reads as the other store target's does.
    let fields = counted("fields diverged", counting("fields diverged"));
    let closing = format!(
        "{PROGRAM}: {total} mapping(s) against {store}: {agreeing} agreeing, {diverged} \
         diverged, {unjudgeable} unjudgeable{fields}.\n",
        total = report.compared.len(),
        store = report.store,
        agreeing = counting("agreeing"),
        diverged = counting("diverged"),
        unjudgeable = counting("unjudgeable"),
    );
    out.push_str(&closing);
}

/// Entries in the store that no declared mapping accounts for, in the shape
/// both `sync`'s and `audit`'s pass reports give it.
///
/// A separate function from [`push_lingering`] rather than a shared one: that
/// one asks `safix_core::store::is_companion`, and a companion here is an entry
/// in a `pass` store rather than a kdbx entry. The two suffixes are equal
/// today, and a shared renderer would make that equality load-bearing for the
/// prose as well as for the name.
fn push_pass_lingering(out: &mut String, entries: &[String]) {
    for entry in entries {
        out.push('\n');
        if safix_core::pass::is_companion(entry) {
            detail(
                out,
                &format!("{entry} is safix's own record of a two-way agreement, and the"),
            );
            detail(
                out,
                "mapping it belonged to is no longer declared. It holds no value \u{2014} only a",
            );
            detail(out, "digest of one \u{2014} and removing it is safe.");
        } else {
            detail(
                out,
                &format!("{entry} is in the store and no mapping declares it."),
            );
            detail(
                out,
                "A mapping removed after this entry was created leaves it looking",
            );
            detail(
                out,
                "exactly like this, and so does an entry a person put in the store",
            );
            detail(
                out,
                "by hand \u{2014} the declarations cannot tell the two apart.",
            );
        }
        detail(out, "Nothing here will remove it; a person does that.");
    }
}

/// The one line a run with nothing to report prints.
///
/// A bridge with no mapping declared and a bridge whose every mapping agrees
/// are different states. One sentence covering both would be read as the second
/// by a consumer who is in the first, which is the failure a report of nothing
/// is most able to hide.
fn agreed(examined: usize) -> String {
    if examined == 0 {
        format!("{PROGRAM}: no mapping is declared.\n")
    } else {
        format!("{PROGRAM}: no disagreement. All {examined} declared mapping(s) agree.\n")
    }
}

/// One mapping's paragraph, by what was found about it.
fn push_disagreement(out: &mut String, finding: &audit::Finding) {
    match &finding.disagreement {
        Disagreement::Values if finding.direction == Direction::TwoWay => {
            let (from, to) = flow(finding);
            headline(
                out,
                &format!(
                    "flake.safix.bridge.mappings.{mapping} is two-way, and {from} and {to} hold \
                     different values.",
                    mapping = finding.mapping,
                ),
            );
            remedy(out, &converging(finding));
        }

        Disagreement::Values => {
            let (from, to) = flow(finding);
            headline(
                out,
                &format!(
                    "flake.safix.bridge.mappings.{mapping} is {direction}, from {from} to {to}, \
                     and the two hold different values.",
                    mapping = finding.mapping,
                    direction = finding.direction,
                ),
            );
            remedy(out, &converging(finding));
        }

        Disagreement::OneSided(side) => push_one_sided(out, finding, *side),

        Disagreement::SafixSideUnreadable => {
            headline(
                out,
                &format!(
                    "flake.safix.bridge.mappings.{mapping} could not be judged: {safix} did not \
                     decrypt for you, so what it holds could not be compared with {clan}.",
                    mapping = finding.mapping,
                    safix = finding.safix,
                    clan = finding.clan,
                ),
            );
            detail(
                out,
                "sops has said why on its own standard error, above this.",
            );
            detail(
                out,
                "A mapping you cannot open is reported rather than left out: a report that",
            );
            detail(
                out,
                "dropped them would be a report about who ran it, and a clean one would mean",
            );
            detail(out, "less than it reads as.");
        }

        Disagreement::Unjudgeable(reason) => {
            let (from, to) = flow(finding);
            headline(
                out,
                &format!(
                    "flake.safix.bridge.mappings.{mapping} could not be judged, so whether {from} \
                     and {to} agree is not known.",
                    mapping = finding.mapping,
                ),
            );
            for line in reason.to_string().lines() {
                detail(out, line);
            }
        }
    }
}

/// One side holds a value and the other does not.
///
/// Which of the two is the source is what decides the remedy, and it comes off
/// the direction rather than off the side. A destination holding nothing is a
/// mapping nothing has transferred yet, and running the verb resolves it. A
/// source holding nothing is a mapping with nothing to send, and the verb would
/// refuse it — so what this names is minting the source first.
fn push_one_sided(out: &mut String, finding: &audit::Finding, side: Side) {
    let (holder, empty) = match side {
        Side::Clan => (&finding.clan, &finding.safix),
        Side::Safix => (&finding.safix, &finding.clan),
    };

    if side.is_source_of(finding.direction) {
        headline(
            out,
            &format!(
                "flake.safix.bridge.mappings.{mapping} is {direction}, and {holder} holds a value \
                 that {empty} does not.",
                mapping = finding.mapping,
                direction = finding.direction,
            ),
        );
        remedy(out, &converging(finding));
        return;
    }

    headline(
        out,
        &format!(
            "flake.safix.bridge.mappings.{mapping} is {direction}, and its source {empty} holds no \
             value while {holder} holds one.",
            mapping = finding.mapping,
            direction = finding.direction,
        ),
    );
    detail(
        out,
        "The direction says the value comes from the source, so there is nothing to send",
    );
    detail(
        out,
        "and nothing here decides which of the two sides should win.",
    );
    remedy(out, &format!("mint the source at {empty}, then:"));
    remedy(out, &format!("    {}", converging(finding)));
}

/// What a sync run did, one line per declared mapping and a closing count.
///
/// The shape [`transfer`] has, because it is the same kind of report over a
/// different relationship: each line names the mapping, its mode, both endpoints
/// and the outcome, the arrow points the way the value moved, and none of them
/// names a value. A mapping that needs a person — a conflict, a refusal, a side
/// that could not be judged — gets its paragraph under its line, because a run
/// that says "conflict" and not what to do about it is a run the operator has to
/// reproduce one mapping at a time to understand.
#[must_use]
pub fn sync(report: &safix_core::sync::Report) -> String {
    let mut out = String::new();
    if report.converged.is_empty() {
        let empty = format!("{PROGRAM}: no mapping is declared.\n");
        out.push_str(&empty);
        return out;
    }

    for entry in &report.converged {
        let line = format!(
            "{PROGRAM}: {mapping}  {flow}  {mode}  {outcome}\n",
            mapping = entry.mapping,
            flow = sync_flow(entry),
            mode = entry.mode,
            outcome = entry.outcome.as_str(),
        );
        out.push_str(&line);
        push_sync_detail(&mut out, entry);
    }

    push_lingering(&mut out, &report.lingering);

    let tally = report.tally();
    // The two field counts are printed only when there is one to print, so a
    // run over mappings declaring no field closes exactly as it closed before
    // fields existed.
    let fields_updated = counted("fields updated", tally.fields_updated);
    let fields_diverged = counted("fields diverged", tally.fields_diverged);
    let closing = format!(
        "{PROGRAM}: {total} mapping(s) against {database}: {} updated, {} pulled, {} unchanged, \
         {} conflict, {} refused, {} not judged{fields_updated}{fields_diverged}.\n",
        tally.updated,
        tally.pulled,
        tally.unchanged,
        tally.conflict,
        tally.refused,
        tally.not_judged,
        total = report.converged.len(),
        database = report.database,
    );
    out.push_str(&closing);
    out
}

/// One mapping's endpoints, in the order the value moved between them.
///
/// A mapping that wrote nothing has no direction to show, so its endpoints are
/// joined by a two-headed arrow: the line is about a relationship rather than
/// about a transfer. `fields updated` is a write toward the database and points
/// the way `updated` does; `fields diverged` wrote nothing and therefore shows
/// no arrow, exactly as a conflict does.
fn sync_flow(entry: &safix_core::sync::Converged) -> String {
    use safix_core::sync::Outcome;
    match entry.outcome {
        Outcome::Updated | Outcome::FieldsUpdated(_) => {
            format!("{} -> {}", entry.safix, entry.kdbx)
        }
        Outcome::Pulled => format!("{} -> {}", entry.kdbx, entry.safix),
        _ => format!("{} <-> {}", entry.safix, entry.kdbx),
    }
}

/// The paragraph under a mapping that needs a person.
fn push_sync_detail(out: &mut String, entry: &safix_core::sync::Converged) {
    use safix_core::model::Mode;
    use safix_core::sync::Outcome;

    match &entry.outcome {
        Outcome::Conflict if entry.mode == Mode::Backup => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} holds a value that is not {}'s, and backup never overwrites one.",
                    entry.kdbx, entry.safix
                ),
            );
            detail(
                out,
                "Nothing was written. Either accept the database's value, or declare",
            );
            remedy(out, "mode = \"safix-to-keepassxc\";");
            detail(
                out,
                "on that mapping, which makes the database follow safix.",
            );
        }

        Outcome::Conflict => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} and {} have both changed since the last agreement.",
                    entry.safix, entry.kdbx
                ),
            );
            detail(
                out,
                "Nothing was written, and nothing here decides which of the two was meant:",
            );
            detail(
                out,
                "last-writer-wins over secrets rewards whichever clock lied best.",
            );
            remedy(out, "to keep safix's value, declare on this mapping:");
            remedy(out, "    mode = \"safix-to-keepassxc\";");
            remedy(out, "to keep the database's, declare instead:");
            remedy(out, "    mode = \"keepassxc-to-safix\";");
            remedy(
                out,
                &format!(
                    "then:  {PROGRAM} sync keepassxc {mapping}",
                    mapping = entry.mapping
                ),
            );
            remedy(out, "and put the mode back to two-way afterwards.");
        }

        Outcome::Refused(reason) | Outcome::NotJudged(reason) => {
            out.push('\n');
            for line in reason.to_string().lines() {
                detail(out, line);
            }
            out.push('\n');
        }

        Outcome::FieldsUpdated(fields) => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} already held {}'s value; the declared fields written are: {}.",
                    entry.kdbx,
                    entry.safix,
                    named_fields(fields),
                ),
            );
            detail(
                out,
                "One write, carrying the value the entry already held: a kdbx save rewrites",
            );
            detail(
                out,
                "the whole file, so a field repair is one of those and not two.",
            );
        }

        Outcome::FieldsDiverged(fields) => push_fields_diverged(out, entry, fields),

        Outcome::Unchanged | Outcome::Updated | Outcome::Pulled => {}
    }
}

/// The paragraph under a mapping whose declared fields differ and whose mode
/// does not write them.
///
/// The remedy is the mode's: a pushing mode resolves it by running again, a
/// pulling mode cannot resolve it at all because a field's one author is the
/// declaration, and `backup` refuses to touch an existing entry on purpose.
fn push_fields_diverged(
    out: &mut String,
    entry: &safix_core::sync::Converged,
    fields: &[&'static str],
) {
    use safix_core::model::Mode;

    out.push('\n');
    detail(
        out,
        &format!(
            "{} and {} agree on the value; these declared fields differ: {}.",
            entry.safix,
            entry.kdbx,
            named_fields(fields),
        ),
    );
    detail(out, "Nothing was written.");
    match entry.mode {
        Mode::KeepassxcToSafix => {
            detail(
                out,
                "This mode writes safix rather than the database, and a field has one",
            );
            detail(
                out,
                "author: the declaration. Edit the declaration to say what the entry",
            );
            detail(out, "holds, or declare a mode that writes the database:");
            remedy(out, "mode = \"safix-to-keepassxc\";");
        }
        Mode::Backup => {
            detail(
                out,
                "backup never overwrites an existing entry, and writing its fields while",
            );
            detail(out, "refusing its value would make backup half a mode.");
        }
        Mode::SafixToKeepassxc | Mode::TwoWay => {
            remedy(
                out,
                &format!(
                    "{PROGRAM} sync keepassxc {mapping}",
                    mapping = entry.mapping
                ),
            );
        }
    }
}

/// One count for a report's closing line, or nothing at all when it is zero.
///
/// `counted("fields updated", 2)` is `", 2 fields updated"`. Absent rather
/// than zero, so a report over mappings declaring no field carries no word
/// about fields anywhere.
fn counted(word: &str, howmany: usize) -> String {
    if howmany == 0 {
        return String::new();
    }
    format!(", {howmany} {word}")
}

/// The declared fields a report names, as a list an operator reads.
///
/// The only thing interpolated anywhere in a field's report: every element is a
/// `&'static str` that came from the field's own name, so no field's content
/// can reach a report through this function or any other — the property is the
/// type's rather than a reviewer's.
fn named_fields(fields: &[&'static str]) -> String {
    match fields {
        [] => String::from("none"),
        [one] => (*one).to_owned(),
        [others @ .., last] => format!("{} and {last}", others.join(", ")),
    }
}

/// A mapping's two endpoints in the order its value moves between them.
///
/// A two-way mapping has no fixed order: `push_disagreement`'s own two-way
/// wording never says "from X to Y", so which of the pair lands in `from` and
/// which in `to` is unobserved here — clan first is arbitrary but consistent.
fn flow(finding: &audit::Finding) -> (&String, &String) {
    match finding.direction {
        Direction::SafixToClan => (&finding.safix, &finding.clan),
        Direction::ClanToSafix | Direction::TwoWay => (&finding.clan, &finding.safix),
    }
}

/// The command that converges one mapping, over the clan target.
///
/// `sync clan <mapping>` rather than a `--direction`-narrowed form: `sync`
/// converges the mapping in its own declared direction regardless, so naming
/// the filter would ask for something the plain form already does.
fn converging(finding: &audit::Finding) -> String {
    format!("{PROGRAM} sync clan {}", finding.mapping)
}

/// What a two-way convergence did, one line per two-way mapping acted on and
/// a closing count.
///
/// The shape [`transfer`] and [`sync`] both have, over
/// [`safix_core::bridge::bridge_sync::Report`] instead: each line names the
/// mapping and both endpoints, and a settled write prints `converged
/// <mapping>` rather than an arrow, because a two-way convergence names no
/// source and no destination \u{2014} the same wording `transfer`'s own dead
/// `TwoWay` arm already carries, for the case that in practice never reaches
/// it: a two-way mapping's outcome is reported through this function rather
/// than through `transfer`'s. A mapping that needs a person \u{2014} a conflict or
/// a refusal \u{2014} gets its paragraph under its line.
///
/// An empty report prints nothing at all, rather than a "no mapping is
/// declared" line of its own: [`transfer`] already prints that sentence
/// for a fleet with nothing declared \u{2014} `bridge::sync`'s own early return
/// covers both a genuinely empty bridge and one whose mappings are all
/// two-way \u{2014} and the ordinary case this covers, a fleet whose bridge
/// declares one-way mappings only, should read exactly as it did before
/// this convergence existed.
#[must_use]
pub fn bridge_sync(report: &safix_core::bridge::bridge_sync::Report) -> String {
    use safix_core::bridge::bridge_sync::Outcome;

    if report.converged.is_empty() {
        return String::new();
    }

    let mut out = String::new();

    for entry in &report.converged {
        let line = if matches!(
            entry.outcome,
            Outcome::UpdatedTowardClan | Outcome::UpdatedTowardSafix
        ) {
            format!("{PROGRAM}: converged {}\n", entry.mapping)
        } else {
            format!(
                "{PROGRAM}: {mapping}  {clan} <-> {safix}  {outcome}\n",
                mapping = entry.mapping,
                clan = entry.clan,
                safix = entry.safix,
                outcome = entry.outcome.as_str(),
            )
        };
        out.push_str(&line);
        push_bridge_sync_detail(&mut out, entry);
    }

    let tally = report.tally();
    let closing = format!(
        "{PROGRAM}: {total} mapping(s): {} converged, {} unchanged, {} conflict, {} refused.\n",
        tally
            .updated_toward_clan
            .saturating_add(tally.updated_toward_safix),
        tally.unchanged,
        tally.conflict,
        tally.refused,
        total = report.converged.len(),
    );
    out.push_str(&closing);
    out
}

/// The paragraph under a two-way mapping that needs a person.
fn push_bridge_sync_detail(out: &mut String, entry: &safix_core::bridge::bridge_sync::Converged) {
    use safix_core::bridge::bridge_sync::Outcome;

    match &entry.outcome {
        Outcome::Conflict => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} and {} have both changed since the last agreement.",
                    entry.safix, entry.clan
                ),
            );
            detail(
                out,
                "Nothing was written, and nothing here decides which of the two was meant:",
            );
            detail(
                out,
                "last-writer-wins over secrets rewards whichever clock lied best.",
            );
            remedy(out, "to keep safix's value, declare on this mapping:");
            remedy(out, "    direction = \"safix-to-clan\";");
            remedy(out, "to keep clan's, declare instead:");
            remedy(out, "    direction = \"clan-to-safix\";");
            remedy(
                out,
                &format!("then:  {PROGRAM} sync clan {}", entry.mapping),
            );
            remedy(out, "and put the direction back to two-way afterwards.");
        }

        Outcome::Refused(reason) => {
            out.push('\n');
            for line in reason.to_string().lines() {
                detail(out, line);
            }
            out.push('\n');
        }

        Outcome::Unchanged | Outcome::UpdatedTowardClan | Outcome::UpdatedTowardSafix => {}
    }
}

/// The 1password target's section of an audit report.
///
/// The shape [`push_keepassxc_audit`] has, and for its reason: this target's
/// outcome is one of exactly four words, so a report that names all four is as
/// short as one that named only the two needing a person. No content of a value
/// or of a field reaches it — a diverged field is named and never printed,
/// because a note body and a field sourced from another entry are themselves
/// secrets.
fn push_onepassword_audit(out: &mut String, report: &audit::OnePasswordReport) {
    use audit::OnePasswordOutcome;

    if report.compared.is_empty() {
        out.push_str(PROGRAM);
        out.push_str(": no mapping is declared.\n");
        return;
    }

    for entry in &report.compared {
        let line = format!(
            "{PROGRAM}: {mapping}  {safix} <-> {item}  {mode}  {outcome}\n",
            mapping = entry.mapping,
            safix = entry.safix,
            item = entry.item,
            mode = entry.mode,
            outcome = entry.outcome.as_str(),
        );
        out.push_str(&line);
        match &entry.outcome {
            OnePasswordOutcome::Diverged => {
                onepassword_remedy(out, entry.mode, &entry.mapping);
            }
            OnePasswordOutcome::FieldsDiverged(fields) => {
                out.push('\n');
                detail(
                    out,
                    &format!(
                        "The two sides agree on the value; these declared fields differ: {}.",
                        named_fields(fields),
                    ),
                );
                onepassword_remedy(out, entry.mode, &entry.mapping);
                out.push('\n');
            }
            OnePasswordOutcome::Unjudgeable(reason) => {
                out.push('\n');
                for line in reason.to_string().lines() {
                    detail(out, line);
                }
                out.push('\n');
            }
            OnePasswordOutcome::Agreeing => {}
        }
    }

    push_onepassword_lingering(out, &report.lingering);

    let counting = |wanted: &str| {
        report
            .compared
            .iter()
            .filter(|entry| entry.outcome.as_str() == wanted)
            .count()
    };
    let fields = counted("fields diverged", counting("fields diverged"));
    let closing = format!(
        "{PROGRAM}: {total} mapping(s) against {account}: {agreeing} agreeing, {diverged} \
         diverged, {unjudgeable} unjudgeable{fields}.\n",
        total = report.compared.len(),
        account = onepassword_account(report.account.as_deref()),
        agreeing = counting("agreeing"),
        diverged = counting("diverged"),
        unjudgeable = counting("unjudgeable"),
    );
    out.push_str(&closing);
}

/// What a diverged 1password mapping's remedy is, which the mode decides.
///
/// A pushing mode resolves it by running again. `1password-to-safix` cannot:
/// for that mode converging toward the declaration is what the mode already
/// means, so the declaration is the author and the remedy is to edit it.
/// `backup` refuses to touch an existing item on purpose.
fn onepassword_remedy(out: &mut String, mode: safix_core::model::OnePasswordMode, mapping: &str) {
    use safix_core::model::OnePasswordMode;

    match mode {
        OnePasswordMode::OnePasswordToSafix => {
            detail(
                out,
                "This mode writes safix rather than the item, and a field has one author:",
            );
            detail(
                out,
                "the declaration. Edit the declaration to say what the item holds, or",
            );
            detail(out, "declare a mode that writes the item:");
            remedy(out, "mode = \"safix-to-1password\";");
        }
        OnePasswordMode::Backup => {
            detail(
                out,
                "backup never overwrites an existing item, and writing its fields while",
            );
            detail(out, "refusing its value would make backup half a mode.");
        }
        OnePasswordMode::SafixToOnePassword | OnePasswordMode::TwoWay => {
            remedy(out, &format!("{PROGRAM} sync 1password {mapping}"));
        }
    }
}

/// Items in a declared vault that no declared mapping accounts for.
///
/// Placed the way [`push_lingering`] is: after the section's own lines and
/// before its closing line. No companion counterpart to distinguish, because
/// this target's memory is a field of the mapped item rather than a second
/// object beside it, so every entry renders the same way.
fn push_onepassword_lingering(out: &mut String, entries: &[String]) {
    for entry in entries {
        out.push('\n');
        detail(
            out,
            &format!("{entry} is in a declared vault and no mapping declares it."),
        );
        detail(
            out,
            "A mapping removed after this item was created leaves it looking exactly",
        );
        detail(
            out,
            "like this, and so does an item a person put in the vault themselves \u{2014} the",
        );
        detail(out, "declarations cannot tell the two apart.");
        detail(out, "Nothing here will remove it; a person does that.");
    }
}

/// How a report names the account it converged against.
///
/// An undeclared account is a working configuration rather than a missing
/// declaration, so the report says which posture it was rather than printing an
/// empty string where a name would be.
fn onepassword_account(account: Option<&str>) -> String {
    match account {
        Some(account) => account.to_owned(),
        None => String::from("the account op resolved"),
    }
}

/// What a pass sync run did, one line per declared mapping.
///
/// The shape [`sync`] has, because the two store targets report the same set
/// of outcomes. The difference is the closing line's subject — a store root
/// rather than a database — and that there is no value-shape refusal to
/// explain, because this store's write takes the whole body on a pipe.
#[must_use]
pub fn pass(report: &safix_core::pass::Report) -> String {
    let mut out = String::new();
    if report.converged.is_empty() {
        let empty = format!("{PROGRAM}: no mapping is declared.\n");
        out.push_str(&empty);
        return out;
    }

    for entry in &report.converged {
        let line = format!(
            "{PROGRAM}: {mapping}  {flow}  {mode}  {outcome}\n",
            mapping = entry.mapping,
            flow = pass_flow(entry),
            mode = entry.mode,
            outcome = entry.outcome.as_str(),
        );
        out.push_str(&line);
        push_pass_detail(&mut out, entry);
    }

    push_pass_lingering(&mut out, &report.lingering);

    let tally = report.tally();
    let fields_updated = counted("fields updated", tally.fields_updated);
    let fields_diverged = counted("fields diverged", tally.fields_diverged);
    let closing = format!(
        "{PROGRAM}: {total} mapping(s) against {store}: {} updated, {} pulled, {} unchanged, \
         {} conflict, {} refused, {} not judged{fields_updated}{fields_diverged}.\n",
        tally.updated,
        tally.pulled,
        tally.unchanged,
        tally.conflict,
        tally.refused,
        tally.not_judged,
        total = report.converged.len(),
        store = report.store,
    );
    out.push_str(&closing);
    out
}

/// One mapping's endpoints, in the order the value moved between them.
///
/// [`sync_flow`]'s rule, over this target's own endpoints: a mapping that wrote
/// nothing has no direction to show, `fields updated` is a write toward the
/// store and points the way `updated` does, and `fields diverged` wrote nothing
/// and therefore shows no arrow.
fn pass_flow(entry: &safix_core::pass::Converged) -> String {
    use safix_core::pass::Outcome;
    match entry.outcome {
        Outcome::Updated | Outcome::FieldsUpdated(_) => {
            format!("{} -> {}", entry.safix, entry.entry)
        }
        Outcome::Pulled => format!("{} -> {}", entry.entry, entry.safix),
        _ => format!("{} <-> {}", entry.safix, entry.entry),
    }
}

/// The paragraph under a pass mapping whose two sides both moved.
fn push_pass_conflict(out: &mut String, entry: &safix_core::pass::Converged) {
    use safix_core::model::PassMode;

    out.push('\n');
    if entry.mode == PassMode::Backup {
        detail(
            out,
            &format!(
                "{} holds a value that is not {}'s, and backup never overwrites one.",
                entry.entry, entry.safix
            ),
        );
        detail(
            out,
            "Nothing was written, and no field either. Either accept the store's value,",
        );
        detail(out, "or declare");
        remedy(out, "mode = \"safix-to-pass\";");
        detail(out, "on that mapping, which makes the store follow safix.");
        return;
    }
    detail(
        out,
        &format!(
            "{} and {} have both changed since the last agreement.",
            entry.safix, entry.entry
        ),
    );
    detail(
        out,
        "Nothing was written, and nothing here decides which of the two was meant:",
    );
    detail(
        out,
        "last-writer-wins over secrets rewards whichever clock lied best.",
    );
    remedy(out, "to keep safix's value, declare on this mapping:");
    remedy(out, "    mode = \"safix-to-pass\";");
    remedy(out, "to keep the store's, declare instead:");
    remedy(out, "    mode = \"pass-to-safix\";");
    remedy(
        out,
        &format!(
            "then:  {PROGRAM} sync pass {mapping}",
            mapping = entry.mapping
        ),
    );
    remedy(out, "and put the mode back to two-way afterwards.");
}

/// The paragraph under a pass mapping that needs a person.
fn push_pass_detail(out: &mut String, entry: &safix_core::pass::Converged) {
    use safix_core::model::PassMode;
    use safix_core::pass::Outcome;

    match &entry.outcome {
        Outcome::Conflict => push_pass_conflict(out, entry),

        Outcome::Refused(reason) | Outcome::NotJudged(reason) => {
            out.push('\n');
            for line in reason.to_string().lines() {
                detail(out, line);
            }
            out.push('\n');
        }

        Outcome::FieldsUpdated(fields) => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} already held {}'s value; the declared fields written are: {}.",
                    entry.entry,
                    entry.safix,
                    named_fields(fields),
                ),
            );
            detail(
                out,
                "One write, carrying the value the record already held: the whole body",
            );
            detail(
                out,
                "crosses on one pipe, so a field repair is one write and not two.",
            );
        }

        Outcome::FieldsDiverged(fields) => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} and {} agree on the value; these declared fields differ: {}.",
                    entry.safix,
                    entry.entry,
                    named_fields(fields),
                ),
            );
            detail(out, "Nothing was written.");
            match entry.mode {
                PassMode::PassToSafix => {
                    detail(
                        out,
                        "This mode writes safix rather than the store, and a field has one",
                    );
                    detail(
                        out,
                        "author: the declaration. Edit the declaration to say what the record",
                    );
                    detail(out, "holds, or declare a mode that writes the store:");
                    remedy(out, "mode = \"safix-to-pass\";");
                }
                PassMode::Backup => {
                    detail(
                        out,
                        "backup never overwrites an existing entry, and writing its fields while",
                    );
                    detail(out, "refusing its value would make backup half a mode.");
                }
                PassMode::SafixToPass | PassMode::TwoWay => {
                    remedy(
                        out,
                        &format!("{PROGRAM} sync pass {mapping}", mapping = entry.mapping),
                    );
                }
            }
        }

        Outcome::Unchanged | Outcome::Updated | Outcome::Pulled => {}
    }
}

/// What a 1password sync run did, one line per declared mapping.
///
/// The shape [`sync`] has. The difference is the closing line's subject — an
/// account rather than a database — and that a refusal here is one mapping's:
/// a failure against the service refuses its own mapping and the run goes on,
/// so every declared mapping has a line whatever happened.
#[must_use]
pub fn onepassword(report: &safix_core::onepassword::Report) -> String {
    let mut out = String::new();
    if report.converged.is_empty() {
        let empty = format!("{PROGRAM}: no mapping is declared.\n");
        out.push_str(&empty);
        return out;
    }

    for entry in &report.converged {
        let line = format!(
            "{PROGRAM}: {mapping}  {flow}  {mode}  {outcome}\n",
            mapping = entry.mapping,
            flow = onepassword_flow(entry),
            mode = entry.mode,
            outcome = entry.outcome.as_str(),
        );
        out.push_str(&line);
        push_onepassword_detail(&mut out, entry);
    }

    push_onepassword_lingering(&mut out, &report.lingering);

    let counting = |wanted: &str| {
        report
            .converged
            .iter()
            .filter(|entry| entry.outcome.as_str() == wanted)
            .count()
    };
    let fields_updated = counted("fields updated", counting("fields updated"));
    let fields_diverged = counted("fields diverged", counting("fields diverged"));
    let closing = format!(
        "{PROGRAM}: {total} mapping(s) against {account}: {} updated, {} pulled, {} unchanged, \
         {} conflict, {} refused, {} not judged{fields_updated}{fields_diverged}.\n",
        counting("updated"),
        counting("pulled"),
        counting("unchanged"),
        counting("conflict"),
        counting("refused"),
        counting("not judged"),
        total = report.converged.len(),
        account = onepassword_account(report.account.as_deref()),
    );
    out.push_str(&closing);
    out
}

/// One mapping's endpoints, in the order the value moved between them.
fn onepassword_flow(entry: &safix_core::onepassword::Converged) -> String {
    use safix_core::onepassword::Outcome;
    match entry.outcome {
        Outcome::Updated | Outcome::FieldsUpdated(_) => {
            format!("{} -> {}", entry.safix, entry.item)
        }
        Outcome::Pulled => format!("{} -> {}", entry.item, entry.safix),
        _ => format!("{} <-> {}", entry.safix, entry.item),
    }
}

/// The paragraph under a 1password mapping that needs a person.
fn push_onepassword_detail(out: &mut String, entry: &safix_core::onepassword::Converged) {
    use safix_core::model::OnePasswordMode;
    use safix_core::onepassword::Outcome;

    match &entry.outcome {
        Outcome::Conflict if entry.mode == OnePasswordMode::Backup => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} holds a value that is not {}'s, and backup never overwrites one.",
                    entry.item, entry.safix
                ),
            );
            detail(
                out,
                "Nothing was written. Either accept the item's value, or declare",
            );
            remedy(out, "mode = \"safix-to-1password\";");
            detail(out, "on that mapping, which makes the item follow safix.");
        }

        Outcome::Conflict => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} and {} have both changed since the last agreement.",
                    entry.safix, entry.item
                ),
            );
            detail(
                out,
                "Nothing was written, and nothing here decides which of the two was meant:",
            );
            detail(
                out,
                "last-writer-wins over secrets rewards whichever clock lied best.",
            );
            remedy(out, "to keep safix's value, declare on this mapping:");
            remedy(out, "    mode = \"safix-to-1password\";");
            remedy(out, "to keep the item's, declare instead:");
            remedy(out, "    mode = \"1password-to-safix\";");
            remedy(
                out,
                &format!(
                    "then:  {PROGRAM} sync 1password {mapping}",
                    mapping = entry.mapping
                ),
            );
            remedy(out, "and put the mode back to two-way afterwards.");
        }

        Outcome::Refused(reason) | Outcome::NotJudged(reason) => {
            out.push('\n');
            for line in reason.to_string().lines() {
                detail(out, line);
            }
            out.push('\n');
        }

        Outcome::FieldsUpdated(fields) => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} already held {}'s value; the declared fields written are: {}.",
                    entry.item,
                    entry.safix,
                    named_fields(fields),
                ),
            );
            detail(
                out,
                "One write, carrying the value the item already held: the whole item",
            );
            detail(
                out,
                "crosses standard input, so a field repair is one edit and not two.",
            );
        }

        Outcome::FieldsDiverged(fields) => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} and {} agree on the value; these declared fields differ: {}.",
                    entry.safix,
                    entry.item,
                    named_fields(fields),
                ),
            );
            detail(out, "Nothing was written.");
            onepassword_remedy(out, entry.mode, &entry.mapping);
        }

        Outcome::Unchanged | Outcome::Updated | Outcome::Pulled => {}
    }
}

/// What a bitwarden sync run did, one line per declared mapping.
///
/// The shape [`sync`] has. The differences are the closing line's subject — a
/// server rather than a database — and that a refusal here is one mapping's: a
/// failure against one item refuses its own mapping and the run goes on, so
/// every declared mapping has a line whatever happened. A refusal that is the
/// run's rather than a mapping's — a locked client, a server that is not the
/// one reached, a failed refresh — never reaches this function at all.
#[must_use]
pub fn bitwarden(report: &safix_core::bitwarden::Report) -> String {
    let mut out = String::new();
    if report.converged.is_empty() {
        let empty = format!("{PROGRAM}: no mapping is declared.\n");
        out.push_str(&empty);
        return out;
    }

    for entry in &report.converged {
        let line = format!(
            "{PROGRAM}: {mapping}  {flow}  {mode}  {outcome}\n",
            mapping = entry.mapping,
            flow = bitwarden_flow(entry),
            mode = entry.mode,
            outcome = entry.outcome.as_str(),
        );
        out.push_str(&line);
        push_bitwarden_detail(&mut out, entry);
    }

    push_bitwarden_lingering(&mut out, &report.lingering);

    let tally = report.tally();
    let fields_updated = counted("fields updated", tally.fields_updated);
    let fields_diverged = counted("fields diverged", tally.fields_diverged);
    let closing = format!(
        "{PROGRAM}: {total} mapping(s) against {server}: {} updated, {} pulled, {} unchanged, \
         {} conflict, {} refused, {} not judged{fields_updated}{fields_diverged}.\n",
        tally.updated,
        tally.pulled,
        tally.unchanged,
        tally.conflict,
        tally.refused,
        tally.not_judged,
        total = report.converged.len(),
        server = bitwarden_server(&report.server),
    );
    out.push_str(&closing);
    out
}

/// How a report names the server it converged against.
///
/// An undeclared server is a working configuration rather than a missing
/// declaration — the client already holds that configuration — so the report
/// says which posture it was rather than printing an empty string where a URL
/// would be.
fn bitwarden_server(server: &str) -> String {
    if server.is_empty() {
        return String::from("the server the client is configured against");
    }
    server.to_owned()
}

/// One mapping's endpoints, in the order the value moved between them.
fn bitwarden_flow(entry: &safix_core::bitwarden::Converged) -> String {
    use safix_core::sync::Outcome;
    match entry.outcome {
        Outcome::Updated | Outcome::FieldsUpdated(_) => {
            format!("{} -> {}", entry.safix, entry.address)
        }
        Outcome::Pulled => format!("{} -> {}", entry.address, entry.safix),
        _ => format!("{} <-> {}", entry.safix, entry.address),
    }
}

/// The paragraph under a bitwarden mapping that needs a person.
fn push_bitwarden_detail(out: &mut String, entry: &safix_core::bitwarden::Converged) {
    use safix_core::model::BitwardenMode;
    use safix_core::sync::Outcome;

    match &entry.outcome {
        Outcome::Conflict if entry.mode == BitwardenMode::Backup => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} holds a value that is not {}'s, and backup never overwrites one.",
                    entry.address, entry.safix
                ),
            );
            detail(
                out,
                "Nothing was written. Either accept the vault's value, or declare",
            );
            remedy(out, "mode = \"safix-to-bitwarden\";");
            detail(out, "on that mapping, which makes the vault follow safix.");
        }

        Outcome::Conflict => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} and {} have both changed since the last agreement.",
                    entry.safix, entry.address
                ),
            );
            detail(
                out,
                "Nothing was written, and nothing here decides which of the two was meant:",
            );
            detail(
                out,
                "last-writer-wins over secrets rewards whichever clock lied best.",
            );
            remedy(out, "to keep safix's value, declare on this mapping:");
            remedy(out, "    mode = \"safix-to-bitwarden\";");
            remedy(out, "to keep the vault's, declare instead:");
            remedy(out, "    mode = \"bitwarden-to-safix\";");
            remedy(
                out,
                &format!(
                    "then:  {PROGRAM} sync bitwarden {mapping}",
                    mapping = entry.mapping
                ),
            );
            remedy(out, "and put the mode back to two-way afterwards.");
        }

        Outcome::Refused(reason) | Outcome::NotJudged(reason) => {
            out.push('\n');
            for line in reason.to_string().lines() {
                detail(out, line);
            }
            out.push('\n');
        }

        Outcome::FieldsUpdated(fields) => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} already held {}'s value; the declared fields written are: {}.",
                    entry.address,
                    entry.safix,
                    named_fields(fields),
                ),
            );
            detail(
                out,
                "One write, carrying the value the item already held: this client replaces",
            );
            detail(
                out,
                "a whole item, so a field repair is one edit and not two.",
            );
        }

        Outcome::FieldsDiverged(fields) => {
            out.push('\n');
            detail(
                out,
                &format!(
                    "{} and {} agree on the value; these declared fields differ: {}.",
                    entry.safix,
                    entry.address,
                    named_fields(fields),
                ),
            );
            detail(out, "Nothing was written.");
            bitwarden_remedy(out, entry.mode, &entry.mapping);
        }

        Outcome::Unchanged | Outcome::Updated | Outcome::Pulled => {}
    }
}

/// The bitwarden target's section of an audit report.
///
/// One line per compared mapping, agreeing included, the way
/// [`push_keepassxc_audit`] lists every one of them.
fn push_bitwarden_audit(out: &mut String, report: &audit::BitwardenReport) {
    use audit::BitwardenOutcome;

    if report.compared.is_empty() {
        out.push_str(PROGRAM);
        out.push_str(": no mapping is declared.\n");
        return;
    }

    for entry in &report.compared {
        let line = format!(
            "{PROGRAM}: {mapping}  {safix} <-> {address}  {mode}  {outcome}\n",
            mapping = entry.mapping,
            safix = entry.safix,
            address = entry.address,
            mode = entry.mode,
            outcome = entry.outcome.as_str(),
        );
        out.push_str(&line);
        match &entry.outcome {
            BitwardenOutcome::Diverged => {
                bitwarden_remedy(out, entry.mode, &entry.mapping);
            }
            BitwardenOutcome::FieldsDiverged(fields) => {
                out.push('\n');
                detail(
                    out,
                    &format!(
                        "The two sides agree on the value; these declared fields differ: {}.",
                        named_fields(fields),
                    ),
                );
                bitwarden_remedy(out, entry.mode, &entry.mapping);
                out.push('\n');
            }
            BitwardenOutcome::Unjudgeable(reason) => {
                out.push('\n');
                for line in reason.to_string().lines() {
                    detail(out, line);
                }
                out.push('\n');
            }
            BitwardenOutcome::Agreeing => {}
        }
    }

    push_bitwarden_lingering(out, &report.lingering);

    let counting = |wanted: &str| {
        report
            .compared
            .iter()
            .filter(|entry| entry.outcome.as_str() == wanted)
            .count()
    };
    let fields = counted("fields diverged", counting("fields diverged"));
    let closing = format!(
        "{PROGRAM}: {total} mapping(s) against {server}: {agreeing} agreeing, {diverged} \
         diverged, {unjudgeable} unjudgeable{fields}.\n",
        total = report.compared.len(),
        server = bitwarden_server(&report.server),
        agreeing = counting("agreeing"),
        diverged = counting("diverged"),
        unjudgeable = counting("unjudgeable"),
    );
    out.push_str(&closing);
}

/// What a diverged bitwarden mapping's remedy is, which the mode decides.
///
/// A pushing mode resolves it by running again. `bitwarden-to-safix` cannot:
/// for that mode converging toward the declaration is what the mode already
/// means, so the declaration is the author and the remedy is to edit it.
/// `backup` refuses to touch an existing item on purpose.
fn bitwarden_remedy(out: &mut String, mode: safix_core::model::BitwardenMode, mapping: &str) {
    use safix_core::model::BitwardenMode;

    match mode {
        BitwardenMode::BitwardenToSafix => {
            detail(
                out,
                "This mode writes safix rather than the item, and a field has one author:",
            );
            detail(
                out,
                "the declaration. Edit the declaration to say what the item holds, or",
            );
            detail(out, "declare a mode that writes the item:");
            remedy(out, "mode = \"safix-to-bitwarden\";");
        }
        BitwardenMode::Backup => {
            detail(
                out,
                "backup never overwrites an existing item, and writing its fields while",
            );
            detail(out, "refusing its value would make backup half a mode.");
        }
        BitwardenMode::SafixToBitwarden | BitwardenMode::TwoWay => {
            remedy(out, &format!("{PROGRAM} sync bitwarden {mapping}"));
        }
    }
}

/// Items under a declared folder that no declared mapping accounts for.
///
/// Placed the way [`push_lingering`] is: after the section's own lines and
/// before its closing line. No companion counterpart to distinguish, because
/// this target's memory is a hidden field of the mapped item rather than a
/// second object beside it, so every entry renders the same way.
fn push_bitwarden_lingering(out: &mut String, entries: &[String]) {
    for entry in entries {
        out.push('\n');
        detail(
            out,
            &format!("{entry} is under a declared folder and no mapping declares it."),
        );
        detail(
            out,
            "A mapping removed after this item was created leaves it looking exactly",
        );
        detail(
            out,
            "like this, and so does an item a person put in the vault themselves \u{2014} the",
        );
        detail(out, "declarations cannot tell the two apart.");
        detail(out, "Nothing here will remove it; a person does that.");
    }
}

#[cfg(test)]
mod tests {
    use safix_core::audit::{ClanReport, Report};

    use super::*;

    #[test]
    fn a_clan_lingering_entry_renders_after_the_findings_and_names_no_value() {
        let report = Report {
            clan: Some(ClanReport {
                examined: 1,
                findings: Vec::new(),
                lingering: vec!["meridian ntfy/orphan".into()],
            }),
            keepassxc: None,
            pass: None,
            bitwarden: None,
            onepassword: None,
        };
        let rendered = audit(&report);
        assert!(rendered.contains(
            "meridian ntfy/orphan is a clan var and no declared mapping accounts for it."
        ));
        assert!(rendered.contains("no disagreement. All 1 declared mapping(s) agree."));
    }
}
