//! The built-in catalogue: well-known web apps with a checked address and
//! the profile that signs in best.
//!
//! Sets decide what a group install adds (`add --set work`). Profiles decide
//! how an app signs in, and the two are independent: YouTube is in the media
//! set but signs in with the Google group profile, so one Google sign-in
//! covers Gmail, Gemini and YouTube.
//!
//! `formerly` lists addresses a service used before. `refresh` moves a webapp
//! that still points at one of them to the current address, so a renamed
//! service keeps working after `nirux update` without the user noticing.

/// How a catalogue app signs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// Its own browser profile.
    Isolated,
    /// The browser's ordinary profile (Microsoft 365 single sign-on).
    Shared,
    /// A profile shared by the apps of one group, apart from ordinary browsing.
    Group(&'static str),
}

#[derive(Debug)]
pub struct Entry {
    pub key: &'static str,
    pub name: &'static str,
    /// Empty when the address is the user's own (Nextcloud).
    pub url: &'static str,
    pub set: &'static str,
    pub profile: Profile,
    pub formerly: &'static [&'static str],
    /// What to ask for when `url` is empty.
    pub ask: &'static str,
}

/// Sets in display order, with a one-line description.
pub const SETS: &[(&str, &str)] = &[
    (
        "work",
        "Microsoft 365: Teams, Outlook, Outlook Calendar, Copilot, Loop, Planner, OneDrive",
    ),
    ("ai", "Claude, ChatGPT, Gemini, Grok, Gemini Notebook"),
    ("google", "Gmail, Google Calendar, Google Drive"),
    ("proton", "Proton Mail, Calendar, Drive and Pass"),
    ("media", "YouTube, YouTube Music, SVT Play"),
    ("other", "Spotify, Discord, GitHub, Excalidraw"),
    ("nextcloud", "Your own Nextcloud (asks for its address)"),
];

const M365: Profile = Profile::Shared;
const GOOGLE: Profile = Profile::Group("google");
const PROTON: Profile = Profile::Group("proton");
const OWN: Profile = Profile::Isolated;

const fn entry(
    key: &'static str,
    name: &'static str,
    url: &'static str,
    set: &'static str,
    profile: Profile,
    formerly: &'static [&'static str],
) -> Entry {
    Entry {
        key,
        name,
        url,
        set,
        profile,
        formerly,
        ask: "",
    }
}

/// Every address was checked on 2026-11-20 (see README, Catalogue).
pub const ENTRIES: &[Entry] = &[
    entry(
        "teams",
        "Teams",
        "https://teams.cloud.microsoft/",
        "work",
        M365,
        &["https://teams.microsoft.com/", "https://teams.live.com/"],
    ),
    entry(
        "outlook",
        "Outlook",
        "https://outlook.cloud.microsoft/mail/",
        "work",
        M365,
        &[
            "https://outlook.office.com/mail/",
            "https://outlook.office365.com/mail/",
        ],
    ),
    entry(
        "outlook-calendar",
        "Outlook Calendar",
        "https://outlook.cloud.microsoft/calendar/",
        "work",
        M365,
        &[
            "https://outlook.office.com/calendar/",
            "https://outlook.office365.com/calendar/",
        ],
    ),
    entry(
        "copilot",
        "Copilot",
        "https://m365.cloud.microsoft/",
        "work",
        M365,
        &[
            "https://www.microsoft365.com/",
            "https://www.office.com/",
            "https://office.com/",
        ],
    ),
    entry(
        "loop",
        "Loop",
        "https://loop.cloud.microsoft/",
        "work",
        M365,
        &["https://loop.microsoft.com/"],
    ),
    entry(
        "planner",
        "Planner",
        "https://planner.cloud.microsoft/",
        "work",
        M365,
        &["https://tasks.office.com/"],
    ),
    entry(
        "onedrive",
        "OneDrive",
        "https://onedrive.cloud.microsoft/",
        "work",
        M365,
        &[],
    ),
    entry("claude", "Claude", "https://claude.ai/", "ai", OWN, &[]),
    entry(
        "chatgpt",
        "ChatGPT",
        "https://chatgpt.com/",
        "ai",
        OWN,
        &["https://chat.openai.com/"],
    ),
    entry(
        "gemini",
        "Gemini",
        "https://gemini.google.com/app",
        "ai",
        GOOGLE,
        &["https://bard.google.com/"],
    ),
    entry("grok", "Grok", "https://grok.com/", "ai", OWN, &[]),
    entry(
        "gemini-notebook",
        "Gemini Notebook",
        "https://notebook.google.com/",
        "ai",
        GOOGLE,
        &["https://notebooklm.google.com/"],
    ),
    entry(
        "gmail",
        "Gmail",
        "https://mail.google.com/",
        "google",
        GOOGLE,
        &[],
    ),
    entry(
        "google-calendar",
        "Google Calendar",
        "https://calendar.google.com/",
        "google",
        GOOGLE,
        &[],
    ),
    entry(
        "google-drive",
        "Google Drive",
        "https://drive.google.com/",
        "google",
        GOOGLE,
        &[],
    ),
    entry(
        "proton-mail",
        "Proton Mail",
        "https://mail.proton.me/",
        "proton",
        PROTON,
        &[
            "https://mail.protonmail.com/",
            "https://account.protonmail.com/",
        ],
    ),
    entry(
        "proton-calendar",
        "Proton Calendar",
        "https://calendar.proton.me/",
        "proton",
        PROTON,
        &["https://calendar.protonmail.com/"],
    ),
    entry(
        "proton-drive",
        "Proton Drive",
        "https://drive.proton.me/",
        "proton",
        PROTON,
        &[],
    ),
    entry(
        "proton-pass",
        "Proton Pass",
        "https://pass.proton.me/",
        "proton",
        PROTON,
        &[],
    ),
    entry(
        "youtube",
        "YouTube",
        "https://www.youtube.com/",
        "media",
        GOOGLE,
        &["https://youtube.com/", "https://m.youtube.com/"],
    ),
    entry(
        "youtube-music",
        "YouTube Music",
        "https://music.youtube.com/",
        "media",
        GOOGLE,
        &[],
    ),
    entry(
        "svt-play",
        "SVT Play",
        "https://www.svtplay.se/",
        "media",
        OWN,
        &["https://svtplay.se/"],
    ),
    entry(
        "spotify",
        "Spotify",
        "https://open.spotify.com/",
        "other",
        OWN,
        &[],
    ),
    entry(
        "discord",
        "Discord",
        "https://discord.com/app",
        "other",
        OWN,
        &[],
    ),
    entry("github", "GitHub", "https://github.com/", "other", OWN, &[]),
    entry(
        "excalidraw",
        "Excalidraw",
        "https://excalidraw.com/",
        "other",
        OWN,
        &[],
    ),
    Entry {
        key: "nextcloud",
        name: "Nextcloud",
        url: "",
        set: "nextcloud",
        profile: OWN,
        formerly: &[],
        ask: "Nextcloud address (for example https://cloud.example.com)",
    },
];

pub fn get(key: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|entry| entry.key == key)
}

pub fn is_set(name: &str) -> bool {
    SETS.iter().any(|(set, _)| *set == name)
}

/// Keys of a set, in catalogue order.
pub fn set_entries(set: &str) -> impl Iterator<Item = &'static Entry> + '_ {
    ENTRIES.iter().filter(move |entry| entry.set == set)
}

/// Host and path with the trailing `/` dropped, for comparing addresses.
fn key_of(url: &str) -> Option<(String, String)> {
    let parts = crate::class::url_parts(url)?;
    let host = parts
        .host
        .strip_prefix("www.")
        .unwrap_or(&parts.host)
        .to_string();
    Some((host, parts.path.trim_end_matches('/').to_string()))
}

/// Whether an address is (or was) this entry's: the same host and path, or
/// the same host when the entry is the only catalogue app on that host and
/// one of the two is the site root (`gemini.google.com` and `/app`).
pub fn owns_url(entry: &Entry, url: &str) -> bool {
    let Some((host, path)) = key_of(url) else {
        return false;
    };
    let mine = std::iter::once(entry.url)
        .chain(entry.formerly.iter().copied())
        .filter_map(key_of);
    for (mine_host, mine_path) in mine {
        if mine_host != host {
            continue;
        }
        if mine_path == path {
            return true;
        }
        let alone = ENTRIES
            .iter()
            .filter(|other| other.key != entry.key)
            .filter_map(|other| key_of(other.url))
            .all(|(other_host, _)| other_host != host);
        if alone && (path.is_empty() || mine_path.is_empty()) {
            return true;
        }
    }
    false
}

/// The current address when `url` is exactly one of the entry's former ones.
pub fn moved_to(url: &str) -> Option<&'static str> {
    let wanted = key_of(url)?;
    ENTRIES
        .iter()
        .filter(|entry| !entry.url.is_empty())
        .find(|entry| {
            entry
                .formerly
                .iter()
                .any(|old| key_of(old).as_ref() == Some(&wanted))
        })
        .map(|entry| entry.url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique_slugs_and_sets_exist() {
        let mut seen = std::collections::HashSet::new();
        for entry in ENTRIES {
            assert!(seen.insert(entry.key), "duplicate key {}", entry.key);
            assert_eq!(crate::slug(entry.key), entry.key);
            assert!(
                is_set(entry.set),
                "{} has unknown set {}",
                entry.key,
                entry.set
            );
            assert!(
                entry.url.is_empty() != entry.ask.is_empty(),
                "{}",
                entry.key
            );
            if !entry.url.is_empty() {
                assert!(entry.url.starts_with("https://"), "{}", entry.key);
                assert!(crate::validate_url(entry.url).is_ok());
            }
            for old in entry.formerly {
                assert!(crate::validate_url(old).is_ok(), "{old}");
                assert_eq!(moved_to(old), Some(entry.url), "{old}");
            }
        }
        for (set, _) in SETS {
            assert!(set_entries(set).next().is_some(), "empty set {set}");
        }
    }

    #[test]
    fn existing_apps_are_recognised() {
        let gemini = get("gemini").unwrap();
        assert!(owns_url(gemini, "https://gemini.google.com"));
        assert!(owns_url(gemini, "https://gemini.google.com/app/"));
        assert!(owns_url(gemini, "https://bard.google.com/"));
        let outlook = get("outlook").unwrap();
        let calendar = get("outlook-calendar").unwrap();
        assert!(owns_url(outlook, "https://outlook.cloud.microsoft/mail"));
        assert!(!owns_url(
            outlook,
            "https://outlook.cloud.microsoft/calendar/"
        ));
        assert!(owns_url(
            calendar,
            "https://outlook.cloud.microsoft/calendar/"
        ));
        // Two catalogue apps share the host, so the root is neither.
        assert!(!owns_url(outlook, "https://outlook.cloud.microsoft/"));
        assert!(owns_url(
            get("copilot").unwrap(),
            "https://m365.cloud.microsoft/"
        ));
        assert!(owns_url(get("youtube").unwrap(), "https://youtube.com"));
        assert!(!owns_url(
            get("youtube").unwrap(),
            "https://music.youtube.com/"
        ));
        assert!(owns_url(get("claude").unwrap(), "https://claude.ai"));
        assert!(!owns_url(get("claude").unwrap(), "https://example.com/"));
    }

    #[test]
    fn former_addresses_move() {
        assert_eq!(
            moved_to("https://notebooklm.google.com/"),
            Some("https://notebook.google.com/")
        );
        assert_eq!(
            moved_to("https://outlook.office.com/mail"),
            Some("https://outlook.cloud.microsoft/mail/")
        );
        assert_eq!(moved_to("https://teams.cloud.microsoft/"), None);
        assert_eq!(moved_to("https://outlook.office.com/mail/inbox"), None);
    }
}
