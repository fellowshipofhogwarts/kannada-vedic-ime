//! kannada_ime — a tiny background agent that turns phonetic English keystrokes
//! into Kannada Unicode (with Vedic accent marks) in any focused text field.
//!
//! Module map
//!   mapping.rs  Baraha tables + lookup           (pure, tested)
//!   engine.rs   buffer → render → minimal edit   (pure, tested)
//!   guide.rs    quick-reference HTML from tables (pure, tested)
//!   state.rs    ON/OFF flag shared across threads
//!   inject.rs   Edit → synthetic input (Unicode / clipboard strategies)
//!   hook/       global keyboard hook, per OS     (win.rs, linux.rs)
//!   tray.rs     tray icon, menu, toasts, event loop

// No console window for release builds on Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod engine;
mod guide;
mod hook;
mod inject;
mod mapping;
mod state;
mod tray;

pub struct Config {
    pub start_off: bool,
    pub inject: inject::Mode,
}

const HELP: &str = "\
kannada_ime — phonetic Kannada typing agent

USAGE: kannada_ime [OPTIONS]

  --off                    start with typing disabled (tray + hotkey still work)
  --inject <unicode|clipboard>
                           how text is delivered (default: unicode)
  --translit <TEXT>        print the Kannada for TEXT and exit (no hooks)
  -h, --help               this help

Toggle at runtime with Ctrl+Shift+K or the tray menu.";

fn main() {
    let mut cfg = Config {
        start_off: false,
        inject: inject::Mode::Unicode,
    };

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--off" => cfg.start_off = true,
            "--inject" => match args.next().as_deref().and_then(inject::Mode::parse) {
                Some(m) => cfg.inject = m,
                None => return usage_error("--inject expects 'unicode' or 'clipboard'"),
            },
            "--translit" => {
                let text = args.collect::<Vec<_>>().join(" ");
                println!("{}", engine::transliterate(&text));
                return;
            }
            "-h" | "--help" => {
                println!("{HELP}");
                return;
            }
            other => return usage_error(&format!("unknown option {other:?}")),
        }
    }

    if !hook::single_instance() {
        return; // already running; its tray icon is the UI
    }

    tray::run(cfg)
}

fn usage_error(msg: &str) {
    eprintln!("kannada_ime: {msg}\n\n{HELP}");
    std::process::exit(2);
}
