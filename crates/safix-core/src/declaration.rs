//! Edits to the declarations a scaffolding verb writes into.
//!
//! Four of them, over two files. A recipient onto a person's
//! `recoveryRecipients` and a name onto their `private`, both in
//! `safix/users/<name>.nix`, which is what [`enroll`](crate::enroll) writes; and a
//! subject into or out of a group's `members` in `safix/groups/<group>.nix`, which
//! is what [`group`](crate::group) writes.
//!
//! Three of the four are additive by construction, because they are incapable of
//! writing a line where one already is — the transforms only insert. The fourth is
//! a removal, and it is the one edit here that takes a line away; it keeps its own
//! outcome vocabulary for that reason, see [`Removal`].
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
//! Parsed and not evaluated, which bounds what this can catch. A membership that
//! would form a cycle among groups parses; what refuses it is the evaluation the
//! next build performs, which names every participant, and that is where the model
//! puts the refusal deliberately — see the `members` option's own description.
//!
//! # What an already-present value does
//!
//! Nothing, and it reports that it did nothing. Enrolling the same card twice is
//! not an error and must not append a second copy of one recipient: the run's
//! remaining work — the re-wrap, the registration, the proof — is worth doing
//! again, and the design's own migration note is that a card already enrolled by
//! hand is exactly what the first real run meets. Adding a subject a group already
//! holds answers the same way, and so does removing one it does not.

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

/// What a removal did.
///
/// Its own vocabulary rather than two more variants on [`Edit`], because an
/// additive edit cannot produce either of the first two: one enum over both acts
/// would give every caller arms no act of theirs can reach, and an unreachable
/// arm is a place a later change lands unnoticed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Removal {
    /// The value was there and now is not; this is the whole file.
    Removed(String),
    /// The value was not there, so nothing was written.
    NotPresent,
    /// There was nothing to remove from: no declaration this understands.
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

/// Add one subject to a group's `members`.
///
/// The list is the shape a group declaration has: a `members` list of subject
/// names, which is the shape `modules/flake/checks/fixture-fleet.nix` holds
/// against the real option. A list already holding names becomes the one-per-line
/// form if it was not one already, so that this edit and every later one is a
/// single inserted line and every name that was there survives verbatim — a
/// formatter may collapse a one-name list back onto its line, and meeting that
/// again is a list to expand rather than a state this cannot edit.
#[must_use]
pub fn add_group_member(declaration: &str, group: &str, subject: &str) -> Edit {
    let lines: Vec<&str> = declaration.lines().collect();
    let Some(at) = members_line(&lines, group) else {
        return Edit::NoAnchor;
    };
    let Some(line) = lines.get(at) else {
        return Edit::NoAnchor;
    };
    let quoted = format!("\"{subject}\"");
    let indent = indent_of(line);
    let member = format!("{indent}  {quoted}");

    match list_shape(line) {
        List::Unextendable => Edit::NoAnchor,
        List::Empty => {
            let mut rebuilt: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
            rebuilt.splice(
                at..at.saturating_add(1),
                [opening_of(line), member, closing_of(line, indent.as_str())],
            );
            Edit::Inserted(joined(&rebuilt, declaration))
        }
        List::Inline => {
            if inline_members(line).iter().any(|held| *held == quoted) {
                return Edit::AlreadyPresent;
            }
            let mut written = vec![opening_of(line)];
            for held in inline_members(line) {
                written.push(format!("{indent}  {held}"));
            }
            written.push(member);
            written.push(closing_of(line, indent.as_str()));
            let mut rebuilt: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
            rebuilt.splice(at..at.saturating_add(1), written);
            Edit::Inserted(joined(&rebuilt, declaration))
        }
        List::Multiline => {
            let Some(closing) = closing_line(&lines, at) else {
                return Edit::NoAnchor;
            };
            if holds_member(&lines, at, closing, &quoted) {
                return Edit::AlreadyPresent;
            }
            let mut rebuilt: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
            rebuilt.insert(closing, member);
            Edit::Inserted(joined(&rebuilt, declaration))
        }
    }
}

/// Remove one subject from a group's `members`.
///
/// The one edit here that takes a line away, and the only one: what it removes is
/// the member's own line, or the member's own name from a list written on one
/// line. It removes nothing else, and it takes nothing back — a subject that has
/// been in a group has read what that group's audience could read, which is what
/// the verb printing this edit's disclosure says.
#[must_use]
pub fn remove_group_member(declaration: &str, group: &str, subject: &str) -> Removal {
    let lines: Vec<&str> = declaration.lines().collect();
    let Some(at) = members_line(&lines, group) else {
        return Removal::NoAnchor;
    };
    let Some(line) = lines.get(at) else {
        return Removal::NoAnchor;
    };
    let quoted = format!("\"{subject}\"");

    match list_shape(line) {
        List::Unextendable => Removal::NoAnchor,
        List::Empty => Removal::NotPresent,
        List::Inline => {
            let held = inline_members(line);
            if !held.iter().any(|member| *member == quoted) {
                return Removal::NotPresent;
            }
            let kept: Vec<String> = held
                .into_iter()
                .filter(|member| *member != quoted)
                .map(String::from)
                .collect();
            let declared = declared_of(line);
            let tail = tail_of(line);
            let rendered = if kept.is_empty() {
                format!("{declared} = [ ]{tail}")
            } else {
                format!("{declared} = [ {} ]{tail}", kept.join(" "))
            };
            let mut rebuilt: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
            rebuilt.splice(at..at.saturating_add(1), [rendered]);
            Removal::Removed(joined(&rebuilt, declaration))
        }
        List::Multiline => {
            let Some(closing) = closing_line(&lines, at) else {
                return Removal::NoAnchor;
            };
            let Some(member) = member_line(&lines, at, closing, &quoted) else {
                return Removal::NotPresent;
            };
            let mut rebuilt: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
            rebuilt.remove(member);
            Removal::Removed(joined(&rebuilt, declaration))
        }
    }
}

/// Add or replace one entry's rotation policy.
///
/// Two shapes are edited, because they are the two an entry is written in: a
/// block opening on the entry's own name, where the line goes inside it, and a
/// dotted declaration of one of the entry's fields, where the line goes beside
/// it carrying the same path. A file declaring neither is [`Edit::NoAnchor`],
/// which is the caller's cue to refuse rather than this function's to guess at
/// where a declaration would go.
///
/// Unlike the three insertions above this one, it may replace: a policy is one
/// value per entry, so assigning a second is the operator restating the first
/// rather than adding to a set. Restating the policy already declared writes
/// nothing and says so.
#[must_use]
pub fn set_entry_rotation(declaration: &str, name: &str, policy: &str) -> Edit {
    let lines: Vec<&str> = declaration.lines().collect();
    let rendered = format!("rotation = \"{policy}\";");

    if let Some((index, declared)) =
        rotation_line(&lines, name).and_then(|at| Some((at, *lines.get(at)?)))
    {
        if declared.trim_end().ends_with(&rendered) {
            return Edit::AlreadyPresent;
        }
        let mut edited: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
        if let Some(slot) = edited.get_mut(index) {
            *slot = format!("{}{rendered}", indent_of(declared));
        }
        return Edit::Inserted(joined(&edited, declaration));
    }

    if let Some((index, opening)) =
        block_line(&lines, name).and_then(|at| Some((at, *lines.get(at)?)))
    {
        let inner = format!("{}  {rendered}", indent_of(opening));
        return Edit::Inserted(inserted_after(&lines, index, &inner, declaration));
    }

    if let Some((index, path)) = dotted_field_line(&lines, name) {
        let indent = lines
            .get(index)
            .map_or_else(String::new, |line| indent_of(line));
        let beside = format!("{indent}{path}.{rendered}");
        return Edit::Inserted(inserted_after(&lines, index, &beside, declaration));
    }

    Edit::NoAnchor
}

/// Remove one entry's rotation policy.
///
/// The second removal in this module, and it takes the whole line away: a
/// policy written as `null` would read as a declared absence the resolver
/// treats identically, and leaving one behind would make an unset entry and a
/// never-set entry look different in a file a person reads.
#[must_use]
pub fn remove_entry_rotation(declaration: &str, name: &str) -> Removal {
    let lines: Vec<&str> = declaration.lines().collect();
    let Some(index) = rotation_line(&lines, name) else {
        return if block_line(&lines, name).is_some() || dotted_field_line(&lines, name).is_some() {
            Removal::NotPresent
        } else {
            Removal::NoAnchor
        };
    };
    let edited: Vec<String> = lines
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != index)
        .map(|(_, line)| (*line).to_owned())
        .collect();
    Removal::Removed(joined(&edited, declaration))
}

/// The lines with one inserted after `index`.
fn inserted_after(lines: &[&str], index: usize, line: &str, original: &str) -> String {
    let mut edited: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
    edited.insert(index.saturating_add(1), line.to_owned());
    joined(&edited, original)
}

/// Where this entry's `rotation` is declared, in either shape.
///
/// The dotted form is looked for first because it is unambiguous: the entry is
/// named on the line that declares the field. The block form's `rotation` is
/// the one inside the entry's own braces, so a second entry's policy in the
/// same file is never taken for this one.
fn rotation_line(lines: &[&str], name: &str) -> Option<usize> {
    let dotted = lines.iter().position(|line| {
        path_of(line).is_some_and(|path| {
            path.len() >= 2
                && path.last().is_some_and(|last| last == "rotation")
                && path
                    .get(path.len().saturating_sub(2))
                    .is_some_and(|at| at == name)
        })
    });
    if dotted.is_some() {
        return dotted;
    }
    let opening = block_line(lines, name)?;
    let closing = closing_brace(lines, opening)?;
    lines
        .iter()
        .enumerate()
        .skip(opening.saturating_add(1))
        .take(closing.saturating_sub(opening).saturating_sub(1))
        .find(|(_, line)| {
            path_of(line).is_some_and(|path| path.last().is_some_and(|last| last == "rotation"))
        })
        .map(|(at, _)| at)
}

/// Where the block declaring this entry opens, when the entry is written as
/// one.
fn block_line(lines: &[&str], name: &str) -> Option<usize> {
    lines.iter().position(|line| {
        line.trim_end().ends_with('{')
            && path_of(line).is_some_and(|path| path.last().is_some_and(|last| last == name))
    })
}

/// Where the block opened at `opening` closes: the first line at its own
/// indentation whose content begins with a closing brace.
fn closing_brace(lines: &[&str], opening: usize) -> Option<usize> {
    let indent = indent_of(lines.get(opening)?);
    lines
        .iter()
        .enumerate()
        .skip(opening.saturating_add(1))
        .find(|(_, line)| line.starts_with(&indent) && line[indent.len()..].starts_with('}'))
        .map(|(at, _)| at)
}

/// A dotted declaration of one of this entry's fields, and the attribute path
/// down to the entry itself.
///
/// A dotted `…private.api-token.generator.script = …` answers
/// `…private.api-token`, which is the path a sibling `rotation` line carries.
fn dotted_field_line(lines: &[&str], name: &str) -> Option<(usize, String)> {
    lines.iter().enumerate().find_map(|(at, line)| {
        let path = path_of(line)?;
        let position = path.iter().position(|part| part == name)?;
        if position.saturating_add(1) >= path.len() {
            return None;
        }
        Some((at, path.get(..=position)?.join(".")))
    })
}

/// The shape a `members` declaration's value is written in.
enum List {
    /// `members = [ ];` — the empty form, which becomes the one-per-line form
    /// holding the first name.
    Empty,
    /// `members = [ "alice" ];` — names on the declaration's own line.
    Inline,
    /// `members = [` — one name per line, closing on a line of its own.
    Multiline,
    /// Anything else: a merge, a function call, a value computed elsewhere. The
    /// option would take some of those, and this editor understands none of them,
    /// so extending one would compound an edit it cannot read — the reasoning
    /// [`Shape::Unextendable`] applies to a recovery set.
    Unextendable,
}

fn list_shape(line: &str) -> List {
    let Some((_, rest)) = line.split_once('=') else {
        return List::Unextendable;
    };
    let rest = rest.trim_start();
    let Some(inner) = rest.strip_prefix('[') else {
        return List::Unextendable;
    };
    match inner.rsplit_once(']') {
        None => List::Multiline,
        Some((held, _)) if held.trim().is_empty() => List::Empty,
        Some(_) => List::Inline,
    }
}

/// The line declaring one group's `members`, in either of the two shapes a
/// declaration writes it.
///
/// The dotted form — `oncall.members = [ … ]`, or the whole path down from
/// `flake.safix.groups` — is looked for first, because it is unambiguous: the
/// group is named on the line that declares the list. The nested form is a block
/// opening on the group's own name, whose first `members` declaration is that
/// group's.
///
/// A file declaring several groups is therefore edited correctly whichever shape
/// it uses, and a `members` line belonging to another group is never taken for
/// this one.
fn members_line(lines: &[&str], group: &str) -> Option<usize> {
    let dotted = lines.iter().position(|line| {
        path_of(line).is_some_and(|path| {
            path.len() >= 2
                && path.last().is_some_and(|last| last == "members")
                && path
                    .get(path.len().saturating_sub(2))
                    .is_some_and(|owner| owner == group)
        })
    });
    if dotted.is_some() {
        return dotted;
    }

    let opens = lines.iter().position(|line| {
        path_of(line).is_some_and(|path| path.last().is_some_and(|last| last == group))
            && line.contains('{')
    })?;
    lines
        .iter()
        .enumerate()
        .skip(opens)
        .find(|(_, line)| {
            path_of(line).is_some_and(|path| path.last().is_some_and(|last| last == "members"))
        })
        .map(|(index, _)| index)
}

/// The attribute path a line declares, if it declares one.
///
/// Quotes are stripped from each component, because `"oncall".members` and
/// `oncall.members` declare the same attribute and the option accepts both.
fn path_of(line: &str) -> Option<Vec<String>> {
    let (attribute, _) = line.trim_start().split_once('=')?;
    let attribute = attribute.trim();
    if attribute.is_empty() || attribute.starts_with('#') || attribute.contains(' ') {
        return None;
    }
    Some(
        attribute
            .split('.')
            .map(|component| component.trim_matches('"').to_owned())
            .collect(),
    )
}

/// The attribute this line declares, as it is written: the indentation, the path
/// and the `=`.
///
/// Taken verbatim rather than rebuilt, because the path is the consumer's — a
/// dotted `flake.safix.groups.oncall.members`, a bare `members` inside a block, a
/// quoted component — and a rewrite that normalised it would edit a line it was
/// only meant to extend.
fn declared_of(line: &str) -> String {
    line.split_once('=')
        .map_or_else(|| line.to_owned(), |(attribute, _)| attribute.to_owned())
        .trim_end()
        .to_owned()
}

/// The list's own line, opened for the one-per-line form.
fn opening_of(line: &str) -> String {
    format!("{} = [", declared_of(line))
}

/// The closing bracket, carrying whatever followed the one it replaces.
fn closing_of(line: &str, indent: &str) -> String {
    format!("{indent}]{}", tail_of(line))
}

/// Whatever a one-line list carries after its closing bracket: the semicolon, and
/// a trailing comment where the declaration has one.
fn tail_of(line: &str) -> String {
    line.rsplit_once(']')
        .map_or_else(String::new, |(_, tail)| tail.to_owned())
}

/// The names a one-line list holds, verbatim and in order.
fn inline_members(line: &str) -> Vec<&str> {
    let Some((_, rest)) = line.split_once('[') else {
        return Vec::new();
    };
    let Some((held, _)) = rest.rsplit_once(']') else {
        return Vec::new();
    };
    held.split_whitespace().collect()
}

/// Where a multi-line list closes.
fn closing_line(lines: &[&str], opening: usize) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .skip(opening.saturating_add(1))
        .find(|(_, line)| line.trim_start().starts_with(']'))
        .map(|(index, _)| index)
}

/// The line one name sits on inside a multi-line list.
fn member_line(lines: &[&str], opening: usize, closing: usize, quoted: &str) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .skip(opening.saturating_add(1))
        .take_while(|(index, _)| *index < closing)
        .find(|(_, line)| line.trim_start().starts_with(quoted))
        .map(|(index, _)| index)
}

/// Whether a multi-line list already holds one name.
fn holds_member(lines: &[&str], opening: usize, closing: usize, quoted: &str) -> bool {
    member_line(lines, opening, closing, quoted).is_some()
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
  flake.safix.users.alice = {
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
  flake.safix.users.alice = {
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
  flake.safix.users.alice = {
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
  flake.safix.users.alice = {
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

    /// The dotted one-line form, which is what a formatter leaves a short list as.
    const INLINE_GROUP: &str = "\
{
  # who is on call
  flake.safix.groups.oncall.members = [ \"alice\" ];
}
";

    /// The one-per-line form, which is what a list of two or more becomes.
    const MULTILINE_GROUP: &str = "\
{
  flake.safix.groups.oncall.members = [
    \"alice\"
    \"bob\"
  ];
}
";

    fn removed(removal: Removal) -> String {
        match removal {
            Removal::Removed(text) => text,
            other => unreachable!("expected a removal, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_membership_becomes_the_one_per_line_form_holding_the_subject() {
        let record = "{\n  flake.safix.groups.oncall.members = [ ];\n}\n";
        let edited = inserted(add_group_member(record, "oncall", "bob"));
        assert_eq!(
            edited,
            "{\n  flake.safix.groups.oncall.members = [\n    \"bob\"\n  ];\n}\n"
        );
    }

    #[test]
    fn a_one_line_membership_keeps_every_name_and_gains_one_line() {
        let edited = inserted(add_group_member(INLINE_GROUP, "oncall", "bob"));
        assert_eq!(
            edited,
            "{\n  # who is on call\n  flake.safix.groups.oncall.members = [\n    \"alice\"\n    \"bob\"\n  ];\n}\n"
        );
    }

    #[test]
    fn a_one_per_line_membership_gains_exactly_one_line() {
        let edited = inserted(add_group_member(MULTILINE_GROUP, "oncall", "carol"));
        assert_eq!(
            edited.lines().count(),
            MULTILINE_GROUP.lines().count().saturating_add(1),
            "the edit was not one inserted line: {edited}"
        );
        assert!(edited.contains("    \"carol\""));
        assert!(edited.contains("    \"alice\""), "a bystander was lost");
        assert!(edited.contains("    \"bob\""), "a bystander was lost");
    }

    #[test]
    fn a_subject_the_group_already_holds_writes_nothing() {
        assert_eq!(
            add_group_member(MULTILINE_GROUP, "oncall", "bob"),
            Edit::AlreadyPresent
        );
        assert_eq!(
            add_group_member(INLINE_GROUP, "oncall", "alice"),
            Edit::AlreadyPresent
        );
    }

    #[test]
    fn the_nested_form_is_edited_and_another_groups_membership_is_not() {
        let record = "\
{
  flake.safix.groups = {
    infra.members = [ \"deck\" ];
    oncall = {
      members = [
        \"alice\"
      ];
    };
  };
}
";
        let edited = inserted(add_group_member(record, "oncall", "bob"));
        assert!(
            edited.contains("        \"bob\""),
            "the edit missed: {edited}"
        );
        assert!(
            edited.contains("infra.members = [ \"deck\" ];"),
            "another group's membership was rewritten: {edited}"
        );
    }

    #[test]
    fn a_group_the_file_does_not_declare_has_nowhere_to_be_edited() {
        assert_eq!(
            add_group_member(MULTILINE_GROUP, "infra", "bob"),
            Edit::NoAnchor
        );
        assert_eq!(
            remove_group_member(MULTILINE_GROUP, "infra", "alice"),
            Removal::NoAnchor
        );
    }

    #[test]
    fn a_membership_this_editor_does_not_understand_is_refused_rather_than_compounded() {
        // The option takes this and this editor cannot read it, so extending it
        // would bury a hand edit under a generated one.
        let record = "{\n  flake.safix.groups.oncall.members = lib.mkAfter [ \"alice\" ];\n}\n";
        assert_eq!(add_group_member(record, "oncall", "bob"), Edit::NoAnchor);
        assert_eq!(
            remove_group_member(record, "oncall", "alice"),
            Removal::NoAnchor
        );
    }

    #[test]
    fn a_removal_takes_the_members_own_line_and_nothing_else() {
        let edited = removed(remove_group_member(MULTILINE_GROUP, "oncall", "alice"));
        assert_eq!(
            edited.lines().count(),
            MULTILINE_GROUP.lines().count().saturating_sub(1),
            "the removal was not one removed line: {edited}"
        );
        assert!(!edited.contains("\"alice\""));
        assert!(edited.contains("    \"bob\""), "a bystander was lost");
    }

    #[test]
    fn a_removal_from_a_one_line_membership_leaves_it_on_its_line() {
        let record = "{\n  flake.safix.groups.oncall.members = [ \"alice\" \"bob\" ]; # both\n}\n";
        let edited = removed(remove_group_member(record, "oncall", "alice"));
        assert_eq!(
            edited,
            "{\n  flake.safix.groups.oncall.members = [ \"bob\" ]; # both\n}\n"
        );
        let emptied = removed(remove_group_member(&edited, "oncall", "bob"));
        assert_eq!(
            emptied,
            "{\n  flake.safix.groups.oncall.members = [ ]; # both\n}\n"
        );
    }

    #[test]
    fn removing_a_subject_the_group_does_not_hold_writes_nothing() {
        assert_eq!(
            remove_group_member(MULTILINE_GROUP, "oncall", "carol"),
            Removal::NotPresent
        );
        assert_eq!(
            remove_group_member("{\n  oncall.members = [ ];\n}\n", "oncall", "carol"),
            Removal::NotPresent
        );
    }

    #[test]
    fn an_addition_and_a_removal_round_trip_to_the_same_membership() {
        let added = inserted(add_group_member(MULTILINE_GROUP, "oncall", "carol"));
        let back = removed(remove_group_member(&added, "oncall", "carol"));
        assert_eq!(back, MULTILINE_GROUP);
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
