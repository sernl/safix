//! The options vim and nvim are opened with, so the staged buffer leaves no
//! state of their own behind.
//!
//! These are programs *recognised*, not programs fallen back to. Which editor
//! runs is still the operator's choice alone, made in [`super::Editor::choose`];
//! this module only answers, for the program already chosen, whether it is one
//! whose defaults write buffer-derived state outside the staging root. vim and
//! nvim are: nvim keeps its swap, undo and backup files and its shada history
//! under its own state directory, and vim with `defaults.vim` writes registers
//! to `~/.viminfo`, so a line yanked while editing would outlive the run.
//!
//! The program is recognised by the last component of its path and nothing
//! else, so a bare `nvim` and a store path ending in `/bin/nvim` are the same
//! program, and a value that merely mentions vim somewhere is not. `vi` is not
//! recognised: on many hosts it is nvi or busybox vi, neither of which accepts
//! `--cmd`, and passing the options there would make the edit fail rather than
//! harden it. `view` and `rvim` are read-only and restricted invocations, and
//! `gvim` forks into its own window unless given `-f`, so none of them is in
//! the shape the rest of `edit` assumes.
//!
//! The two option strings are agenix-rekey's, verbatim from
//! `apps/edit-view.nix:154` and `:156` at `87e5d5f`, so the two projects can be
//! compared line for line. Upstream passes `--cmd` alone; `-n` is added before
//! it because nvim creates its swap file in its own state directory before the
//! autocommand can switch it off, and that file records the user, the host and
//! the staged path. `-n` applies while the editor starts, before any buffer is
//! read, so no swap file exists at any moment. `noswapfile` stays in the
//! autocommand so the strings remain upstream's.
//!
//! There is no opt-out. The options reach only the buffer safix staged, which
//! nobody wants recoverable from a swap, undo or history file; an operator who
//! wants other behaviour names a program this does not recognise.

use std::path::Path;

/// The autocommand nvim is given: every buffer read loses its backups,
/// modelines, shell temporary files, swap file, undo file and write backup, and
/// the session writes no shada file.
const NVIM: &str = "au BufRead * setlocal nobackup nomodeline noshelltemp noswapfile noundofile nowritebackup shadafile=NONE";

/// The autocommand vim is given: the same settings, with an empty `viminfo` in
/// place of nvim's `shadafile=NONE`.
const VIM: &str = r#"au BufRead * setlocal nobackup nomodeline noshelltemp noswapfile noundofile nowritebackup viminfo="""#;

/// The arguments inserted before the staged path for `program`, or none when
/// the program is not one this recognises.
pub(super) fn options(program: &str) -> Option<[&'static str; 3]> {
    let command = match Path::new(program).file_name()?.to_str()? {
        "nvim" => NVIM,
        "vim" => VIM,
        _ => return None,
    };
    Some(["-n", "--cmd", command])
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;

    use super::super::Editor;
    use super::{NVIM, VIM};

    const STAGED: &str = "/run/user/1000/safix.staging/api-token";

    /// The vector the editor is run with, for an `EDITOR` holding `value`.
    fn argv(value: &str) -> (String, Vec<OsString>) {
        let Ok(editor) = Editor::choose(None, Some(value.to_owned())) else {
            unreachable!("a set variable names an editor");
        };
        let vector = editor.argv(Path::new(STAGED));
        (editor.program, vector)
    }

    fn expected(words: &[&str]) -> Vec<OsString> {
        words.iter().map(OsString::from).collect()
    }

    #[test]
    fn nvim_receives_the_nvim_options_before_the_path() {
        assert_eq!(
            argv("nvim"),
            ("nvim".to_owned(), expected(&["-n", "--cmd", NVIM, STAGED]))
        );
    }

    #[test]
    fn vim_receives_the_vim_options_before_the_path() {
        assert_eq!(
            argv("vim"),
            ("vim".to_owned(), expected(&["-n", "--cmd", VIM, STAGED]))
        );
    }

    #[test]
    fn a_path_naming_nvim_is_recognised() {
        let program = "/nix/store/0c3kbl7b8vjdbq3n5wq1f7yv3x4k2c1a-neovim/bin/nvim";
        assert_eq!(
            argv(program),
            (program.to_owned(), expected(&["-n", "--cmd", NVIM, STAGED]))
        );
    }

    #[test]
    fn the_operators_arguments_come_first() {
        assert_eq!(
            argv("nvim -u NONE"),
            (
                "nvim".to_owned(),
                expected(&["-u", "NONE", "-n", "--cmd", NVIM, STAGED])
            )
        );
    }

    /// A program this does not recognise: its own arguments and the path, and
    /// nothing else.
    fn unchanged(value: &str, program: &str, words: &[&str]) {
        assert_eq!(
            argv(value),
            (program.to_owned(), expected(words)),
            "{value} gained arguments it was not given"
        );
    }

    #[test]
    fn an_editor_with_arguments_is_opened_unchanged() {
        unchanged("code --wait", "code", &["--wait", STAGED]);
    }

    #[test]
    fn bare_vi_is_opened_unchanged() {
        unchanged("vi", "vi", &[STAGED]);
    }

    #[test]
    fn gvim_is_opened_unchanged() {
        unchanged("gvim", "gvim", &[STAGED]);
    }

    #[test]
    fn a_path_ending_in_vimdiff_is_opened_unchanged() {
        unchanged("/usr/bin/vimdiff", "/usr/bin/vimdiff", &[STAGED]);
    }
}
