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
use std::sync::Arc;

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
fn toast_identity_ready() -> bool {
    use std::os::windows::process::CommandExt;
    use std::sync::OnceLock;
    static READY: OnceLock<bool> = OnceLock::new();
    *READY.get_or_init(|| {
        let key = format!("HKCU\\Software\\Classes\\AppUserModelId\\{TOAST_ID}");
        std::process::Command::new("reg")
            .args(["add", &key, "/v", "DisplayName", "/t", "REG_SZ", "/d", "Kannada IME", "/f"])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
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

/// Write the quick-reference page to a per-process file in the temp folder and
/// open it. `create_new` refuses to follow a file or link that somebody else
/// planted under that name.
fn open_guide() {
    let path = std::env::temp_dir().join(format!(
        "kannada_ime_quick_reference_{}.html",
        std::process::id()
    ));
    if !path.exists() {
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .and_then(|mut f| f.write_all(guide::html().as_bytes()));
        if let Err(e) = written {
            eprintln!("kannada_ime: cannot write guide: {e}");
            return;
        }
    }
    let _ = open::that(&path);
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
    let mut settings = Settings::load();
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
                    settings.save();
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
                    settings.save();
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
                    settings.save();
                } else if ev.id == t.quit_id {
                    tray.take(); // drop the icon so it vanishes immediately
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    })
}
