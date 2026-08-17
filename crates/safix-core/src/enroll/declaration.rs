//! Additive edits to a person's custody record.
//!
//! Two edits, both to `safix/users/<name>.nix`, both additive by construction:
//! a recipient onto `recoveryRecipients`, and a name onto `private`. Neither can
//! remove or replace anything, because neither is capable of writing a line where
//! one already is — the transforms below only ever insert.
//!
//! # Why this is text and not a nix evaluation
//!
//! The file being edited is a file a person reads. An evaluation would produce a
//! value, and writing that value back would reformat the whole record and lose
//! every comment in it, which for a custody record is most of what it says. So
//! the edits are line insertions, and what holds them is that the result is
//! parsed by the real parser before anything is staged — the discipline
//! [`adduser`](crate::adduser) already applies to the record it writes.
//!
//! # What an already-present value does
//!
//! Nothing, and it reports that it did nothing. Enrolling the same card twice is
//! not an error and must not append a second copy of one recipient: the run's
//! remaining work — the re-wrap, the registration, the proof — is worth doing
//! again, and the design's own migration note is that a card already enrolled by
//! hand is exactly what the first real run meets.

/// What an insertion did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// The value was not there and now is; this is the whole file.
    Inserted(String),
    /// The value was already there, so nothing was written.
    AlreadyPresent,
    /// There was nowhere to put it: the file declares no `recipient`, so it is
    /// not a custody record this knows how to extend.
    NoAnchor,
}

/// Add one recipient to the person's `recoveryRecipients`, keyed by its anchor.
///
/// The attrset is created when it is absent, which is the ordinary case: the
/// scaffold leaves it out deliberately, and a card is the first thing that
/// belongs in it. The shape written is the option's own — an anchor naming a
/// `key` — because this file's next reader is the module system, and a shape
/// only a human accepts is an enrollment that fails at the next evaluation.
/// `modules/flake/checks/fixture-fleet.nix` holds the same shape verbatim
/// against the real option, so the two cannot drift apart silently.
#[must_use]
pub fn add_recovery_recipient(declaration: &str, anchor: &str, recipient: &str) -> Edit {
    if quoted_present(declaration, recipient) {
        return Edit::AlreadyPresent;
    }
    let entry = format!("\"{anchor}\".key = \"{recipient}\";");
    match recovery_shape(declaration) {
        Shape::Absent => create_recovery_recipients(declaration, &entry),
        Shape::Attrset => match insert_into_attrset(declaration, "recoveryRecipients", &entry) {
            Some(edited) => Edit::Inserted(edited),
            None => Edit::NoAnchor,
        },
        Shape::Unextendable => Edit::NoAnchor,
    }
}

/// What the file already declares `recoveryRecipients` as.
enum Shape {
    /// Not declared; the block is created whole.
    Absent,
    /// The option's own attrset form, extendable in place.
    Attrset,
    /// Some other value — a list, say — which the option would refuse at
    /// evaluation anyway; extending it would compound a hand edit this editor
    /// does not understand, so it is refused as having no anchor instead.
    Unextendable,
}

fn recovery_shape(declaration: &str) -> Shape {
    let Some(line) = declaration
        .lines()
        .find(|line| declares(line, "recoveryRecipients"))
    else {
        return Shape::Absent;
    };
    if line.contains('[') {
        Shape::Unextendable
    } else {
        Shape::Attrset
    }
}

/// Add one name to the person's `private` set, as an entry declaring nothing.
///
/// What makes a name a secret the write path can resolve. It declares where the
/// value lives and not that one is there, which is the state every first `set`
/// writes into.
#[must_use]
pub fn add_private_entry(declaration: &str, name: &str) -> Edit {
    if attribute_present(declaration, name) {
        return Edit::AlreadyPresent;
    }
    match insert_into_attrset(declaration, "private", &format!("{name} = {{ }};")) {
        Some(edited) => Edit::Inserted(edited),
        None => Edit::NoAnchor,
    }
}

/// Whether a quoted string is already in the file.
fn quoted_present(declaration: &str, value: &str) -> bool {
    declaration.contains(&format!("\"{value}\""))
}

/// Whether an attribute of this name is already declared anywhere in the file.
fn attribute_present(declaration: &str, name: &str) -> bool {
    declaration.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed
            .strip_prefix(name)
            .is_some_and(|rest| rest.trim_start().starts_with('='))
    })
}

/// Insert one rendered entry into an existing attrset-valued attribute.
///
/// `None` when the attribute is not declared, which is the caller's cue to make
/// it rather than this function's to guess at where it would go. The entry
/// arrives rendered but unindented, so one inserter serves an empty private set
/// and a recovery anchor alike.
fn insert_into_attrset(declaration: &str, attribute: &str, entry: &str) -> Option<String> {
    let lines: Vec<&str> = declaration.lines().collect();
    let opening = lines.iter().position(|line| declares(line, attribute))?;
    let opening_line = lines.get(opening)?;
    let indent = indent_of(opening_line);

    // `private = { };` — the empty form the scaffold writes. It becomes the
    // multi-line form holding one entry, which is what it looks like once
    // somebody holds something.
    if opening_line.contains('{') && opening_line.contains('}') {
        let mut rebuilt: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
        let opened = format!("{indent}{attribute} = {{");
        let entry_line = format!("{indent}  {entry}");
        let closed = format!("{indent}}};");
        rebuilt.splice(
            opening..opening.saturating_add(1),
            [opened, entry_line, closed],
        );
        return Some(joined(&rebuilt, declaration));
    }

    let closing = lines
        .iter()
        .enumerate()
        .skip(opening)
        .find(|(_, line)| line.trim_start().starts_with('}'))
        .map(|(index, _)| index)?;
    let mut rebuilt: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
    rebuilt.insert(closing, format!("{indent}  {entry}"));
    Some(joined(&rebuilt, declaration))
}

/// Write a `recoveryRecipients` attrset where the record has none.
///
/// Anchored under the `recipient` line, because that is the field it is the
/// counterpart of and a reader looking for one will be looking at the other. The
/// comment is two lines and states the one thing the field's presence does not:
/// that adding to it is additive and removing from it revokes nothing.
fn create_recovery_recipients(declaration: &str, entry: &str) -> Edit {
    let lines: Vec<&str> = declaration.lines().collect();
    let Some(anchor) = lines.iter().position(|line| declares(line, "recipient")) else {
        return Edit::NoAnchor;
    };
    let Some(anchor_line) = lines.get(anchor) else {
        return Edit::NoAnchor;
    };
    let indent = indent_of(anchor_line);

    let block = [
        String::new(),
        format!("{indent}# Every key that can also open what this person holds. Additive: a"),
        format!("{indent}# recipient added here reaches every file their audience covers at the"),
        format!("{indent}# next re-wrap, and one removed from here revokes nothing that was"),
        format!("{indent}# already readable. Cards belong here and not in `recipient`, because"),
        format!("{indent}# activation decrypts without a person present and a card needs a touch."),
        format!("{indent}recoveryRecipients = {{"),
        format!("{indent}  {entry}"),
        format!("{indent}}};"),
    ];

    let mut rebuilt: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
    let after = anchor.saturating_add(1);
    for (offset, line) in block.into_iter().enumerate() {
        rebuilt.insert(after.saturating_add(offset), line);
    }
    Edit::Inserted(joined(&rebuilt, declaration))
}

/// Whether this line declares that attribute.
fn declares(line: &str, attribute: &str) -> bool {
    line.trim_start()
        .strip_prefix(attribute)
        .is_some_and(|rest| rest.trim_start().starts_with('='))
}

/// The leading whitespace of a line, as a string.
fn indent_of(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

/// The lines back into one document, keeping whatever the original ended with.
fn joined(lines: &[String], original: &str) -> String {
    let mut text = lines.join("\n");
    if original.ends_with('\n') {
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The scaffold `adduser` writes, cut to the lines these edits touch.
    const SCAFFOLD: &str = "\
{
  flake.safix.users.ana = {
    # handed over by them.
    recipient = \"age1software\";

    carries = { };
    private = { };
  };
}
";

    const CARD: &str = "age1yubikey1qfixture";
    const ANCHOR: &str = "yubikey-12345678";

    fn inserted(edit: Edit) -> String {
        match edit {
            Edit::Inserted(text) => text,
            other => unreachable!("expected an insertion, got {other:?}"),
        }
    }

    #[test]
    fn a_record_with_no_recovery_set_gets_one_holding_the_card() {
        let edited = inserted(add_recovery_recipient(SCAFFOLD, ANCHOR, CARD));
        // The exact shape the option types: an anchor naming a key. The same
        // literal shape stands in `modules/flake/checks/fixture-fleet.nix`
        // against the real option, which is what binds this writer to it.
        assert!(edited.contains("    recoveryRecipients = {"));
        assert!(edited.contains(&format!("      \"{ANCHOR}\".key = \"{CARD}\";")));
        assert!(edited.contains("    };"));
        assert!(
            edited.contains("recipient = \"age1software\";"),
            "the primary recipient was disturbed"
        );
        assert!(edited.contains("carries = { };"), "a bystander was lost");
        // The set is anchored below the field it is the counterpart of.
        let recipient_at = edited.find("recipient = \"age1software\"").unwrap();
        let recovery_at = edited.find("recoveryRecipients").unwrap();
        assert!(recovery_at > recipient_at);
    }

    #[test]
    fn a_second_card_joins_the_first_and_the_first_stays() {
        let first = inserted(add_recovery_recipient(SCAFFOLD, ANCHOR, CARD));
        let second = inserted(add_recovery_recipient(
            &first,
            "yubikey-87654321",
            "age1yubikey1qbackup",
        ));
        assert!(second.contains(&format!("\"{ANCHOR}\".key = \"{CARD}\";")));
        assert!(second.contains("\"yubikey-87654321\".key = \"age1yubikey1qbackup\";"));
        assert_eq!(
            second.matches("recoveryRecipients").count(),
            1,
            "a second set was written instead of the first being extended"
        );
    }

    #[test]
    fn the_same_card_twice_writes_nothing() {
        let first = inserted(add_recovery_recipient(SCAFFOLD, ANCHOR, CARD));
        assert_eq!(
            add_recovery_recipient(&first, ANCHOR, CARD),
            Edit::AlreadyPresent
        );
    }

    #[test]
    fn a_hand_written_attrset_gains_an_anchor_and_keeps_its_own() {
        let record = "\
{
  flake.safix.users.ana = {
    recipient = \"age1software\";
    recoveryRecipients = {
      master = {
        key = \"age1master\";
      };
    };
  };
}
";
        let edited = inserted(add_recovery_recipient(record, ANCHOR, CARD));
        assert!(edited.contains(&format!("      \"{ANCHOR}\".key = \"{CARD}\";")));
        assert!(
            edited.contains("key = \"age1master\";"),
            "a bystander was lost"
        );
        assert_eq!(edited.matches("recoveryRecipients").count(), 1);
    }

    #[test]
    fn a_list_valued_declaration_is_refused_rather_than_compounded() {
        // Not the option's type, so evaluation would refuse it anyway; extending
        // it would bury that error under a second one.
        let record = "\
{
  flake.safix.users.ana = {
    recipient = \"age1software\";
    recoveryRecipients = [ \"age1existing\" ];
  };
}
";
        assert_eq!(add_recovery_recipient(record, ANCHOR, CARD), Edit::NoAnchor);
    }

    #[test]
    fn a_record_with_no_recipient_has_nowhere_to_add_one() {
        assert_eq!(
            add_recovery_recipient("{ }\n", ANCHOR, CARD),
            Edit::NoAnchor
        );
    }

    #[test]
    fn an_empty_private_set_becomes_one_holding_the_named_entry() {
        let edited = inserted(add_private_entry(SCAFFOLD, "card-12345678-piv-access"));
        assert!(edited.contains("    private = {"));
        assert!(edited.contains("      card-12345678-piv-access = { };"));
        assert!(edited.contains("    };"));
        assert!(edited.contains("carries = { };"), "a bystander was lost");
    }

    #[test]
    fn a_private_set_already_holding_entries_gains_one() {
        let record = "\
{
  flake.safix.users.ana = {
    recipient = \"age1software\";
    private = {
      mail-password = { };
    };
  };
}
";
        let edited = inserted(add_private_entry(record, "card-1-piv-access"));
        assert!(edited.contains("      mail-password = { };"));
        assert!(edited.contains("      card-1-piv-access = { };"));
    }

    #[test]
    fn a_private_entry_already_declared_writes_nothing() {
        let edited = inserted(add_private_entry(SCAFFOLD, "card-1-piv-access"));
        assert_eq!(
            add_private_entry(&edited, "card-1-piv-access"),
            Edit::AlreadyPresent
        );
    }

    #[test]
    fn neither_edit_ever_shortens_the_file() {
        let edited = inserted(add_recovery_recipient(SCAFFOLD, ANCHOR, CARD));
        let edited = inserted(add_private_entry(&edited, "card-1-piv-access"));
        for line in SCAFFOLD.lines().filter(|line| !line.trim().is_empty()) {
            if line.trim() == "private = { };" {
                continue;
            }
            assert!(
                edited.contains(line.trim()),
                "the edits lost a line: {line}"
            );
        }
    }
}
