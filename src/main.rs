use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Clone)]
struct App {
    id: String,
    name: String,
    url: String,
    browser: String,
    shared_profile: bool,
    icon: Option<String>,
}

fn home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn data_home() -> PathBuf {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local/share"))
}

fn state_dir() -> PathBuf {
    data_home().join("niruwebapp")
}
fn apps_dir() -> PathBuf {
    state_dir().join("apps")
}
fn profiles_dir() -> PathBuf {
    state_dir().join("profiles")
}
fn desktop_dir() -> PathBuf {
    data_home().join("applications")
}
fn icons_dir() -> PathBuf {
    data_home().join("icons/hicolor/128x128/apps")
}
fn state_path() -> PathBuf {
    state_dir().join("apps.tsv")
}

fn fail(message: &str) -> ! {
    eprintln!("niruwebapp: {message}");
    std::process::exit(2);
}
fn die(message: &str) -> ! {
    eprintln!("niruwebapp: {message}");
    std::process::exit(1);
}

fn usage() {
    println!("Usage: niruwebapp <command> [arguments]");
    println!("  create NAME URL [--browser PATH] [--shared-profile] [--icon PATH]");
    println!("  list");
    println!("  launch ID|NAME");
    println!("  remove ID|NAME [--delete-profile]");
    println!("  edit ID|NAME [--name NAME] [--url URL] [--icon PATH]");
    println!("  icon ID|NAME PATH");
}

fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "webapp".into()
    } else {
        out
    }
}

fn validate_url(url: &str) -> Result<(), String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("URL must start with http:// or https://".into());
    }
    if url.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
        return Err("URL must not contain whitespace or control characters".into());
    }
    if url.len() > 4096 {
        return Err("URL is too long".into());
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("name must not be empty".into());
    }
    if name.chars().any(char::is_control) {
        return Err("name must not contain control characters".into());
    }
    if name.chars().count() > 80 {
        return Err("name is too long".into());
    }
    Ok(())
}

fn browser_candidates() -> &'static [&'static str] {
    &[
        "helium",
        "chromium",
        "chromium-browser",
        "google-chrome-stable",
        "brave",
        "brave-browser",
    ]
}

fn browser_available(candidate: &str) -> bool {
    Path::new(candidate).is_file()
        || Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
}

fn find_browser(explicit: Option<&str>) -> String {
    if let Some(path) = explicit {
        if browser_available(path) {
            return path.to_string();
        }
        die(&format!("browser not found: {path}"));
    }
    if let Ok(path) = env::var("NIRUWEBAPP_BROWSER") {
        if !path.is_empty() {
            return find_browser(Some(&path));
        }
    }
    let local_bin = home().join(".local/bin");
    for candidate in browser_candidates() {
        let local = local_bin.join(candidate);
        if local.is_file() {
            return local.to_string_lossy().to_string();
        }
        if browser_available(candidate) {
            return candidate.to_string();
        }
    }
    die("no supported browser found (Helium/Chromium). Install one or use --browser.");
}

fn desktop_exec() -> String {
    let stable = home().join(".local/bin/niruwebapp");
    let path = if stable.is_file() {
        stable
    } else {
        env::current_exe().unwrap_or_else(|_| PathBuf::from("niruwebapp"))
    };
    let text = path.to_string_lossy().to_string();
    if text.chars().any(char::is_whitespace) {
        format!("\"{}\"", text.replace('"', "\\\""))
    } else {
        text
    }
}

fn load() -> Vec<App> {
    let mut apps = Vec::new();
    let Ok(text) = fs::read_to_string(state_path()) else {
        return apps;
    };
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 5 {
            continue;
        }
        apps.push(App {
            id: fields[0].into(),
            name: fields[1].into(),
            url: fields[2].into(),
            browser: fields[3].into(),
            shared_profile: fields[4] == "shared",
            icon: fields.get(5).filter(|s| !s.is_empty()).map(|s| (*s).into()),
        });
    }
    apps
}

fn save(apps: &[App]) {
    fs::create_dir_all(state_dir())
        .unwrap_or_else(|e| die(&format!("create state directory: {e}")));
    let text: String = apps
        .iter()
        .map(|app| {
            format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                app.id,
                app.name,
                app.url,
                app.browser,
                if app.shared_profile {
                    "shared"
                } else {
                    "profile"
                },
                app.icon.as_deref().unwrap_or("")
            )
        })
        .collect();
    let temp = state_path().with_extension("tmp");
    fs::write(&temp, text).unwrap_or_else(|e| die(&format!("write state: {e}")));
    fs::rename(&temp, state_path()).unwrap_or_else(|e| die(&format!("activate state: {e}")));
}

fn desktop(app: &App) {
    let _ = fs::create_dir_all(desktop_dir());
    let _ = fs::create_dir_all(apps_dir());
    let path = desktop_dir().join(format!("niruwebapp-{}.desktop", app.id));
    let icon = app.icon.as_deref().unwrap_or("web-browser");
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName={}\nComment=Webapp\nExec={} launch {}\nIcon={}\nTerminal=false\nCategories=Network;WebBrowser;\nStartupWMClass=niruwebapp-{}\n",
        app.name.replace(['\n', '\r'], " "), desktop_exec(), app.id, icon, app.id
    );
    fs::write(&path, entry).unwrap_or_else(|e| die(&format!("write {}: {e}", path.display())));
}

fn find(apps: &[App], query: &str) -> Option<usize> {
    apps.iter()
        .position(|app| app.id == query || app.name.eq_ignore_ascii_case(query))
}

fn parse_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn install_icon(source: &Path, id: &str, previous: Option<&str>) -> String {
    if !source.is_file() {
        die(&format!("icon not found: {}", source.display()));
    }
    let extension = source
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .filter(|ext| matches!(ext.as_str(), "png" | "svg" | "xpm" | "jpg" | "jpeg"))
        .unwrap_or_else(|| die("icon must be PNG, SVG, XPM or JPEG"));
    let _ = fs::create_dir_all(icons_dir());
    let target = icons_dir().join(format!("niruwebapp-{id}.{extension}"));
    let target_text = target.to_string_lossy().to_string();
    if previous != Some(target_text.as_str()) {
        if let Some(old) = previous {
            let _ = fs::remove_file(old);
        }
    }
    fs::copy(source, &target).unwrap_or_else(|e| die(&format!("copy icon: {e}")));
    target_text
}

fn remove_icon(app: &App) {
    if let Some(icon) = &app.icon {
        if Path::new(icon).starts_with(icons_dir()) {
            let _ = fs::remove_file(icon);
        }
    }
}

fn create(args: &[String]) {
    let name = args
        .first()
        .cloned()
        .unwrap_or_else(|| fail("create needs NAME URL"));
    let url = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| fail("create needs NAME URL"));
    validate_name(&name).unwrap_or_else(|e| fail(&e));
    validate_url(&url).unwrap_or_else(|e| fail(&e));
    let browser = find_browser(parse_value(args, "--browser").as_deref());
    let shared_profile = args.iter().any(|arg| arg == "--shared-profile");
    let mut apps = load();
    let base = slug(&name);
    let mut id = base.clone();
    let mut n = 2;
    while apps.iter().any(|app| app.id == id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    let icon =
        parse_value(args, "--icon").map(|source| install_icon(Path::new(&source), &id, None));
    let app = App {
        id,
        name,
        url,
        browser,
        shared_profile,
        icon,
    };
    if !app.shared_profile {
        let _ = fs::create_dir_all(profiles_dir().join(&app.id));
    }
    desktop(&app);
    apps.push(app.clone());
    save(&apps);
    println!("created {} ({})", app.name, app.id);
}

fn launch(app: &App) {
    let profile = profiles_dir().join(&app.id);
    let _ = fs::create_dir_all(&profile);
    let mut command = Command::new(&app.browser);
    command.args([
        format!("--app={}", app.url),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        format!("--class=niruwebapp-{}", app.id),
    ]);
    if !app.shared_profile {
        command.arg(format!("--user-data-dir={}", profile.display()));
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
        .spawn()
        .map(|_| ())
        .unwrap_or_else(|e| die(&format!("cannot start {}: {e}", app.browser)));
}

fn cmdline_uses_profile(cmdline: &[u8], profile: &std::path::Path) -> bool {
    let expected = format!("--user-data-dir={}", profile.display());
    cmdline
        .split(|byte| *byte == 0)
        .any(|argument| argument == expected.as_bytes())
}

fn profile_in_use(profile: &std::path::Path) -> bool {
    let Ok(entries) = fs::read_dir("/proc") else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        let path = entry.path().join("cmdline");
        if let Ok(cmdline) = fs::read(path) {
            if cmdline_uses_profile(&cmdline, profile) {
                return true;
            }
        }
    }
    false
}

fn remove(args: &[String]) {
    let query = args
        .first()
        .cloned()
        .unwrap_or_else(|| fail("remove needs ID or NAME"));
    let mut apps = load();
    let Some(index) = find(&apps, &query) else {
        die(&format!("webapp not found: {query}"))
    };
    let delete_profile = args.iter().any(|arg| arg == "--delete-profile");
    let profile = profiles_dir().join(&apps[index].id);
    if delete_profile {
        if profile_in_use(&profile) {
            die(&format!(
                "profile is in use by a running webapp; close it and retry: {}",
                profile.display()
            ));
        }
        if profile.exists() {
            fs::remove_dir_all(&profile).unwrap_or_else(|e| {
                die(&format!("cannot remove profile {}: {e}", profile.display()))
            });
        }
    }
    let app = apps.remove(index);
    remove_icon(&app);
    let _ = fs::remove_file(desktop_dir().join(format!("niruwebapp-{}.desktop", app.id)));
    let _ = fs::remove_file(apps_dir().join(format!("{}.toml", app.id)));
    save(&apps);
    println!("removed {}", app.name);
}

fn set_icon(args: &[String]) {
    let query = args
        .first()
        .cloned()
        .unwrap_or_else(|| fail("icon needs ID or NAME and PATH"));
    let source = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| fail("icon needs ID or NAME and PATH"));
    let mut apps = load();
    let Some(index) = find(&apps, &query) else {
        die(&format!("webapp not found: {query}"))
    };
    let previous = apps[index].icon.clone();
    apps[index].icon = Some(install_icon(
        Path::new(&source),
        &apps[index].id,
        previous.as_deref(),
    ));
    desktop(&apps[index]);
    save(&apps);
    println!("updated icon for {}", apps[index].name);
}

fn edit(args: &[String]) {
    let query = args
        .first()
        .cloned()
        .unwrap_or_else(|| fail("edit needs ID or NAME"));
    let mut apps = load();
    let Some(index) = find(&apps, &query) else {
        die(&format!("webapp not found: {query}"))
    };
    if let Some(name) = parse_value(args, "--name") {
        validate_name(&name).unwrap_or_else(|e| fail(&e));
        apps[index].name = name;
    }
    if let Some(url) = parse_value(args, "--url") {
        validate_url(&url).unwrap_or_else(|e| fail(&e));
        apps[index].url = url;
    }
    if let Some(source) = parse_value(args, "--icon") {
        let previous = apps[index].icon.clone();
        apps[index].icon = Some(install_icon(
            Path::new(&source),
            &apps[index].id,
            previous.as_deref(),
        ));
    }
    desktop(&apps[index]);
    save(&apps);
    println!("updated {}", apps[index].name);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--version") {
        println!("NIRUWEBAPP {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.is_empty() || args[0] == "--help" || args[0] == "help" {
        usage();
        return;
    }
    let mut apps = load();
    match args[0].as_str() {
        "create" => create(&args[1..]),
        "list" => {
            for app in apps {
                println!("{}\t{}\t{}", app.id, app.name, app.url);
            }
        }
        "launch" => {
            let query = args
                .get(1)
                .cloned()
                .unwrap_or_else(|| fail("launch needs ID or NAME"));
            let Some(index) = find(&apps, &query) else {
                die(&format!("webapp not found: {query}"))
            };
            launch(&apps.remove(index));
        }
        "remove" => remove(&args[1..]),
        "edit" => edit(&args[1..]),
        "icon" => set_icon(&args[1..]),
        _ => {
            usage();
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slug_is_stable_and_safe() {
        assert_eq!(slug("YouTube Music"), "youtube-music");
        assert_eq!(slug("SVTRadio 24/7"), "svtradio-24-7");
    }
    #[test]
    fn profile_ownership_is_matched_on_the_full_argument() {
        let profile = std::path::Path::new("/tmp/profile x");
        assert!(cmdline_uses_profile(
            b"helium\0--user-data-dir=/tmp/profile x\0",
            profile
        ));
        assert!(!cmdline_uses_profile(
            b"helium\0--user-data-dir=/tmp/profile x-other\0",
            profile
        ));
    }

    #[test]
    fn urls_are_strictly_http() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("file:///tmp/x").is_err());
        assert!(validate_url("https://example.com/a b").is_err());
    }
}
