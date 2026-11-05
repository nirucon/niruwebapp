mod class;
mod desktop;
mod icon;
mod x11;

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
    icon_kind: IconKind,
}

/// Where an app's icon came from. Stored as the seventh state column; older
/// rows without it are `Legacy` (a user icon, or the generic browser icon).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum IconKind {
    Legacy,
    Letter,
    User,
    Fetched,
}

impl IconKind {
    fn as_str(self) -> &'static str {
        match self {
            IconKind::Legacy => "",
            IconKind::Letter => "letter",
            IconKind::User => "user",
            IconKind::Fetched => "fetched",
        }
    }
    fn parse(text: &str) -> Self {
        match text {
            "letter" => IconKind::Letter,
            "user" => IconKind::User,
            "fetched" => IconKind::Fetched,
            _ => IconKind::Legacy,
        }
    }
}

impl App {
    /// The WM_CLASS value this webapp's windows carry (see `class`).
    fn expected_class(&self) -> String {
        class::expected_class(&self.id, &self.url, self.shared_profile)
    }

    fn owns_window(&self, window: &x11::ClientWindow) -> bool {
        class::window_matches(
            &self.id,
            &self.url,
            self.shared_profile,
            &window.instance,
            &window.class,
        )
    }

    fn desktop_path(&self) -> PathBuf {
        desktop_dir().join(format!("niruwebapp-{}.desktop", self.id))
    }
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
fn config_home() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
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
    println!("  create NAME URL [--browser PATH] [--shared-profile]");
    println!("       [--icon PATH | --fetch-icon]");
    println!("  list");
    println!("  launch ID|NAME [--new-window]");
    println!("  remove ID|NAME [--delete-profile]");
    println!("  edit ID|NAME [--name NAME] [--url URL] [--icon PATH | --fetch-icon]");
    println!("       [--shared-profile | --isolated-profile]");
    println!("  icon ID|NAME PATH");
    println!("  refresh            regenerate letter icons and desktop entries");
    println!("  doctor");
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

/// The program desktop entries run: the stable NIRUX link when present.
fn desktop_exec() -> String {
    let stable = home().join(".local/bin/niruwebapp");
    let path = if stable.is_file() {
        stable
    } else {
        env::current_exe().unwrap_or_else(|_| PathBuf::from("niruwebapp"))
    };
    path.to_string_lossy().to_string()
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
            icon_kind: IconKind::parse(fields.get(6).copied().unwrap_or("")),
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
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                app.id,
                app.name,
                app.url,
                app.browser,
                if app.shared_profile {
                    "shared"
                } else {
                    "profile"
                },
                app.icon.as_deref().unwrap_or(""),
                app.icon_kind.as_str()
            )
        })
        .collect();
    let temp = state_path().with_extension("tmp");
    fs::write(&temp, text).unwrap_or_else(|e| die(&format!("write state: {e}")));
    fs::rename(&temp, state_path()).unwrap_or_else(|e| die(&format!("activate state: {e}")));
}

fn desktop_text(app: &App) -> String {
    let exec = desktop_exec();
    // The host tells webapps apart in launchers and makes them searchable by
    // domain.
    let comment = match class::display_host(&app.url) {
        Some(host) => format!("Webapp · {host}"),
        None => "Webapp".to_string(),
    };
    desktop::Entry {
        name: &app.name,
        comment: &comment,
        exec: &[&exec, "launch", &app.id],
        icon: app.icon.as_deref().unwrap_or("web-browser"),
        wm_class: &app.expected_class(),
    }
    .text()
}

fn desktop(app: &App) {
    let _ = fs::create_dir_all(desktop_dir());
    let _ = fs::create_dir_all(apps_dir());
    let path = app.desktop_path();
    icon::write_atomic(&path, desktop_text(app).as_bytes())
        .unwrap_or_else(|e| die(&format!("write {}: {e}", path.display())));
}

/// Read-only report: every webapp, its expected class, and whether a window
/// with that class is open now.
fn doctor(apps: &[App]) {
    let windows = x11::Windows::connect();
    let clients = windows.as_ref().map(x11::Windows::clients);
    if clients.is_none() {
        println!("note: no X display; window state unknown");
    }
    println!("ID\tPROFILE\tEXPECTED CLASS\tWINDOWS\tENTRY\tBROWSER");
    for app in apps {
        let open = clients
            .as_ref()
            .map(|list| list.iter().filter(|w| app.owns_window(w)).count());
        let entry = match fs::read_to_string(app.desktop_path()) {
            Ok(text) if text == desktop_text(app) => "ok",
            Ok(_) => "stale",
            Err(_) => "missing",
        };
        let browser = if browser_exists(&app.browser) {
            "ok"
        } else {
            "missing"
        };
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            app.id,
            if app.shared_profile {
                "shared"
            } else {
                "isolated"
            },
            app.expected_class(),
            open.map_or_else(|| "?".to_string(), |n| n.to_string()),
            entry,
            browser
        );
    }
    for window in clients.iter().flatten() {
        if window.class.starts_with("niruwebapp-")
            && !apps.iter().any(|app| app.owns_window(window))
        {
            println!(
                "note: window 0x{:x} ({} {}) matches no webapp",
                window.id, window.instance, window.class
            );
        }
    }
}

/// Whether a stored browser (absolute path or PATH name) still exists.
fn browser_exists(browser: &str) -> bool {
    if browser.contains('/') {
        return Path::new(browser).is_file();
    }
    env::var_os("PATH")
        .map(|paths| env::split_paths(&paths).any(|dir| dir.join(browser).is_file()))
        .unwrap_or(false)
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

fn profile_mode_from_args(args: &[String]) -> Result<Option<bool>, &'static str> {
    let shared = args.iter().any(|arg| arg == "--shared-profile");
    let isolated = args.iter().any(|arg| arg == "--isolated-profile");
    match (shared, isolated) {
        (true, true) => Err("choose either --shared-profile or --isolated-profile"),
        (true, false) => Ok(Some(true)),
        (false, true) => Ok(Some(false)),
        (false, false) => Ok(None),
    }
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

fn letter_icon_path(id: &str) -> PathBuf {
    icons_dir().join(format!("niruwebapp-{id}.png"))
}

/// Point the app at its own icon file, removing a previous one we own.
fn adopt_icon(app: &mut App, path: &Path, kind: IconKind) {
    let path = path.to_string_lossy().to_string();
    if let Some(old) = app.icon.as_deref().filter(|old| *old != path) {
        if Path::new(old).starts_with(icons_dir()) {
            let _ = fs::remove_file(old);
        }
    }
    app.icon = Some(path);
    app.icon_kind = kind;
}

/// Switch the app to a letter icon; `paint_letter_icons` draws it.
fn use_letter_icon(app: &mut App) {
    adopt_icon(app, &letter_icon_path(&app.id), IconKind::Letter);
}

/// Draw every letter icon in the current accent colour. Labels depend on the
/// other names (a shared first letter gets a second letter), so this runs
/// after every change to the set of names.
fn paint_letter_icons(apps: &[App]) {
    let mut painter = None;
    for app in apps.iter().filter(|app| app.icon_kind == IconKind::Letter) {
        let others = apps
            .iter()
            .filter(|other| other.id != app.id)
            .map(|other| other.name.as_str());
        let label = icon::label_for(&app.name, others);
        let painter =
            painter.get_or_insert_with(|| icon::Painter::new(icon::accent(&config_home())));
        if let Err(e) = painter.write(&letter_icon_path(&app.id), &label) {
            eprintln!("niruwebapp: cannot write icon for {}: {e}", app.id);
        }
    }
}

/// `--fetch-icon`: the explicit opt-in network request. Keeps the current
/// icon (or a letter icon) when nothing usable is found.
fn fetch_icon(app: &mut App) {
    let Some(host) = class::url_parts(&app.url).map(|parts| parts.host) else {
        return use_letter_icon(app);
    };
    match icon::fetch_site_icon(&host) {
        Ok(png) => {
            let path = letter_icon_path(&app.id);
            icon::write_atomic(&path, &png).unwrap_or_else(|e| die(&format!("write icon: {e}")));
            adopt_icon(app, &path, IconKind::Fetched);
            println!("fetched icon from {host}");
        }
        Err(e) => {
            eprintln!("niruwebapp: no icon fetched from {host} ({e}); keeping the local icon");
            if app.icon.is_none() {
                use_letter_icon(app);
            }
        }
    }
}

/// Give pre-0.2 rows without an icon a letter icon, redraw the letter icons
/// and rewrite every desktop entry.
fn refresh_apps(apps: &mut [App]) {
    for app in apps.iter_mut() {
        match app.icon_kind {
            IconKind::Legacy if app.icon.is_none() => use_letter_icon(app),
            IconKind::Legacy => app.icon_kind = IconKind::User,
            _ => {}
        }
    }
    paint_letter_icons(apps);
    for app in apps.iter() {
        desktop(app);
    }
    save(apps);
}

const STATE_FORMAT: &str = "2";

/// One-time, idempotent upgrade of pre-0.2 state: back up the state file and
/// every desktop entry, give icon-less apps a letter icon, and rewrite the
/// entries through `desktop()` (new class, comment and escaping).
fn migrate_if_needed() {
    let marker = state_dir().join("format");
    if !state_path().exists()
        || fs::read_to_string(&marker).is_ok_and(|text| text.trim() == STATE_FORMAT)
    {
        return;
    }
    let mut apps = load();
    let backup = state_dir().join("backup-0.1");
    if fs::create_dir(&backup).is_ok() {
        let _ = fs::copy(state_path(), backup.join("apps.tsv"));
        for app in &apps {
            let entry = app.desktop_path();
            if let Some(name) = entry.file_name() {
                let _ = fs::copy(&entry, backup.join(name));
            }
        }
    }
    refresh_apps(&mut apps);
    if let Err(e) = fs::write(&marker, format!("{STATE_FORMAT}\n")) {
        eprintln!("niruwebapp: cannot record state format: {e}");
        return;
    }
    eprintln!(
        "niruwebapp: upgraded {} webapp(s); backup in {}",
        apps.len(),
        backup.display()
    );
}

fn mark_state_current() {
    let _ = fs::write(state_dir().join("format"), format!("{STATE_FORMAT}\n"));
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
    let user_icon =
        parse_value(args, "--icon").map(|source| install_icon(Path::new(&source), &id, None));
    let mut app = App {
        id,
        name,
        url,
        browser,
        shared_profile,
        icon_kind: if user_icon.is_some() {
            IconKind::User
        } else {
            IconKind::Letter
        },
        icon: user_icon,
    };
    if app.icon.is_none() {
        if args.iter().any(|arg| arg == "--fetch-icon") {
            fetch_icon(&mut app);
        } else {
            use_letter_icon(&mut app);
        }
    }
    if !app.shared_profile {
        let _ = fs::create_dir_all(profiles_dir().join(&app.id));
    }
    desktop(&app);
    apps.push(app.clone());
    paint_letter_icons(&apps);
    save(&apps);
    mark_state_current();
    println!("created {} ({})", app.name, app.id);
}

/// A second launch within this window, with no window mapped yet, is taken
/// as a double click while the browser starts.
const LAUNCH_GUARD: std::time::Duration = std::time::Duration::from_secs(3);

fn launch_guard_path(app: &App) -> PathBuf {
    let base = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(state_dir);
    base.join("niruwebapp").join(format!("{}.launch", app.id))
}

fn recently_launched(app: &App) -> bool {
    fs::metadata(launch_guard_path(app))
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.elapsed().ok())
        .is_some_and(|age| age < LAUNCH_GUARD)
}

fn mark_launched(app: &App) {
    let path = launch_guard_path(app);
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(path, b"");
}

/// Activate an open window of this webapp. Returns false when none is open
/// or there is no X display.
fn activate_existing(app: &App) -> bool {
    let Some(windows) = x11::Windows::connect() else {
        return false;
    };
    let Some(window) = windows.clients().into_iter().find(|w| app.owns_window(w)) else {
        return false;
    };
    windows.activate(window.id)
}

fn launch_command(apps: &mut Vec<App>, args: &[String]) {
    let query = args
        .first()
        .cloned()
        .unwrap_or_else(|| fail("launch needs ID or NAME"));
    let Some(index) = find(apps, &query) else {
        die(&format!("webapp not found: {query}"))
    };
    let new_window = args.iter().any(|arg| arg == "--new-window");
    let app = apps.remove(index);
    if !new_window {
        if activate_existing(&app) {
            return;
        }
        if recently_launched(&app) {
            eprintln!(
                "niruwebapp: {} is already starting; use --new-window for another window",
                app.name
            );
            return;
        }
    }
    mark_launched(&app);
    launch(&app);
}

fn launch(app: &App) {
    let profile = profiles_dir().join(&app.id);
    let _ = fs::create_dir_all(&profile);
    let mut command = Command::new(&app.browser);
    command.args([
        format!("--app={}", class::ascii_url(&app.url)),
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
    let _ = fs::remove_file(app.desktop_path());
    let _ = fs::remove_file(apps_dir().join(format!("{}.toml", app.id)));
    paint_letter_icons(&apps);
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
    apps[index].icon_kind = IconKind::User;
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
        apps[index].icon_kind = IconKind::User;
    } else if args.iter().any(|arg| arg == "--fetch-icon") {
        fetch_icon(&mut apps[index]);
    }
    let mut profile_changed = false;
    match profile_mode_from_args(args) {
        Ok(Some(shared)) => {
            if shared != apps[index].shared_profile {
                apps[index].shared_profile = shared;
                profile_changed = true;
            }
            if !shared {
                let _ = fs::create_dir_all(profiles_dir().join(&apps[index].id));
            }
        }
        Ok(None) => {}
        Err(message) => fail(message),
    }
    paint_letter_icons(&apps);
    desktop(&apps[index]);
    save(&apps);
    // A new URL or profile mode means a different window; do not let the
    // double-launch guard swallow the next launch.
    let _ = fs::remove_file(launch_guard_path(&apps[index]));
    println!("updated {}", apps[index].name);
    if profile_changed {
        eprintln!(
            "note: restart {} for the new profile mode to take effect",
            apps[index].name
        );
    }
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
    migrate_if_needed();
    let mut apps = load();
    match args[0].as_str() {
        "create" => create(&args[1..]),
        "list" => {
            for app in apps {
                println!("{}\t{}\t{}", app.id, app.name, app.url);
            }
        }
        "launch" => launch_command(&mut apps, &args[1..]),
        "remove" => remove(&args[1..]),
        "edit" => edit(&args[1..]),
        "icon" => set_icon(&args[1..]),
        "doctor" => doctor(&apps),
        "refresh" => {
            refresh_apps(&mut apps);
            mark_state_current();
            println!("refreshed {} webapp(s)", apps.len());
        }
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

    #[test]
    fn profile_mode_flags_are_exclusive() {
        let args = |items: &[&str]| {
            items
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(profile_mode_from_args(&args(&[])), Ok(None));
        assert_eq!(
            profile_mode_from_args(&args(&["--shared-profile"])),
            Ok(Some(true))
        );
        assert_eq!(
            profile_mode_from_args(&args(&["--isolated-profile"])),
            Ok(Some(false))
        );
        assert!(
            profile_mode_from_args(&args(&["--shared-profile", "--isolated-profile"])).is_err()
        );
    }
}
