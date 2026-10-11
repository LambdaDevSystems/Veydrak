mod config;
mod guard;
mod lan;
mod pty;
mod state;
mod sysmon;

use state::AppState;

pub fn run() {
    if let Err(msg) = guard::refuse_root() {
        eprintln!("{msg}");
        std::process::exit(1);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            pty::manager::create_pty,
            pty::manager::write_pty,
            pty::manager::resize_pty,
            pty::manager::close_pty,
            config::get_config,
            config::set_theme,
            lan::roster::get_roster,
            lan::roster::toggle_pin,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            lan::roster::load_pins(&handle);
            tauri::async_runtime::spawn(sysmon::monitor_loop(handle.clone()));
            tauri::async_runtime::spawn(lan::ping::run_roster_loop(handle));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
