//! Safe command-string construction for `nodecmd`/daemon tasks.
//!
//! The daemon splits the command with `shellquote.Split` and execs the argv
//! directly (no shell), so injection is via extra argv tokens, not shell
//! metacharacters. [`shell_join`] quotes each token so a value containing
//! spaces or specials survives `Split` as exactly one argument. The Go plugin
//! used `%q` (double quotes) for install-script arguments; single-quoting
//! yields the identical argv after `Split` and handles non-ASCII correctly.

/// Joins arguments into a single command string, single-quoting any token that
/// is empty or contains characters outside a conservative safe set. Compatible
/// with `github.com/kballard/go-shellquote`'s `Split`.
pub fn shell_join(args: &[&str]) -> String {
    args.iter().map(|a| quote(a)).collect::<Vec<_>>().join(" ")
}

fn is_safe(s: &str) -> bool {
    !s.is_empty()
        && s.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b'_' | b'@' | b'%' | b'+' | b'=' | b':' | b',' | b'.' | b'/' | b'-'
                )
        })
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
}
