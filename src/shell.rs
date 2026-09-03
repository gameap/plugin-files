//! Safe command-string construction for `nodecmd`/daemon tasks.
//!
//! The daemon splits the command with `shellquote.Split` and execs the argv
//! directly (no shell), so injection is via extra argv tokens, not shell
//! metacharacters. [`shell_join`] quotes each token so a value containing
//! spaces or specials survives `Split` as exactly one argument. The Go plugin
//! used `%q` (double quotes) for install-script arguments; single-quoting
//! yields the identical argv after `Split` and handles non-ASCII correctly.
//!
//! Windows nodes need [`shell_join_windows`]: before splitting, the daemon
//! doubles every backslash (its `pkg/shellquote`), so a bare or double-quoted
//! `C:\gameap` comes back unchanged while a single-quoted one keeps the doubled
//! backslashes. Double quotes are therefore the only quoting that round-trips
//! a Windows path, and a backslash counts as a plain character there.

/// Joins arguments into a single command string, single-quoting any token that
/// is empty or contains characters outside a conservative safe set. Compatible
/// with `github.com/kballard/go-shellquote`'s `Split`.
pub fn shell_join(args: &[&str]) -> String {
    args.iter().map(|a| quote(a)).collect::<Vec<_>>().join(" ")
}

/// [`shell_join`] for commands a Windows daemon splits (see the module docs).
pub fn shell_join_windows(args: &[&str]) -> String {
    args.iter()
        .map(|a| quote_windows(a))
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_safe_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'_' | b'@' | b'%' | b'+' | b'=' | b':' | b',' | b'.' | b'/' | b'-'
        )
}

fn is_safe(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(is_safe_byte)
}

fn is_safe_windows(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| is_safe_byte(b) || b == b'\\')
}

fn quote(s: &str) -> String {
    if is_safe(s) {
        return s.to_string();
    }
    // Single-quote wrap; close-quote, escaped literal quote, reopen for each '.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn quote_windows(s: &str) -> String {
    if is_safe_windows(s) {
        return s.to_string();
    }
    // Double-quote wrap. A backslash escape cannot express an embedded double
    // quote here (the daemon would double the backslash first), so the quoted
    // run is closed, the quote spliced in as a single-quoted token, and the
    // run reopened — adjacent quoted runs form one word.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        if ch == '"' {
            out.push_str("\"'\"'\"");
        } else {
            out.push(ch);
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_tokens_pass_through() {
        assert_eq!(
            shell_join(&["get-tool", "https://example.com/install.sh"]),
            "get-tool https://example.com/install.sh"
        );
        assert_eq!(
            shell_join(&["--data-dir=/srv/gameap", "--ftp-listen-address=:21"]),
            "--data-dir=/srv/gameap --ftp-listen-address=:21"
        );
    }

    #[test]
    fn spaces_are_quoted() {
        assert_eq!(
            shell_join(&["--data-dir=/srv/game ap"]),
            "'--data-dir=/srv/game ap'"
        );
    }

    #[test]
    fn embedded_quote_is_escaped() {
        assert_eq!(quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn empty_token_is_quoted() {
        assert_eq!(quote(""), "''");
    }

    #[test]
    fn windows_bare_backslash_path_passes_through() {
        assert_eq!(
            shell_join_windows(&[r"C:\gameap\tools\gameap-files\gameap-files.exe", "version"]),
            r"C:\gameap\tools\gameap-files\gameap-files.exe version"
        );
    }

    #[test]
    fn windows_spaces_use_double_quotes() {
        assert_eq!(
            shell_join_windows(&["-DataDir", r"C:\Program Files\gameap"]),
            r#"-DataDir "C:\Program Files\gameap""#
        );
    }

    #[test]
    fn windows_placeholder_token_is_double_quoted() {
        assert_eq!(quote_windows("{node_work_path}"), r#""{node_work_path}""#);
        assert_eq!(
            quote_windows("{node_tools_path}/install-files-windows.ps1"),
            r#""{node_tools_path}/install-files-windows.ps1""#
        );
    }

    #[test]
    fn windows_embedded_double_quote() {
        assert_eq!(quote_windows(r#"a"b"#), r#""a"'"'"b""#);
    }

    #[test]
    fn windows_empty_token() {
        assert_eq!(quote_windows(""), r#""""#);
    }
}
