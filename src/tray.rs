//! tray.rs — system-tray UI, owns the main thread's event loop.
//!
//! * `tao` provides the event loop (a Win32 message loop on Windows, GTK on
//!   Linux) which `tray-icon` needs on the thread that created the icon.
//! * The keyboard hook runs on its own thread and talks to this loop through an
//!   `EventLoopProxy` (hotkey pressed) — never by touching tray objects directly.
//! * Menu clicks arrive through `muda`'s global handler, also forwarded to the
//!   loop as user events, so *all* tray mutation happens on this thread.
//!
//! Icons are raw 64×64 RGBA blobs embedded with `include_bytes!` (no image
//! decoder linked in); regenerate them with `tools/make_icons.py`.

use std::sync::Arc;

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::state::{AppState, ToggleSource};
use crate::{guide, hook, Config};

const ICON_PX: u32 = 64;
static ICON_ON: &[u8] = include_bytes!("../assets/icons/on_64.rgba");
static ICON_OFF: &[u8] = include_bytes!("../assets/icons/off_64.rgba");

enum UserEvent {
    Menu(MenuEvent),
    Toggled(ToggleSource),
}

struct Tray {
    icon: TrayIcon,
    check: CheckMenuItem,
    on: Icon,
    off: Icon,
    toggle_id: MenuId,
    guide_id: MenuId,
    quit_id: MenuId,
}

fn tooltip(active: bool) -> &'static str {
    if active {
        "Kannada IME: ON (English → ಕನ್ನಡ)  Ctrl+Shift+K"
    } else {
        "Kannada IME: OFF (native pass-through)  Ctrl+Shift+K"
    }
}

impl Tray {
    fn new(active: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let on = Icon::from_rgba(ICON_ON.to_vec(), ICON_PX, ICON_PX)?;
        let off = Icon::from_rgba(ICON_OFF.to_vec(), ICON_PX, ICON_PX)?;

        let check = CheckMenuItem::new("Kannada typing (Ctrl+Shift+K)", true, active, None);
        let guide_item = MenuItem::new("Phonetic && Vedic quick reference", true, None);
        let quit = MenuItem::new("Exit", true, None);

        let menu = Menu::new();
        menu.append_items(&[&check, &guide_item, &PredefinedMenuItem::separator(), &quit])?;

        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(tooltip(active))
            .with_icon(if active { on.clone() } else { off.clone() })
            .build()?;

        Ok(Self {
            icon,
            toggle_id: check.id().clone(),
            guide_id: guide_item.id().clone(),
            quit_id: quit.id().clone(),
            check,
            on,
            off,
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

/// OS-native toast, off the UI thread (notification back-ends can block on D-Bus
/// or WinRT for a moment).
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
    let body = format!("{body}  (via {via})");
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .appname("Kannada IME")
            .summary(title)
            .body(&body)
            .timeout(notify_rust::Timeout::Milliseconds(2500))
            .show();
    });
}

fn open_guide() {
    let path = std::env::temp_dir().join("kannada_ime_quick_reference.html");
    match std::fs::write(&path, guide::html()) {
        Ok(()) => {
            let _ = open::that(&path);
        }
        Err(e) => eprintln!("kannada_ime: cannot write guide: {e}"),
    }
}

/// Build everything and run the UI loop. Never returns.
pub fn run(cfg: Config) -> ! {
    let state = AppState::new(!cfg.start_off);

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
        hook::spawn(hook::HookConfig {
            state: Arc::clone(&state),
            mode: cfg.inject,
            on_hotkey: Arc::new(move || {
                let _ = p.send_event(UserEvent::Toggled(ToggleSource::Hotkey));
            }),
        });
    }

    let mut tray: Option<Tray> = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            // Linux/macOS require the tray to be created once the loop runs.
            Event::NewEvents(StartCause::Init) => match Tray::new(state.is_active()) {
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

            Event::UserEvent(UserEvent::Menu(ev)) => {
                let Some(t) = tray.as_mut() else { return };
                if ev.id == t.toggle_id {
                    let active = state.toggle();
                    t.show(active); // also fixes the auto-toggled check mark
                    toast(active, ToggleSource::Menu);
                } else if ev.id == t.guide_id {
                    open_guide();
                } else if ev.id == t.quit_id {
                    tray.take(); // drop the icon so it vanishes immediately
                    *control_flow = ControlFlow::Exit;
                }
            }

            _ => {}
        }
    })
}
