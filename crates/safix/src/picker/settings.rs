//! What the picker remembers between runs.
//!
//! Three things, and each one is a state the operator put the picker in rather
//! than a preference they configured: whether the preview is showing, whether
//! the extra columns are, and which entry they chose last. A picker that opened
//! with the preview on after being closed with it off would be asking for the
//! same keystroke every run.
//!
//! # Why this is not a configuration file
//!
//! Nothing here is declared anywhere, nothing reads it but the picker, and
//! every field is written by a keystroke. So a corrupt file is overwritten
//! rather than refused: the alternative is a command that will not open a list
//! because of a file the operator never edited, and there is nothing in it
//! worth a refusal — the worst outcome of ignoring it is one keystroke.
//!
//! It holds the name of the last entry chosen, which is a name and not a value.
//! That is still something a person may not want on disk, and it is why the
//! file is created `0600` and why it lives in the state directory rather than
//! beside the repository, where it would be a candidate for a commit.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::PathBuf;

use serde_json::{Map, Value};

/// The mode the file is created with, and the mode it is put back to.
const PRIVATE: u32 = 0o600;

/// The mode the directory above it is created with.
const PRIVATE_DIRECTORY: u32 = 0o700;

/// What the picker remembers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Settings {
    /// Whether the value pane is drawn and the highlighted value decrypted.
    pub preview: bool,
    /// Whether the columns behind tab are shown.
    pub extra_columns: bool,
    /// The entry each user chose last, by user.
    ///
    /// Per user rather than one name, because one person's workstation resolves
    /// several declared users and the name they chose as one of them says
    /// nothing about what they would choose as another.
    pub last_chosen: BTreeMap<String, String>,
}

impl Default for Settings {
    /// The preview on, the extra columns off, nothing chosen yet.
    ///
    /// Also what a missing or unreadable file means, which is why the defaults
    /// are here rather than spelled out at each reader.
    fn default() -> Self {
        Self {
            preview: true,
            extra_columns: false,
            last_chosen: BTreeMap::new(),
        }
    }
}

impl Settings {
    /// The name this user chose last, when the file remembers one.
    pub(crate) fn chosen_by(&self, user: &str) -> Option<&str> {
        self.last_chosen.get(user).map(String::as_str)
    }

    /// Record that this user has just chosen this name.
    pub(crate) fn choose(&mut self, user: &str, name: &str) {
        let _ = self.last_chosen.insert(user.to_owned(), name.to_owned());
    }
}

/// Where the file lives, or nothing when no state directory can be named.
///
/// `XDG_STATE_HOME` when it is set to an absolute path, and
/// `$HOME/.local/state` otherwise, which is what the basedir specification
/// names as that variable's own default. Neither set is no file: the picker
/// then works exactly as it does on the first run, every run.
fn path() -> Option<PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|state| state.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|home| home.is_absolute())
                .map(|home| home.join(".local").join("state"))
        })?;
    Some(state.join("safix").join("picker.json"))
}

/// Whatever the file remembers, or the defaults.
pub(crate) fn load() -> Settings {
    path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map_or_else(Settings::default, |text| decode(&text))
}

/// Write these settings, best effort.
///
/// Nothing is returned and nothing is reported: this is called on every toggle
/// and on the way out of a choice, and a state directory that cannot be written
/// is not a reason to refuse a selection the operator has already made.
pub(crate) fn store(settings: &Settings) {
    let Some(path) = path() else {
        return;
    };
    if let Some(directory) = path.parent() {
        let _ = std::fs::DirBuilder::new()
            .recursive(true)
            .mode(PRIVATE_DIRECTORY)
            .create(directory);
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(PRIVATE)
        .open(&path)
    else {
        return;
    };
    // The mode above applies to a file this call creates. A file that already
    // existed keeps whatever mode it had, so it is set explicitly as well —
    // this file names an entry, and a run that tightened it once should not
    // depend on which run created it.
    let _ = file.set_permissions(std::fs::Permissions::from_mode(PRIVATE));
    let _ = file.write_all(encode(settings).as_bytes());
    let _ = file.flush();
}

/// The settings one file's text holds, taking the default for anything it does
/// not.
///
/// Per field rather than all-or-nothing: a file written by a newer version that
/// carries a field this does not know is still a file whose `preview` this can
/// read.
fn decode(text: &str) -> Settings {
    let mut settings = Settings::default();
    let Ok(Value::Object(fields)) = serde_json::from_str::<Value>(text) else {
        return settings;
    };
    if let Some(preview) = fields.get("preview").and_then(Value::as_bool) {
        settings.preview = preview;
    }
    if let Some(extra) = fields.get("extraColumns").and_then(Value::as_bool) {
        settings.extra_columns = extra;
    }
    if let Some(Value::Object(chosen)) = fields.get("lastChosen") {
        for (user, name) in chosen {
            if let Some(name) = name.as_str() {
                let _ = settings.last_chosen.insert(user.clone(), name.to_owned());
            }
        }
    }
    settings
}

/// The file's text for these settings, with a trailing newline.
///
/// The three fields are written in the order this module documents them in
/// rather than in the order a map would sort them, because the file is meant to
/// be readable by whoever finds it; the names inside `lastChosen` are escaped
/// and ordered by the JSON writer, since a declared user name is not this
/// module's to assume anything about.
fn encode(settings: &Settings) -> String {
    let mut chosen = Map::new();
    for (user, name) in &settings.last_chosen {
        let _ = chosen.insert(user.clone(), Value::String(name.clone()));
    }
    format!(
        "{{\"preview\":{},\"extraColumns\":{},\"lastChosen\":{}}}\n",
        settings.preview,
        settings.extra_columns,
        Value::Object(chosen)
    )
}

#[cfg(test)]
mod tests {
    use super::{Settings, decode, encode};

    #[test]
    fn the_defaults_are_the_preview_on_and_the_extra_columns_off() {
        let settings = Settings::default();
        assert!(settings.preview);
        assert!(!settings.extra_columns);
        assert_eq!(settings.chosen_by("alice"), None);
    }

    #[test]
    fn a_file_reads_back_as_what_was_written() {
        let mut written = Settings {
            preview: false,
            extra_columns: true,
            last_chosen: std::collections::BTreeMap::new(),
        };
        written.choose("alice", "api-token");
        written.choose("bob", "fleet-token");
        assert_eq!(decode(&encode(&written)), written);
    }

    /// The field names are the ones the file's own documentation states, which
    /// is what makes a hand-written file readable and a written one inspectable.
    #[test]
    fn the_written_fields_are_the_documented_ones() {
        let mut settings = Settings::default();
        settings.choose("alice", "api-token");
        assert_eq!(
            encode(&settings),
            "{\"preview\":true,\"extraColumns\":false,\
             \"lastChosen\":{\"alice\":\"api-token\"}}\n"
        );
    }

    /// A corrupt file is the defaults, not a refusal and not a partial read of
    /// whatever parsed.
    #[test]
    fn anything_that_is_not_this_file_reads_as_the_defaults() {
        for text in [
            "",
            "{",
            "null",
            "[]",
            "not json at all",
            "{\"preview\": \"yes\"}",
            "{\"lastChosen\": \"api-token\"}",
        ] {
            assert_eq!(
                decode(text),
                Settings::default(),
                "{text:?} was not read as the defaults"
            );
        }
    }

    /// A field this version does not know does not cost the ones it does.
    #[test]
    fn an_unknown_field_leaves_the_known_ones_readable() {
        let settings = decode("{\"preview\":false,\"sortOrder\":\"created\"}");
        assert!(!settings.preview);
        assert!(!settings.extra_columns);
    }
}
