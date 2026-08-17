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

/// Add one recipient to the person's `recoveryRecipients`.
///
/// The list is created when it is absent, which is the ordinary case: the
/// scaffold leaves it out deliberately, and a card is the first thing that
/// belongs in it.
#[must_use]
pub fn add_recovery_recipient(declaration: &str, recipient: &str) -> Edit {
    if quoted_present(declaration, recipient) {
        return Edit::AlreadyPresent;
    }
    match insert_into_list(declaration, "recoveryRecipients", recipient) {
        Some(edited) => Edit::Inserted(edited),
        None => create_recovery_recipients(declaration, recipient),
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
    match insert_into_attrset(declaration, "private", name) {
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

/// Insert a quoted element into an existing list-valued attribute.
///
/// `None` when the attribute is not declared, which is the caller's cue to make
/// it rather than this function's to guess at where it would go.
fn insert_into_list(declaration: &str, attribute: &str, element: &str) -> Option<String> {
    let lines: Vec<&str> = declaration.lines().collect();
    let opening = lines.iter().position(|line| declares(line, attribute))?;
    let opening_line = lines.get(opening)?;

    // The one-line form: the list opens and closes on the attribute's own line,
    // so the element goes in front of the bracket that closes it. Nix separates
    // list elements by whitespace, so no comma is involved and nothing existing
    // is touched.
    if let Some(bracket) = opening_line.rfind(']')
        && opening_line.contains('[')
    {
        let (before, after) = opening_line.split_at(bracket);
        let mut rebuilt: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
        let replacement = format!("{}\"{element}\" {after}", pad(before));
        *rebuilt.get_mut(opening)? = replacement;
        return Some(joined(&rebuilt, declaration));
    }

    // The multi-line form: the element goes on its own line above the one that
    // closes the list, indented like whatever is already inside it.
    let closing = lines
        .iter()
        .enumerate()
        .skip(opening)
        .find(|(_, line)| line.trim_start().starts_with(']'))
        .map(|(index, _)| index)?;
    // Indented like whatever is already inside the list, and one step in from the
    // attribute when nothing is: the line that closes it is not an element and its
    // indentation is the list's own rather than its contents'.
    let inner = lines
        .get(opening.saturating_add(1)..closing)
        .and_then(|inside| inside.iter().find(|line| !line.trim().is_empty()))
        .map_or_else(
            || format!("{}  ", indent_of(opening_line)),
            |line| indent_of(line),
        );

    let mut rebuilt: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
    rebuilt.insert(closing, format!("{inner}\"{element}\""));
    Some(joined(&rebuilt, declaration))
}

/// Insert an empty entry into an existing attrset-valued attribute.
fn insert_into_attrset(declaration: &str, attribute: &str, name: &str) -> Option<String> {
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
        let entry = format!("{indent}  {name} = {{ }};");
        let closed = format!("{indent}}};");
        rebuilt.splice(opening..opening.saturating_add(1), [opened, entry, closed]);
        return Some(joined(&rebuilt, declaration));
    }

    let closing = lines
        .iter()
        .enumerate()
        .skip(opening)
        .find(|(_, line)| line.trim_start().starts_with('}'))
        .map(|(index, _)| index)?;
    let mut rebuilt: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
    rebuilt.insert(closing, format!("{indent}  {name} = {{ }};"));
    Some(joined(&rebuilt, declaration))
}

/// Write a `recoveryRecipients` list where the record has none.
///
/// Anchored under the `recipient` line, because that is the field it is the
/// counterpart of and a reader looking for one will be looking at the other. The
/// comment is two lines and states the one thing the field's presence does not:
/// that adding to it is additive and removing from it revokes nothing.
fn create_recovery_recipients(declaration: &str, recipient: &str) -> Edit {
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
        format!("{indent}recoveryRecipients = [ \"{recipient}\" ];"),
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

/// The text with one space at the end, so an inserted element does not collide
/// with what precedes it.
fn pad(before: &str) -> String {
    if before.ends_with(char::is_whitespace) || before.is_empty() {
        return before.to_owned();
    }
    format!("{before} ")
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

    fn inserted(edit: Edit) -> String {
        match edit {
            Edit::Inserted(text) => text,
            other => unreachable!("expected an insertion, got {other:?}"),
        }
    }

    #[test]
    fn a_record_with_no_recovery_list_gets_one_holding_the_card() {
        let edited = inserted(add_recovery_recipient(SCAFFOLD, CARD));
        assert!(edited.contains(&format!("recoveryRecipients = [ \"{CARD}\" ];")));
        assert!(
            edited.contains("recipient = \"age1software\";"),
            "the primary recipient was disturbed"
        );
        assert!(edited.contains("carries = { };"), "a bystander was lost");
        // The list is anchored below the field it is the counterpart of.
        let recipient_at = edited.find("recipient = \"age1software\"").unwrap();
        let recovery_at = edited.find("recoveryRecipients").unwrap();
        assert!(recovery_at > recipient_at);
    }

    #[test]
    fn a_second_card_joins_the_first_and_the_first_stays() {
        let first = inserted(add_recovery_recipient(SCAFFOLD, CARD));
        let second = inserted(add_recovery_recipient(&first, "age1yubikey1qbackup"));
        assert!(second.contains(&format!("\"{CARD}\"")));
        assert!(second.contains("\"age1yubikey1qbackup\""));
        assert_eq!(
            second.matches("recoveryRecipients").count(),
            1,
            "a second list was written instead of the first being extended"
        );
    }

    #[test]
    fn the_same_card_twice_writes_nothing() {
        let first = inserted(add_recovery_recipient(SCAFFOLD, CARD));
        assert_eq!(add_recovery_recipient(&first, CARD), Edit::AlreadyPresent);
    }

    #[test]
    fn a_multi_line_list_gains_a_line_and_keeps_its_indentation() {
        let record = "\
{
  flake.safix.users.ana = {
    recipient = \"age1software\";
    recoveryRecipients = [
      \"age1existing\"
    ];
  };
}
";
        let edited = inserted(add_recovery_recipient(record, CARD));
        assert!(edited.contains(&format!("      \"{CARD}\"")));
        assert!(edited.contains("      \"age1existing\""));
        assert!(edited.contains("    ];"));
    }

    #[test]
    fn an_empty_multi_line_list_gains_its_first_element() {
        let record = "\
{
  flake.safix.users.ana = {
    recipient = \"age1software\";
    recoveryRecipients = [
    ];
  };
}
";
        let edited = inserted(add_recovery_recipient(record, CARD));
        assert!(edited.contains(&format!("      \"{CARD}\"")));
    }

    #[test]
    fn a_record_with_no_recipient_has_nowhere_to_add_one() {
        assert_eq!(add_recovery_recipient("{ }\n", CARD), Edit::NoAnchor);
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
        let edited = inserted(add_recovery_recipient(SCAFFOLD, CARD));
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
