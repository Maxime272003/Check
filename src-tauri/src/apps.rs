//! Post-it liés à une application : ils apparaissent quand le programme se lance
//! et disparaissent quand il se ferme. On interroge la liste des processus toutes les 2 s,
//! seulement si au moins une checklist posée est liée à un programme.

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tauri::{AppHandle, Manager};

use crate::data::AppState;
use crate::widgets;

/// Noms (en minuscules) des programmes en cours d'exécution.
#[derive(Default)]
pub struct Running(pub Mutex<HashSet<String>>);

pub fn normalize(name: &str) -> String {
    let n = name.trim().to_lowercase();
    if n.is_empty() || n.ends_with(".exe") || !cfg!(windows) { n } else { format!("{n}.exe") }
}

fn snapshot(sys: &mut System) -> HashSet<String> {
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes().values().map(|p| p.name().to_string_lossy().to_lowercase()).collect()
}

pub fn is_running(app: &AppHandle, exe: &str) -> bool {
    app.try_state::<Running>()
        .map(|r| r.0.lock().unwrap().contains(&normalize(exe)))
        .unwrap_or(false)
}

pub fn start_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let Ok(mut sys) = System::new() else {
            eprintln!("liste des programmes indisponible : post-it liés à une application désactivés");
            return;
        };
        loop {
            let needed = app
                .state::<AppState>()
                .snapshot()
                .lists
                .iter()
                .any(|l| l.pinned && l.widget.show_with.as_deref().is_some_and(|s| !s.is_empty()));
            if needed {
                let now = snapshot(&mut sys);
                let changed = {
                    let running = app.state::<Running>();
                    let mut cur = running.0.lock().unwrap();
                    let changed = *cur != now;
                    *cur = now;
                    changed
                };
                if changed {
                    let data = app.state::<AppState>().snapshot();
                    widgets::apply_app_triggers(&app, &data);
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

#[derive(Serialize)]
pub struct AppEntry {
    pub exe: String,
}

/// Programmes en cours (hors programmes système), pour choisir à quoi lier un post-it.
#[tauri::command]
pub async fn running_apps() -> Vec<AppEntry> {
    let Ok(mut sys) = System::new() else { return Vec::new() };
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet));
    let mut seen = HashSet::new();
    let mut out: Vec<AppEntry> = sys
        .processes()
        .values()
        .filter(|p| {
            let exe = p.exe().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            !exe.is_empty() && !exe.starts_with("c:\\windows\\") && !exe.contains("\\windowsapps\\microsoft.")
        })
        .map(|p| p.name().to_string_lossy().to_lowercase())
        .filter(|n| seen.insert(n.clone()))
        .map(|exe| AppEntry { exe })
        .collect();
    out.sort_by(|a, b| a.exe.cmp(&b.exe));
    out
}
