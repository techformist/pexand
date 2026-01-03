#![windows_subsystem = "windows"]

use crossbeam_channel::unbounded;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use pexand::db::{Bootstrapper, Database};
use pexand::sentinel::Sentinel;
use pexand::ui::{run_ui, SystemTray, UiExternalMessage};
use std::sync::{Arc, Mutex};

fn main() -> iced::Result {
    // Initialize database and get connection
    let conn = match Database::init() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Failed to initialize database: {:?}", e);
            std::process::exit(1);
        }
    };

    // Bootstrap default snippets on first run
    if let Err(e) = Bootstrapper::seed_defaults(&conn) {
        eprintln!("Failed to seed defaults: {:?}", e);
        std::process::exit(1);
    }

    // Wrap connection in Arc<Mutex<>> for sharing across threads
    let db_conn = Arc::new(Mutex::new(conn));

    // Create and start the Sentinel with shared connection
    let sentinel = match Sentinel::new(Arc::clone(&db_conn)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create Sentinel: {:?}", e);
            std::process::exit(1);
        }
    };

    let (_handle, tx) = sentinel.start();

    // Channel for UI window control
    let (ui_tx, ui_rx) = unbounded();

    // Initialize system tray
    let _tray = match SystemTray::new(ui_tx.clone()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to create system tray: {:?}", e);
            std::process::exit(1);
        }
    };

    // Setup global hotkey (Ctrl+Shift+Alt+P)
    let hotkey_manager = GlobalHotKeyManager::new().expect("Failed to create hotkey manager");
    let hotkey = HotKey::new(
        Some(Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT),
        Code::KeyP,
    );
    hotkey_manager
        .register(hotkey)
        .expect("Failed to register global hotkey");

    // Listen for hotkey events and forward to UI (always show on hotkey)
    let hk_rx = GlobalHotKeyEvent::receiver();
    {
        let ui_tx = ui_tx.clone();
        std::thread::spawn(move || {
            for event in hk_rx.iter() {
                if event.id == hotkey.id() {
                    let _ = ui_tx.send(UiExternalMessage::Show);
                }
            }
        });
    }

    // Run the UI with shared connection (starts minimized to tray)
    run_ui(db_conn, tx, ui_rx)
}
