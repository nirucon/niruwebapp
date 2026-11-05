//! Window class derivation for Chromium-family `--app` windows.
//!
//! Chromium names an app window after its launch URL: the canonical host,
//! `_`, and the canonical path (`GenerateApplicationNameFromURL`). On X11 that
//! name becomes the WM_CLASS *instance* after file-name sanitizing
//! (`GetWMClassFromAppName`): characters that are illegal in file names
//! (`"~*/:<>?\|` and control/format characters, plus whitespace, `.` and `~`
//! at either end) become `_`, then leading and trailing `_` are trimmed.
//!
//! `--class` only sets the WM_CLASS *class* and only when it starts a new
//! browser process. An isolated webapp always owns its process, so its class
//! is `niruwebapp-<id>`. A shared-profile webapp usually joins the running
//! browser, so the instance name is the only stable identity.

/// The parts of an http(s) URL that Chromium uses for the app name.
#[derive(Debug, PartialEq, Eq)]
pub struct UrlParts {
    pub host: String,
    pub path: String,
}

/// Split and canonicalize an http(s) URL the way Chromium's GURL does for
/// host and path: lowercase ASCII host, IDN labels as punycode, no port, no
/// userinfo, `/` for an empty path, dot segments resolved, and non-ASCII or
/// unsafe path bytes percent-encoded.
pub fn url_parts(url: &str) -> Option<UrlParts> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let host = if authority.starts_with('[') {
        // IPv6 literal: keep the brackets, drop the port.
        let close = authority.find(']')?;
        authority[..=close].to_ascii_lowercase()
    } else {
        let host = authority.split(':').next().unwrap_or("");
        idn_host(host)?
    };
    if host.is_empty() {
        return None;
    }
    let path_end = tail.find(['?', '#']).unwrap_or(tail.len());
    let path = canonical_path(&tail[..path_end]);
    Some(UrlParts { host, path })
}

/// The URL with an IDN host as punycode and every non-ASCII byte
/// percent-encoded. Chromium does not open an app window for a raw Unicode
/// `--app=` argument, so the browser always gets this form.
pub fn ascii_url(url: &str) -> String {
    if url.is_ascii() {
        return url.to_string();
    }
    let (scheme, rest) = match url.find("://") {
        Some(index) => url.split_at(index + 3),
        None => return url.to_string(),
    };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    let (userinfo, hostport) = match authority.rfind('@') {
        Some(index) => authority.split_at(index + 1),
        None => ("", authority),
    };
    let (host, port) = match hostport.rfind(':') {
        Some(index) if !hostport.starts_with('[') => hostport.split_at(index),
        _ => (hostport, ""),
    };
    let host = idn_host(host).unwrap_or_else(|| host.to_string());
    let mut out = format!("{scheme}{}{host}{port}", percent_non_ascii(userinfo));
    out.push_str(&percent_non_ascii(tail));
    out
}

fn percent_non_ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_ascii() {
            out.push(ch);
        } else {
            let mut buffer = [0u8; 4];
            for byte in ch.encode_utf8(&mut buffer).bytes() {
                out.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    out
}

fn idn_host(host: &str) -> Option<String> {
    let host = host.trim_end_matches('.');
    let mut labels = Vec::new();
    for label in host.split('.') {
        let lower: String = label.chars().flat_map(char::to_lowercase).collect();
        if lower.is_ascii() {
            labels.push(lower);
        } else {
            labels.push(format!("xn--{}", punycode(&lower)?));
        }
    }
    Some(labels.join("."))
}

fn canonical_path(raw: &str) -> String {
    let raw = if raw.is_empty() { "/" } else { raw };
    // Resolve "." and ".." segments as GURL does.
    let mut segments: Vec<&str> = Vec::new();
    let parts: Vec<&str> = raw.split('/').skip(1).collect();
    let last = parts.len().saturating_sub(1);
    for (index, segment) in parts.iter().enumerate() {
        match *segment {
            "." | "%2e" | "%2E" => {
                if index == last {
                    segments.push("");
                }
            }
            ".." | "%2e%2e" | "%2E%2E" | ".%2e" | "%2e." => {
                segments.pop();
                if index == last {
                    segments.push("");
                }
            }
            other => segments.push(other),
        }
    }
    let mut out = String::from("/");
    out.push_str(&segments.join("/"));
    let mut encoded = String::with_capacity(out.len());
    for byte in out.bytes() {
        let keep =
            byte.is_ascii_graphic() && !matches!(byte, b'"' | b'<' | b'>' | b'`' | b'{' | b'}');
        if keep {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// RFC 3492 punycode encoding of one label (without the `xn--` prefix).
fn punycode(input: &str) -> Option<String> {
    const BASE: u32 = 36;
    const TMIN: u32 = 1;
    const TMAX: u32 = 26;
    const SKEW: u32 = 38;
    const DAMP: u32 = 700;
    fn adapt(mut delta: u32, points: u32, first: bool) -> u32 {
        delta /= if first { DAMP } else { 2 };
        delta += delta / points;
        let mut k = 0;
        while delta > ((BASE - TMIN) * TMAX) / 2 {
            delta /= BASE - TMIN;
            k += BASE;
        }
        k + (BASE - TMIN + 1) * delta / (delta + SKEW)
    }
    fn digit(d: u32) -> char {
        (if d < 26 {
            b'a' + d as u8
        } else {
            b'0' + (d - 26) as u8
        }) as char
    }
    let chars: Vec<u32> = input.chars().map(u32::from).collect();
    let mut output: String = input.chars().filter(char::is_ascii).collect();
    let basic = output.len() as u32;
    let mut handled = basic;
    if basic > 0 {
        output.push('-');
    }
    let (mut n, mut delta, mut bias) = (128u32, 0u32, 72u32);
    while (handled as usize) < chars.len() {
        let m = *chars.iter().filter(|&&c| c >= n).min()?;
        delta = delta.checked_add((m - n).checked_mul(handled + 1)?)?;
        n = m;
        for &c in &chars {
            if c < n {
                delta = delta.checked_add(1)?;
            }
            if c == n {
                let mut q = delta;
                let mut k = BASE;
                loop {
                    let t = if k <= bias {
                        TMIN
                    } else if k >= bias + TMAX {
                        TMAX
                    } else {
                        k - bias
                    };
                    if q < t {
                        break;
                    }
                    output.push(digit(t + (q - t) % (BASE - t)));
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                output.push(digit(q));
                bias = adapt(delta, handled + 1, handled == basic);
                delta = 0;
                handled += 1;
            }
        }
        delta += 1;
        n += 1;
    }
    Some(output)
}

fn illegal_anywhere(ch: char) -> bool {
    matches!(
        ch,
        '"' | '~' | '*' | '/' | ':' | '<' | '>' | '?' | '\\' | '|'
    ) || ch.is_control()
        || matches!(ch, '\u{ad}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2064}' | '\u{feff}')
}

fn illegal_at_ends(ch: char) -> bool {
    ch.is_whitespace() || ch == '.' || ch == '~'
}

/// The WM_CLASS instance name Chromium gives an `--app=URL` window.
pub fn app_instance(url: &str) -> Option<String> {
    let parts = url_parts(url)?;
    let name = format!("{}_{}", parts.host, parts.path);
    let count = name.chars().count();
    let sanitized: String = name
        .chars()
        .enumerate()
        .map(|(index, ch)| {
            let at_end = index == 0 || index + 1 == count;
            if illegal_anywhere(ch) || (at_end && illegal_at_ends(ch)) {
                '_'
            } else {
                ch
            }
        })
        .collect();
    Some(sanitized.trim_matches('_').to_string())
}

/// The WM_CLASS class an isolated webapp gets from `--class`.
pub fn isolated_class(id: &str) -> String {
    format!("niruwebapp-{id}")
}

/// What a desktop entry should declare as StartupWMClass.
pub fn expected_class(id: &str, url: &str, shared_profile: bool) -> String {
    if shared_profile {
        app_instance(url).unwrap_or_else(|| isolated_class(id))
    } else {
        isolated_class(id)
    }
}

/// Whether a window's WM_CLASS (instance, class) belongs to this webapp.
pub fn window_matches(
    id: &str,
    url: &str,
    shared_profile: bool,
    instance: &str,
    class: &str,
) -> bool {
    let wanted_instance = app_instance(url);
    if shared_profile {
        wanted_instance.as_deref() == Some(instance)
    } else {
        class == isolated_class(id)
            && wanted_instance
                .as_deref()
                .is_none_or(|want| want == instance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_instance_matches_chromium() {
        let table = [
            ("https://chatgpt.com", "chatgpt.com"),
            ("https://chatgpt.com/", "chatgpt.com"),
            ("https://claude.ai/new", "claude.ai__new"),
            ("https://gemini.google.com/app", "gemini.google.com__app"),
            ("https://teams.cloud.microsoft/", "teams.cloud.microsoft"),
            (
                "https://outlook.cloud.microsoft/mail/",
                "outlook.cloud.microsoft__mail",
            ),
            (
                "https://outlook.cloud.microsoft/mail",
                "outlook.cloud.microsoft__mail",
            ),
            (
                "https://music.youtube.com/?feature=x#top",
                "music.youtube.com",
            ),
            ("https://Example.COM:8443/A/b", "example.com__A_b"),
            ("http://user:pw@example.com:80/x", "example.com__x"),
            ("https://example.com/a/./b/../c", "example.com__a_c"),
            ("https://example.com/foo.", "example.com__foo"),
            ("https://example.com/~user", "example.com___user"),
            ("https://example.com/a:b", "example.com__a_b"),
            ("https://bücher.de/", "xn--bcher-kva.de"),
            (
                "https://münchen.de/straße",
                "xn--mnchen-3ya.de__stra%C3%9Fe",
            ),
            ("https://例え.テスト/", "xn--r8jz45g.xn--zckzah"),
            ("https://[::1]:8080/x", "[__1]__x"),
            ("https://example.com/a%20b", "example.com__a%20b"),
        ];
        for (url, want) in table {
            assert_eq!(app_instance(url).as_deref(), Some(want), "{url}");
        }
    }

    #[test]
    fn expected_class_depends_on_profile_mode() {
        assert_eq!(
            expected_class("teams", "https://teams.cloud.microsoft/", true),
            "teams.cloud.microsoft"
        );
        assert_eq!(
            expected_class("claude", "https://claude.ai", false),
            "niruwebapp-claude"
        );
    }

    #[test]
    fn windows_are_matched_by_mode() {
        let url = "https://teams.cloud.microsoft/";
        assert!(window_matches(
            "teams",
            url,
            true,
            "teams.cloud.microsoft",
            "Helium"
        ));
        assert!(window_matches(
            "teams",
            url,
            true,
            "teams.cloud.microsoft",
            "niruwebapp-teams"
        ));
        assert!(!window_matches(
            "teams",
            url,
            true,
            "outlook.cloud.microsoft__mail",
            "Helium"
        ));
        assert!(window_matches(
            "claude",
            "https://claude.ai",
            false,
            "claude.ai",
            "niruwebapp-claude"
        ));
        assert!(!window_matches(
            "claude",
            "https://claude.ai",
            false,
            "claude.ai",
            "Helium"
        ));
        assert!(!window_matches(
            "claude",
            "https://claude.ai",
            false,
            "Navigator",
            "niruwebapp-claude"
        ));
    }

    #[test]
    fn browser_gets_an_ascii_url() {
        assert_eq!(ascii_url("https://example.com/a"), "https://example.com/a");
        assert_eq!(ascii_url("https://bücher.de/"), "https://xn--bcher-kva.de/");
        assert_eq!(
            ascii_url("https://München.de:8080/straße?q=ö"),
            "https://xn--mnchen-3ya.de:8080/stra%C3%9Fe?q=%C3%B6"
        );
        assert_eq!(
            app_instance(&ascii_url("https://münchen.de/straße")),
            app_instance("https://münchen.de/straße")
        );
    }

    #[test]
    fn punycode_reference_vectors() {
        assert_eq!(punycode("bücher").as_deref(), Some("bcher-kva"));
        assert_eq!(punycode("münchen").as_deref(), Some("mnchen-3ya"));
        assert_eq!(punycode("例え").as_deref(), Some("r8jz45g"));
        assert_eq!(punycode("テスト").as_deref(), Some("zckzah"));
    }
}
