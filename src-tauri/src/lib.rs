mod model;
mod schedule;
mod updater;
mod widgets;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use model::AppData;

pub struct AppState {
    data: Mutex<AppData>,
    path: PathBuf,
    dirty: AtomicBool,
}

struct TrayPause(CheckMenuItem<Wry>);

#[derive(Serialize, Clone)]
struct DataChanged {
    data: AppData,
    origin: String,
}

fn now_ms() -> i64 {
    chrono::Local::now().timestamp_millis()
}

impl AppState {
    fn snapshot(&self) -> AppData {
        self.data.lock().unwrap().clone()
    }

    fn save_now(&self) {
        let data = self.snapshot();
        match serde_json::to_vec_pretty(&data) {
            Ok(bytes) => {
                let tmp = self.path.with_extension("json.tmp");
                if std::fs::write(&tmp, bytes).is_ok() {
                    let _ = std::fs::rename(&tmp, &self.path);
                }
            }
            Err(e) => eprintln!("sauvegarde: {e}"),
        }
        self.dirty.store(false, Ordering::SeqCst);
    }
}

fn load(path: &PathBuf) -> (AppData, bool) {
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice::<AppData>(&bytes) {
            Ok(d) => (d, false),
            Err(e) => {
                eprintln!("données illisibles ({e}), sauvegarde de secours créée");
                let _ = std::fs::copy(path, path.with_extension("json.bak"));
                (model::seed(now_ms()), false)
            }
        },
        Err(_) => (model::seed(now_ms()), true),
    }
}

fn broadcast(app: &AppHandle, origin: &str) {
    let data = app.state::<AppState>().snapshot();
    let _ = app.emit("data-changed", DataChanged { data, origin: origin.into() });
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

// ---------- Commandes ----------

#[tauri::command]
fn get_data(state: tauri::State<AppState>) -> AppData {
    state.snapshot()
}

#[tauri::command]
fn save_data(app: AppHandle, state: tauri::State<AppState>, mut data: AppData, origin: String) {
    let old = {
        let mut cur = state.data.lock().unwrap();
        let now = now_ms();
        for l in data.lists.iter_mut() {
            // La position des post-it est gérée côté Rust (événements de déplacement).
            if let Some(o) = cur.lists.iter().find(|o| o.id == l.id) {
                l.widget.x = o.widget.x;
                l.widget.y = o.widget.y;
            }
            if l.last_reset == 0 {
                l.last_reset = now;
            }
        }
        std::mem::replace(&mut *cur, data.clone())
    };
    if old.settings.paused != data.settings.paused {
        if let Some(p) = app.try_state::<TrayPause>() {
            let _ = p.0.set_checked(data.settings.paused);
        }
    }
    state.save_now();
    let _ = app.emit("data-changed", DataChanged { data: data.clone(), origin });
    widgets::sync(&app, Some(&old), &data);
}

#[tauri::command]
fn open_main(app: AppHandle, list_id: Option<String>) {
    show_main(&app);
    if let Some(id) = list_id {
        let _ = app.emit_to("main", "focus-list", id);
    }
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let m = app.autolaunch();
    if enabled { m.enable() } else { m.disable() }.map_err(|e| e.to_string())?;
    Ok(m.is_enabled().unwrap_or(enabled))
}

#[tauri::command]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with("https://github.com/") {
        return Err("Lien refusé".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.state::<AppState>().save_now();
    app.exit(0);
}

// ---------- Planificateur ----------

fn run_resets(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut reset_names = Vec::new();
    {
        let mut d = state.data.lock().unwrap();
        if d.settings.paused {
            return;
        }
        let now = now_ms();
        for l in d.lists.iter_mut() {
            if l.last_reset == 0 {
                l.last_reset = now;
                continue;
            }
            if let Some(next) = schedule::next_reset(&l.schedule, l.last_reset) {
                if now >= next {
                    let had_checked = l.items.iter().any(|i| i.done);
                    l.items.iter_mut().for_each(|i| i.done = false);
                    l.last_reset = now;
                    if had_checked {
                        reset_names.push(l.name.clone());
                    }
                    state.dirty.store(true, Ordering::SeqCst);
                }
            }
        }
    }
    if state.dirty.load(Ordering::SeqCst) {
        state.save_now();
        broadcast(app, "scheduler");
    }
    if !reset_names.is_empty() && state.snapshot().settings.notify_reset {
        let body = if reset_names.len() == 1 {
            format!("« {} » a été décochée.", reset_names[0])
        } else {
            format!("{} checklists ont été décochées.", reset_names.len())
        };
        let _ = app.notification().builder().title("Check").body(body).show();
    }
}

fn start_ticker(app: AppHandle) {
    std::thread::spawn(move || {
        let mut tick: u64 = 0;
        loop {
            std::thread::sleep(Duration::from_secs(1));
            tick += 1;
            let state = app.state::<AppState>();
            if tick % 15 == 1 {
                run_resets(&app);
            }
            if state.dirty.load(Ordering::SeqCst) && tick % 2 == 0 {
                state.save_now();
            }
        }
    });
}

// ---------- Zone de notification ----------

fn build_tray(app: &AppHandle, paused: bool) -> tauri::Result<()> {
    let open_i = MenuItem::with_id(app, "open", "Ouvrir Check", true, None::<&str>)?;
    let new_i = MenuItem::with_id(app, "new", "Nouvelle checklist", true, None::<&str>)?;
    let pause_i = CheckMenuItem::with_id(app, "pause", "Suspendre les remises à zéro", true, paused, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, "settings", "Paramètres", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_i, &new_i, &pause_i, &settings_i, &sep, &quit_i])?;
    app.manage(TrayPause(pause_i));

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Check")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "open" => show_main(app),
            "new" => {
                show_main(app);
                let _ = app.emit_to("main", "ui-action", "new");
            }
            "settings" => {
                show_main(app);
                let _ = app.emit_to("main", "ui-action", "settings");
            }
            "pause" => {
                let state = app.state::<AppState>();
                let paused = {
                    let mut d = state.data.lock().unwrap();
                    d.settings.paused = !d.settings.paused;
                    d.settings.paused
                };
                if let Some(p) = app.try_state::<TrayPause>() {
                    let _ = p.0.set_checked(paused);
                }
                state.save_now();
                broadcast(app, "tray");
            }
            "quit" => {
                app.state::<AppState>().save_now();
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

// ---------- Application ----------

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("data.json");
            let (data, first_run) = load(&path);
            app.manage(AppState { data: Mutex::new(data.clone()), path, dirty: AtomicBool::new(first_run) });
            if first_run {
                let _ = app.autolaunch().enable();
            }
            let handle = app.handle().clone();
            build_tray(&handle, data.settings.paused)?;

            let hidden_launch = std::env::args().any(|a| a == "--hidden");
            if !(hidden_launch && data.settings.start_hidden) {
                show_main(&handle);
            }
            if data.settings.restore_widgets {
                widgets::sync(&handle, None, &data);
            }
            start_ticker(handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label().to_string();
            match event {
                WindowEvent::CloseRequested { api, .. } if label == "main" => {
                    let app = window.app_handle();
                    if app.state::<AppState>().snapshot().settings.close_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        app.state::<AppState>().save_now();
                        app.exit(0);
                    }
                }
                WindowEvent::Moved(pos) if label.starts_with(widgets::PREFIX) => {
                    let scale = window.scale_factor().unwrap_or(1.0);
                    let p = pos.to_logical::<f64>(scale);
                    let state = window.app_handle().state::<AppState>();
                    let mut d = state.data.lock().unwrap();
                    if let Some(l) = d.lists.iter_mut().find(|l| widgets::label(&l.id) == label) {
                        l.widget.x = Some(p.x);
                        l.widget.y = Some(p.y);
                        state.dirty.store(true, Ordering::SeqCst);
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_data,
            save_data,
            open_main,
            get_autostart,
            set_autostart,
            app_version,
            open_url,
            quit_app,
            updater::check_update,
            updater::install_update
        ])
        .build(tauri::generate_context!())
        .expect("erreur au lancement de Check")
        .run(|_app, event| {
            // L'app reste vivante dans la zone de notification quand toutes les fenêtres sont fermées.
            if let RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
