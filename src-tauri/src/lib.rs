mod apps;
mod data;
mod model;
mod schedule;
mod updater;
mod widgets;

use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::Serialize;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent, Wry};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use data::AppState;
use model::{AppData, Checklist, HistEntry, Settings};

/// Afficher / masquer la fenêtre de Check.
fn shortcut_main() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyC)
}
/// Afficher / ramener devant / masquer les post-it.
fn shortcut_widgets() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyB)
}

fn toggle_widgets(app: &AppHandle) {
    let data = app.state::<AppState>().snapshot();
    widgets::toggle_all(app, &data);
}

struct TrayPause(CheckMenuItem<Wry>);

#[derive(Serialize, Clone)]
struct DataChanged {
    data: AppData,
    origin: String,
}

#[derive(Serialize, Clone)]
struct ListChanged {
    list: Checklist,
    origin: String,
}

#[derive(Serialize, Clone)]
struct SettingsChanged {
    settings: Settings,
    origin: String,
}

pub fn now_ms() -> i64 {
    chrono::Local::now().timestamp_millis()
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

fn toggle_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let visible = w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false);
        if visible && w.is_focused().unwrap_or(false) {
            let _ = w.hide();
        } else {
            show_main(app);
        }
    }
}

fn sync_tray_pause(app: &AppHandle, paused: bool) {
    if let Some(p) = app.try_state::<TrayPause>() {
        let _ = p.0.set_checked(paused);
    }
}

// ---------- Commandes : données ----------

#[tauri::command]
fn get_data(state: tauri::State<AppState>) -> AppData {
    state.snapshot()
}

/// Remplace tout l'état (réordonner, supprimer, annuler, importer).
// Les commandes qui peuvent ouvrir une fenêtre post-it sont `async` : elles tournent hors du fil
// principal. Créer une fenêtre depuis une commande synchrone bloque la boucle de Windows
// (la fenêtre principale ne répondait plus : impossible de la réduire, fermer ou déplacer).

#[tauri::command]
async fn save_data(app: AppHandle, data: AppData, origin: String) {
    let state = app.state::<AppState>();
    let (old, new) = state.replace(data);
    if old.settings.paused != new.settings.paused {
        sync_tray_pause(&app, new.settings.paused);
    }
    let _ = app.emit("data-changed", DataChanged { data: new.clone(), origin });
    widgets::sync(&app, Some(&old), &new);
}

/// Met à jour une seule checklist (cocher, renommer, options du post-it…) : plus léger que `save_data`.
#[tauri::command]
async fn save_list(app: AppHandle, list: Checklist, origin: String) {
    let state = app.state::<AppState>();
    let (old, new, saved) = state.upsert_list(list);
    let _ = app.emit("list-changed", ListChanged { list: saved, origin });
    widgets::sync(&app, Some(&old), &new);
}

#[tauri::command]
async fn save_settings(app: AppHandle, settings: Settings, origin: String) {
    let state = app.state::<AppState>();
    let paused_changed = {
        let mut d = state.data.lock().unwrap();
        let changed = d.settings.paused != settings.paused;
        d.settings = settings.clone();
        changed
    };
    state.dirty.store(true, Ordering::SeqCst);
    if paused_changed {
        sync_tray_pause(&app, settings.paused);
    }
    let _ = app.emit("settings-changed", SettingsChanged { settings, origin });
}

#[tauri::command]
async fn export_data(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let name = format!("check-export-{}.json", chrono::Local::now().format("%Y-%m-%d"));
    let Some(path) = app.dialog().file().add_filter("JSON", &["json"]).set_file_name(&name).blocking_save_file() else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let data = app.state::<AppState>().snapshot();
    let bytes = serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())?;
    std::fs::write(&path, bytes).map_err(|e| format!("Écriture impossible : {e}"))?;
    Ok(Some(path.display().to_string()))
}

/// Importe des checklists depuis un export ou une sauvegarde : elles s'ajoutent aux existantes.
#[tauri::command]
async fn import_data(app: AppHandle) -> Result<usize, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(path) = app.dialog().file().add_filter("JSON", &["json"]).blocking_pick_file() else {
        return Ok(0);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let bytes = std::fs::read(&path).map_err(|e| format!("Lecture impossible : {e}"))?;
    let imported: AppData = serde_json::from_slice(&bytes).map_err(|_| "Ce fichier n’est pas un export Check valide.".to_string())?;
    let count = app.state::<AppState>().import(imported.lists);
    broadcast(&app, "import");
    Ok(count)
}

#[tauri::command]
fn open_backups(app: AppHandle) -> Result<(), String> {
    let dir = app.state::<AppState>().backup_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener().open_path(dir.display().to_string(), None::<&str>).map_err(|e| e.to_string())
}

// ---------- Commandes : système ----------

#[tauri::command]
async fn open_main(app: AppHandle, list_id: Option<String>) {
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
async fn toggle_widgets_cmd(app: AppHandle) {
    toggle_widgets(&app);
}

#[tauri::command]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
fn widget_count(app: AppHandle) -> usize {
    app.webview_windows().keys().filter(|l| l.starts_with(widgets::PREFIX)).count()
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
    let mut changed = false;
    {
        let mut d = state.data.lock().unwrap();
        if d.settings.paused {
            return;
        }
        let now = now_ms();
        for l in d.lists.iter_mut() {
            if l.last_reset == 0 {
                l.last_reset = now;
                changed = true;
                continue;
            }
            let Some(next) = schedule::next_reset(&l.schedule, l.last_reset) else { continue };
            if now < next {
                continue;
            }
            let total = l.items.len() as u32;
            let done = l.items.iter().filter(|i| i.done).count() as u32;
            l.history.push(HistEntry { at: now, done, total, completed: l.completed_this_period || (total > 0 && done == total) });
            if l.history.len() > model::HISTORY_MAX {
                let extra = l.history.len() - model::HISTORY_MAX;
                l.history.drain(0..extra);
            }
            l.completed_this_period = false;
            l.items.iter_mut().for_each(|i| i.done = false);
            l.last_reset = now;
            if done > 0 {
                reset_names.push(l.name.clone());
            }
            changed = true;
        }
    }
    if changed {
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
            // Les écritures sont regroupées : au plus une par seconde.
            if state.dirty.load(Ordering::SeqCst) {
                state.save_now();
            }
            if tick % 15 == 1 {
                run_resets(&app);
                widgets::ensure_visible(&app);
            }
            if tick % (6 * 3600) == 0 {
                state.backup();
            }
        }
    });
}

// ---------- Zone de notification ----------

fn build_tray(app: &AppHandle, paused: bool) -> tauri::Result<()> {
    let open_i = MenuItem::with_id(app, "open", "Ouvrir Check", true, Some("Ctrl+Alt+C"))?;
    let new_i = MenuItem::with_id(app, "new", "Nouvelle checklist", true, None::<&str>)?;
    let widgets_i = MenuItem::with_id(app, "widgets", "Afficher / masquer les post-it", true, Some("Ctrl+Alt+B"))?;
    let pause_i = CheckMenuItem::with_id(app, "pause", "Suspendre les remises à zéro", true, paused, None::<&str>)?;
    let settings_i = MenuItem::with_id(app, "settings", "Paramètres", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_i, &new_i, &widgets_i, &pause_i, &settings_i, &sep, &quit_i])?;
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
            "widgets" => toggle_widgets(app),
            "settings" => {
                show_main(app);
                let _ = app.emit_to("main", "ui-action", "settings");
            }
            "pause" => {
                let state = app.state::<AppState>();
                let settings = {
                    let mut d = state.data.lock().unwrap();
                    d.settings.paused = !d.settings.paused;
                    d.settings.clone()
                };
                sync_tray_pause(app, settings.paused);
                state.dirty.store(true, Ordering::SeqCst);
                let _ = app.emit("settings-changed", SettingsChanged { settings, origin: "tray".into() });
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if shortcut.id() == shortcut_main().id() {
                        toggle_main(app);
                    } else if shortcut.id() == shortcut_widgets().id() {
                        toggle_widgets(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            app.manage(apps::Running::default());
            let (state, first_run) = AppState::load(dir)?;
            let data = state.snapshot();
            state.backup();
            app.manage(state);
            if first_run {
                let _ = app.autolaunch().enable();
            }
            for sc in [shortcut_main(), shortcut_widgets()] {
                if let Err(e) = app.global_shortcut().register(sc) {
                    eprintln!("raccourci global indisponible : {e}");
                }
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
            apps::start_watcher(handle.clone());
            start_ticker(handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label().to_string();
            let app = window.app_handle();
            match event {
                WindowEvent::CloseRequested { api, .. } if label == "main" => {
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
                    let state = app.state::<AppState>();
                    let mut d = state.data.lock().unwrap();
                    if let Some(l) = d.lists.iter_mut().find(|l| widgets::label(&l.id) == label) {
                        l.widget.x = Some(p.x);
                        l.widget.y = Some(p.y);
                        l.widget.px = Some(pos.x);
                        l.widget.py = Some(pos.y);
                        state.dirty.store(true, Ordering::SeqCst);
                    }
                }
                WindowEvent::Focused(focused) if label.starts_with(widgets::PREFIX) => {
                    if let Some(w) = app.get_webview_window(&label) {
                        widgets::set_low_memory(&w, !focused);
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_data,
            save_data,
            save_list,
            save_settings,
            export_data,
            import_data,
            open_backups,
            open_main,
            get_autostart,
            set_autostart,
            app_version,
            widget_count,
            toggle_widgets_cmd,
            open_url,
            quit_app,
            apps::running_apps,
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
