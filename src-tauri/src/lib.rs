//! on-screen-translator core.
//!
//! Split per ADR-0001: everything expensive lives here (OS access, pixels,
//! network, secrets) and the webview only draws. See CLAUDE.md for the
//! invariants this crate must not break.

pub mod capture;
pub mod error;
pub mod hotkey;
pub mod ipc;
pub mod logging;
pub mod pipeline;
pub mod protocol;
pub mod session;
pub mod settings;
pub mod tray;
pub mod windows_mgr;

use tauri::Manager;
use tracing::{error, info, warn};

use hotkey::HotkeyState;
use session::SessionStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Held for the process lifetime; dropping it stops the log writer.
    let _log_guard = logging::init();
    install_panic_hook();

    let sessions = SessionStore::new();

    tauri::Builder::default()
        // FR-04: a second launch signals the first and exits, so there is
        // never a second tray icon or a second global hook.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            info!("second instance signalled; focusing the existing one");
            let _ = app.get_webview_window(windows_mgr::SETTINGS).map(|w| {
                let _ = w.show();
                let _ = w.set_focus();
            });
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(sessions.clone())
        .manage(HotkeyState::new())
        .register_uri_scheme_protocol(protocol::SCHEME, move |ctx, request| {
            let sessions = ctx.app_handle().state::<SessionStore>();
            protocol::handle(&sessions, &request)
        })
        .invoke_handler(tauri::generate_handler![
            ipc::get_capture_frame,
            ipc::translate_region,
            ipc::retranslate,
            ipc::cancel_capture,
            ipc::close_overlay,
            ipc::close_panel,
            ipc::get_panel_result,
            ipc::get_settings,
            ipc::save_settings,
            ipc::set_hotkey,
            ipc::set_provider_key,
            ipc::has_provider_key,
            ipc::clear_provider_key,
            ipc::test_provider,
            ipc::set_autostart,
            ipc::list_ocr_languages,
            ipc::copy_to_clipboard,
            ipc::open_settings,
            ipc::open_external,
            ipc::get_app_info,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let loaded = settings::load();
            match &loaded.outcome {
                settings::LoadOutcome::RecoveredFromCorrupt(reason) => {
                    // FR-61: start on defaults and say so, never crash and
                    // never silently reset.
                    warn!(reason, "settings file was unreadable; started on defaults");
                }
                settings::LoadOutcome::CreatedDefaults => {
                    info!(
                        target_language = %loaded.settings.target_language,
                        "no settings file; using defaults"
                    );
                }
                settings::LoadOutcome::Loaded => {}
            }

            // FR-12: if this fails we start anyway, in a degraded state with a
            // visible reason, rather than refusing to run.
            let result = hotkey::register(&handle, &loaded.settings.hotkey);
            if !result.ok {
                warn!(
                    requested = %loaded.settings.hotkey,
                    message = ?result.message,
                    "starting without a global hotkey"
                );
            }

            tray::build(&handle)?;

            info!(
                version = %handle.package_info().version,
                hotkey_ok = result.ok,
                "started"
            );
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build the application")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                // `code` distinguishes the two ways an exit is requested:
                //   None     - the last window closed. For a tray app that is
                //              normal life, not a reason to die (FR-01).
                //   Some(..) - someone called app.exit(), i.e. the user chose
                //              Quit. Preventing that leaves a tray icon the
                //              user cannot get rid of, which NFR-R5 forbids.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}

/// A panic in a tray app is otherwise an invisible death: no window disappears,
/// nothing is reported, the hotkey just stops working (NFR-R6).
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".into());
        error!(location, payload = %info, "panic");
        previous(info);
    }));
}
