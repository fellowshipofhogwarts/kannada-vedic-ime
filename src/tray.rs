//! tray.rs — system-tray UI, owns the main thread's event loop.
//!
//! * `tao` provides the event loop (a Win32 message loop on Windows, GTK on
//!   Linux) which `tray-icon` needs on the thread that created the icon.
//! * The keyboard hook runs on its own thread and talks to this loop through an
//!   `EventLoopProxy` (hotkey pressed, hook failed) — never by touching tray
//!   objects directly.
//! * Menu clicks arrive through `muda`'s global handler, also forwarded to the
//!   loop as user events, so *all* tray mutation happens on this thread.
//! * The update check runs on a short-lived background thread and reports back
//!   through the same proxy.
//!
//! Icons are raw 64×64 RGBA blobs embedded with `include_bytes!` (no image
//! decoder linked in); regenerate them with `tools/make_icons.py`.

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::settings::Settings;
use crate::state::{AppState, ToggleSource};
use crate::{guide, hook, update, Config};

const ICON_PX: u32 = 64;
static ICON_ON: &[u8] = include_bytes!("../assets/icons/on_64.rgba");
static ICON_OFF: &[u8] = include_bytes!("../assets/icons/off_64.rgba");

enum UserEvent {
    Menu(MenuEvent),
    Toggled(ToggleSource),
    /// The keyboard or mouse hook could not start (reason for the toast).
    HookFailed(String),
    /// Result of a GitHub check; `manual` = the user clicked the menu item.
    UpdateChecked { latest: Option<String>, manual: bool },
}

struct Tray {
    icon: TrayIcon,
    check: CheckMenuItem,
    vedic: CheckMenuItem,
    digits: CheckMenuItem,
    update_item: MenuItem,
    auto_check: CheckMenuItem,
    on: Icon,
    off: Icon,
    toggle_id: MenuId,
    vedic_id: MenuId,
    digits_id: MenuId,
    guide_id: MenuId,
    update_id: MenuId,
    auto_id: MenuId,
    quit_id: MenuId,
    /// Set once a newer release has been seen.
    update_tag: Option<String>,
}

fn tooltip(active: bool) -> &'static str {
    if active {
        "Kannada IME: ON (English → ಕನ್ನಡ)  Ctrl+Shift+K"
    } else {
        "Kannada IME: OFF (native pass-through)  Ctrl+Shift+K"
    }
}

fn check_label() -> String {
    format!("Check for updates (this is v{})", update::CURRENT)
}

impl Tray {
    fn new(active: bool, settings: Settings) -> Result<Self, Box<dyn std::error::Error>> {
        let on = Icon::from_rgba(ICON_ON.to_vec(), ICON_PX, ICON_PX)?;
        let off = Icon::from_rgba(ICON_OFF.to_vec(), ICON_PX, ICON_PX)?;

        let check = CheckMenuItem::new("Kannada typing (Ctrl+Shift+K)", true, active, None);
        let vedic = CheckMenuItem::new("Vedic mode (accent keys, ꣳ)", true, settings.vedic, None);
        let digits = CheckMenuItem::new(
            "Kannada digits (೧ ೨ ೩); untick for 1 2 3",
            true,
            settings.kannada_digits,
            None,
        );
        let guide_item = MenuItem::new("Phonetic && Vedic quick reference", true, None);
        let update_item = MenuItem::new(check_label(), true, None);
        let auto_check = CheckMenuItem::new(
            "Check for updates at startup",
            true,
            settings.update_check,
            None,
        );
        let quit = MenuItem::new("Exit", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &check,
            &vedic,
            &digits,
            &guide_item,
            &PredefinedMenuItem::separator(),
            &update_item,
            &auto_check,
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(tooltip(active))
            .with_icon(if active { on.clone() } else { off.clone() })
            .build()?;

        Ok(Self {
            icon,
            toggle_id: check.id().clone(),
            vedic_id: vedic.id().clone(),
            digits_id: digits.id().clone(),
            guide_id: guide_item.id().clone(),
            update_id: update_item.id().clone(),
            auto_id: auto_check.id().clone(),
            quit_id: quit.id().clone(),
            check,
            vedic,
            digits,
            update_item,
            auto_check,
            on,
            off,
            update_tag: None,
        })
    }

    /// Reflect the engine state in icon, tooltip and menu check mark.
    fn show(&mut self, active: bool) {
        let _ = self.icon.set_icon(Some(if active {
            self.on.clone()
        } else {
            self.off.clone()
        }));
        let _ = self.icon.set_tooltip(Some(tooltip(active)));
        self.check.set_checked(active);
    }
}

/// Windows shows a toast under the name registered for its AppUserModelID.
/// Without one, toasts borrow "Windows PowerShell". We register our own name
/// once per user (a tiny HKCU registry entry, no admin needed). If that fails
/// for any reason we keep the old behaviour, so a toast is never lost.
#[cfg(windows)]
const TOAST_ID: &str = "KannadaVedicIME.Typer";

#[cfg(windows)]
fn toast_key() -> String {
    format!("HKCU\\Software\\Classes\\AppUserModelId\\{TOAST_ID}")
}

/// Run Windows' own `reg.exe` by full path (0.5.3: by bare name Windows would
/// look in the program's folder first) without a console window.
#[cfg(windows)]
fn reg(args: &[&str]) -> bool {
    use std::os::windows::process::CommandExt;
    std::process::Command::new(update::system32().join("reg.exe"))
        .args(args)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn toast_identity_ready() -> bool {
    use std::sync::OnceLock;
    static READY: OnceLock<bool> = OnceLock::new();
    *READY.get_or_init(|| {
        let key = toast_key();
        // Already registered (every launch after the first): nothing to write.
        reg(&["query", &key, "/v", "DisplayName"])
            || reg(&["add", &key, "/v", "DisplayName", "/t", "REG_SZ", "/d", "Kannada IME", "/f"])
    })
}

/// OS-native toast, off the UI thread (notification back-ends can block on D-Bus
/// or WinRT for a moment).
fn notify(title: &str, body: &str) {
    let title = title.to_string();
    let body = body.to_string();
    std::thread::spawn(move || {
        let mut n = notify_rust::Notification::new();
        n.appname("Kannada IME")
            .summary(&title)
            .body(&body)
            .timeout(notify_rust::Timeout::Milliseconds(4000));
        #[cfg(windows)]
        if toast_identity_ready() {
            n.app_id(TOAST_ID);
        }
        let _ = n.show();
    });
}

fn toast(active: bool, source: ToggleSource) {
    let (title, body) = if active {
        (
            "Kannada typing ON",
            "English → ಕನ್ನಡ (phonetic). Ctrl+Shift+K to turn off.",
        )
    } else {
        (
            "Kannada typing OFF",
            "Keyboard is back to normal pass-through.",
        )
    };
    let via = match source {
        ToggleSource::Hotkey => "hotkey",
        ToggleSource::Menu => "tray menu",
    };
    notify(title, &format!("{body}  (via {via})"));
}

const GUIDE_PREFIX: &str = "kannada_ime_quick_reference_";

/// The guide file this run created (deleted again on Exit).
static GUIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Write the quick-reference page to a new file in the temp folder. The name
/// includes the process id and the time, and `create_new` refuses any name that
/// already exists, so a file or link somebody else planted is never used
/// (0.5.3: an existing file used to be opened as it was).
fn write_guide() -> std::io::Result<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    for n in 0..16u32 {
        let path = std::env::temp_dir().join(format!(
            "{GUIDE_PREFIX}{}_{stamp}_{n}.html",
            std::process::id()
        ));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                f.write_all(guide::html().as_bytes())?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "no free file name in the temp folder",
    ))
}

/// Open the quick-reference page, writing it the first time.
fn open_guide() {
    let mut guide = GUIDE.lock().unwrap_or_else(|e| e.into_inner());
    let path = match guide.as_ref().filter(|p| p.is_file()) {
        Some(p) => p.clone(),
        None => match write_guide() {
            Ok(p) => {
                *guide = Some(p.clone());
                p
            }
            Err(e) => {
                eprintln!("kannada_ime: cannot write guide: {e}");
                return;
            }
        },
    };
    let _ = open::that(&path);
}

/// Delete the guide file of this run (on Exit).
fn remove_own_guide() {
    if let Some(p) = GUIDE.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = std::fs::remove_file(p);
    }
}

/// Delete guide files left behind by earlier runs (0.5.3: they were never
/// deleted). Only plain files with our name pattern that are older than
/// `min_age` are touched; links are never followed.
fn remove_old_guides(min_age: std::time::Duration) -> usize {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(GUIDE_PREFIX) || !name.ends_with(".html") {
            continue;
        }
        let Ok(kind) = entry.file_type() else { continue };
        let old_enough = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age >= min_age);
        if kind.is_file() && old_enough && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// `--cleanup`: remove everything the program leaves on this computer, so it can
/// be deleted without traces (0.5.3). Returns one line per step, for printing.
pub fn cleanup() -> Vec<String> {
    let mut report = vec![
        "Kannada IME clean-up (exit the program from the tray menu first).".to_string(),
    ];
    match crate::settings::folder() {
        Some(dir) if dir.exists() => match std::fs::remove_dir_all(&dir) {
            Ok(()) => report.push(format!("Removed the settings folder {}", dir.display())),
            Err(e) => report.push(format!("Could not remove {}: {e}", dir.display())),
        },
        _ => report.push("No settings folder found.".to_string()),
    }
    #[cfg(windows)]
    {
        let key = toast_key();
        if !reg(&["query", &key]) {
            report.push("No registry entry found.".to_string());
        } else if reg(&["delete", &key, "/f"]) {
            report.push(format!("Removed the registry entry {key}"));
        } else {
            report.push(format!("Could not remove the registry entry {key}"));
        }
    }
    let n = remove_old_guides(std::time::Duration::ZERO);
    report.push(format!("Removed {n} quick-reference file(s) from the temp folder."));
    report.push("Done. You can now delete kannada_ime.exe and the font if you like.".to_string());
    report
}

/// Save the settings; tell the user if that failed (0.5.3: failures were silent).
fn save_settings(settings: &Settings) {
    if let Err(e) = settings.save() {
        notify(
            "Kannada IME: settings not saved",
            &format!("Your choice works now but will be forgotten after a restart ({e})."),
        );
    }
}

fn start_update_check(proxy: &tao::event_loop::EventLoopProxy<UserEvent>, manual: bool) {
    let p = proxy.clone();
    std::thread::spawn(move || {
        let latest = update::fetch_latest_tag();
        let _ = p.send_event(UserEvent::UpdateChecked { latest, manual });
    });
}

/// Build everything and run the UI loop. Never returns.
pub fn run(cfg: Config) -> ! {
    let (mut settings, problems) = Settings::load_report();
    if !problems.is_empty() {
        // Typos in a hand-edited settings.txt: say so once (0.5.3).
        let mut body = problems.iter().take(3).cloned().collect::<Vec<_>>().join("\n");
        if problems.len() > 3 {
            body.push_str(&format!("\n… and {} more.", problems.len() - 3));
        }
        notify("Kannada IME: settings.txt", &body);
    }
    // Guide pages from earlier runs (an hour old, so another running copy on
    // Linux keeps its own).
    remove_old_guides(std::time::Duration::from_secs(3600));
    let state = AppState::new(!cfg.start_off, settings.vedic, settings.kannada_digits);

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    {
        let p = proxy.clone();
        MenuEvent::set_event_handler(Some(move |e| {
            let _ = p.send_event(UserEvent::Menu(e));
        }));
    }

    {
        let p = proxy.clone();
        let p_err = proxy.clone();
        hook::spawn(hook::HookConfig {
            state: Arc::clone(&state),
            mode: cfg.inject,
            on_hotkey: Arc::new(move || {
                let _ = p.send_event(UserEvent::Toggled(ToggleSource::Hotkey));
            }),
            on_error: Arc::new(move |msg| {
                let _ = p_err.send_event(UserEvent::HookFailed(msg));
            }),
        });
    }

    if settings.update_check {
        start_update_check(&proxy, false);
    }

    let mut tray: Option<Tray> = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            // Linux/macOS require the tray to be created once the loop runs.
            Event::NewEvents(StartCause::Init) => match Tray::new(state.is_active(), settings) {
                Ok(t) => tray = Some(t),
                Err(e) => {
                    eprintln!("kannada_ime: cannot create tray icon: {e}");
                    *control_flow = ControlFlow::Exit;
                }
            },
            Event::UserEvent(UserEvent::Toggled(source)) => {
                if let Some(t) = tray.as_mut() {
                    let active = state.is_active();
                    t.show(active);
                    toast(active, source);
                }
            }
            Event::UserEvent(UserEvent::HookFailed(msg)) => {
                // 0.5.1: also used for a failed mouse hook, so the title names
                // the problem generally; the message says what is affected.
                notify("Kannada IME: input problem", &msg);
            }
            Event::UserEvent(UserEvent::UpdateChecked { latest, manual }) => {
                let Some(t) = tray.as_mut() else { return };
                match latest {
                    Some(tag) if update::is_newer(&tag, update::CURRENT) => {
                        t.update_item.set_text(format!("Download update ({tag})"));
                        notify(
                            "New version available",
                            &format!(
                                "Kannada IME {tag} is out (you have v{}). \
                                 Tray menu → Download update.",
                                update::CURRENT
                            ),
                        );
                        t.update_tag = Some(tag);
                        if manual {
                            let _ = open::that(update::RELEASES_URL);
                        }
                    }
                    Some(_) if manual => notify(
                        "Kannada IME is up to date",
                        &format!("You have the latest version (v{}).", update::CURRENT),
                    ),
                    None if manual => notify(
                        "Could not check for updates",
                        "No answer from GitHub. Check your internet connection and try again.",
                    ),
                    _ => {} // automatic check: say nothing unless there is news
                }
            }
            Event::UserEvent(UserEvent::Menu(ev)) => {
                let Some(t) = tray.as_mut() else { return };
                if ev.id == t.toggle_id {
                    let active = state.toggle();
                    t.show(active); // also fixes the auto-toggled check mark
                    toast(active, ToggleSource::Menu);
                } else if ev.id == t.vedic_id {
                    // The check mark has already flipped itself: read the new value.
                    let on = t.vedic.is_checked();
                    state.set_vedic(on);
                    settings.vedic = on;
                    save_settings(&settings);
                    notify(
                        if on { "Vedic mode ON" } else { "Vedic mode OFF" },
                        if on {
                            "Accent keys and ꣳ are active."
                        } else {
                            "Accent keys type normally; no ꣳ."
                        },
                    );
                } else if ev.id == t.digits_id {
                    let on = t.digits.is_checked();
                    state.set_digits(on);
                    settings.kannada_digits = on;
                    save_settings(&settings);
                    notify(
                        if on { "Kannada digits" } else { "English digits" },
                        if on {
                            "The number keys type ೧ ೨ ೩ (numpad too)."
                        } else {
                            "The number keys type 1 2 3 as usual."
                        },
                    );
                } else if ev.id == t.guide_id {
                    open_guide();
                } else if ev.id == t.update_id {
                    if t.update_tag.is_some() {
                        let _ = open::that(update::RELEASES_URL);
                    } else {
                        start_update_check(&proxy, true);
                    }
                } else if ev.id == t.auto_id {
                    settings.update_check = t.auto_check.is_checked();
                    save_settings(&settings);
                } else if ev.id == t.quit_id {
                    tray.take(); // drop the icon so it vanishes immediately
                    remove_own_guide();
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    })
}
