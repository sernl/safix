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
                "No mode deletes an entry, so this is what a mapping that was removed",
            );
            detail(out, "leaves behind.");
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
            "No mode deletes a clan var, so this is what a mapping that was removed",
        );
        detail(
            out,
            "leaves behind. A person removes it, with clan's own command.",
        );
    }
}

/// The keepassxc target's section of an audit report.
///
/// One line per compared mapping — agreeing included, the way [`sync`]'s own
/// report lists every mapping rather than only the ones that need a person —
/// because a keepassxc mapping's outcome is one of exactly three words rather
/// than the clan target's open-ended disagreement, and a report that named
/// all three is as short as one that named only two of them.
fn push_keepassxc_audit(out: &mut String, report: &audit::KeepassxcReport) {
    use audit::KeepassxcOutcome;

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
    let unjudgeable = report
        .compared
        .iter()
        .filter(|entry| matches!(entry.outcome, KeepassxcOutcome::Unjudgeable(_)))
        .count();
    let closing = format!(
        "{PROGRAM}: {total} mapping(s) against {database}: {agreeing} agreeing, {diverged} \
         diverged, {unjudgeable} unjudgeable.\n",
        total = report.compared.len(),
        database = report.database,
    );
    out.push_str(&closing);
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
    let closing = format!(
        "{PROGRAM}: {total} mapping(s) against {database}: {} updated, {} pulled, {} unchanged, \
         {} conflict, {} refused, {} not judged.\n",
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
/// about a transfer.
fn sync_flow(entry: &safix_core::sync::Converged) -> String {
    use safix_core::sync::Outcome;
    match entry.outcome {
        Outcome::Updated => format!("{} -> {}", entry.safix, entry.kdbx),
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

        Outcome::Unchanged | Outcome::Updated | Outcome::Pulled => {}
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
        };
        let rendered = audit(&report);
        assert!(rendered.contains(
            "meridian ntfy/orphan is a clan var and no declared mapping accounts for it."
        ));
        assert!(rendered.contains("no disagreement. All 1 declared mapping(s) agree."));
    }
}
