//! Desktop Entry Specification escaping.
//!
//! String values escape `\` as `\\` and control characters as `\n`, `\t`,
//! `\r` (a leading space as `\s`, which parsers would otherwise trim).
//! `Exec` arguments with reserved characters are double-quoted, with `"`,
//! `` ` ``, `$` and `\` backslash-escaped inside the quotes; every `%` is
//! doubled; and the whole `Exec` value then gets the string escaping too,
//! as the spec requires (so a literal `$` in a quoted path is `\\$`).

/// Escape a `string`/`localestring` value.
pub fn escape_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut leading = true;
    for ch in value.chars() {
        match ch {
            ' ' if leading => out.push_str("\\s"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
        if ch != ' ' {
            leading = false;
        }
    }
    out
}

const RESERVED: &[char] = &[
    ' ', '\t', '\n', '"', '\'', '\\', '>', '<', '~', '|', '&', ';', '$', '*', '?', '#', '(', ')',
    '`',
];

/// One `Exec` argument, quoted when needed, with `%` doubled (before the
/// string escaping of the whole value).
pub fn exec_arg(arg: &str) -> String {
    let body = if arg.is_empty() || arg.contains(RESERVED) {
        let mut quoted = String::from("\"");
        for ch in arg.chars() {
            if matches!(ch, '"' | '`' | '$' | '\\') {
                quoted.push('\\');
            }
            quoted.push(ch);
        }
        quoted.push('"');
        quoted
    } else {
        arg.to_string()
    };
    body.replace('%', "%%")
}

/// A complete `Exec` value for a command line.
pub fn exec_value(argv: &[&str]) -> String {
    let line: Vec<String> = argv.iter().map(|arg| exec_arg(arg)).collect();
    escape_string(&line.join(" "))
}

pub struct Entry<'a> {
    pub name: &'a str,
    pub comment: &'a str,
    pub exec: &'a [&'a str],
    pub icon: &'a str,
    pub wm_class: &'a str,
}

impl Entry<'_> {
    pub fn text(&self) -> String {
        format!(
            "[Desktop Entry]\nType=Application\nName={}\nComment={}\nExec={}\nIcon={}\nTerminal=false\nCategories=Network;WebBrowser;\nStartupWMClass={}\n",
            escape_string(self.name),
            escape_string(self.comment),
            exec_value(self.exec),
            escape_string(self.icon),
            escape_string(self.wm_class),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference parser: string unescaping, then Exec quoting and `%%`.
    fn unescape_string(value: &str) -> String {
        let mut out = String::new();
        let mut chars = value.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('s') => out.push(' '),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('\\') => out.push('\\'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                }
            } else {
                out.push(ch);
            }
        }
        out
    }

    fn parse_exec(value: &str) -> Vec<String> {
        let line = unescape_string(value);
        let mut args = Vec::new();
        let mut arg = String::new();
        let mut started = false;
        let mut quoted = false;
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '"' => {
                    quoted = !quoted;
                    started = true;
                }
                '\\' if quoted => arg.push(chars.next().unwrap()),
                '%' => {
                    assert_eq!(chars.next(), Some('%'), "lone % in {value}");
                    arg.push('%');
                    started = true;
                }
                ' ' if !quoted => {
                    if started {
                        args.push(std::mem::take(&mut arg));
                        started = false;
                    }
                }
                c => {
                    assert!(
                        quoted || !RESERVED.contains(&c),
                        "unquoted {c:?} in {value}"
                    );
                    arg.push(c);
                    started = true;
                }
            }
        }
        assert!(!quoted, "unterminated quote in {value}");
        if started {
            args.push(arg);
        }
        args
    }

    #[test]
    fn exec_round_trips_reserved_characters() {
        let paths = [
            "/home/niru/.local/bin/niruwebapp",
            "/opt/my apps/niruwebapp",
            "/tmp/100%/niruwebapp",
            "/tmp/$HOME/niru\"web`app",
            "/tmp/back\\slash/ä ö/niruwebapp",
            "/tmp/'single'/niruwebapp",
        ];
        for path in paths {
            let argv = [path, "launch", "teams"];
            let value = exec_value(&argv);
            assert_eq!(parse_exec(&value), argv, "{value}");
        }
        assert_eq!(
            exec_value(&["/usr/bin/x", "launch", "a"]),
            "/usr/bin/x launch a"
        );
        assert_eq!(exec_value(&["/a b/x"]), "\"/a b/x\"");
        assert_eq!(exec_value(&["/a%b/x"]), "/a%%b/x");
        assert_eq!(exec_value(&["/a$b/x"]), "\"/a\\\\$b/x\"");
    }

    #[test]
    fn strings_round_trip() {
        let names = [
            "Teams",
            "100% Fun",
            "Price $5",
            "\"Quoted\" `name`",
            "Back\\slash",
            " leading space",
            "Smörgåsbord 日本",
        ];
        for name in names {
            let escaped = escape_string(name);
            assert!(!escaped.contains('\n'));
            assert!(!escaped.starts_with(' '));
            assert_eq!(unescape_string(&escaped), name, "{escaped}");
        }
        assert_eq!(escape_string("a\nb"), "a\\nb");
    }

    #[test]
    fn entry_has_one_line_per_key() {
        let text = Entry {
            name: "Evil\nExec=rm -rf ~",
            comment: "Webapp",
            exec: &["/bin/niruwebapp", "launch", "evil"],
            icon: "/x/icon.png",
            wm_class: "niruwebapp-evil",
        }
        .text();
        assert_eq!(text.lines().filter(|l| l.starts_with("Exec=")).count(), 1);
        assert!(text.contains("Name=Evil\\nExec=rm -rf ~\n"));
    }
}
