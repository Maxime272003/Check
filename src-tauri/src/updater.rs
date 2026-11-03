//! Mises à jour via tauri-plugin-updater : `latest.json` de la dernière release GitHub,
//! installateur signé (clé publique dans tauri.conf.json, clé privée dans les secrets de la CI).

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    /// `None` : aucune release publiée.
    pub latest: Option<String>,
    pub available: bool,
    pub notes: String,
    pub html_url: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress {
    downloaded: u64,
    total: u64,
}

fn release_url(version: &str) -> String {
    format!("https://github.com/Maxime272003/Check/releases/tag/v{version}")
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(u)) => Ok(UpdateInfo {
            latest: Some(u.version.clone()),
            available: true,
            notes: u.body.clone().unwrap_or_default(),
            html_url: Some(release_url(&u.version)),
            current,
        }),
        Ok(None) => Ok(UpdateInfo { latest: Some(current.clone()), current, ..Default::default() }),
        Err(tauri_plugin_updater::Error::ReleaseNotFound) => Ok(UpdateInfo { current, ..Default::default() }),
        Err(e) => Err(format!("Vérification impossible : {e}")),
    }
}

/// Télécharge, vérifie la signature, installe (mode passif) puis relance Check.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Aucune mise à jour disponible")?;

    let mut downloaded: u64 = 0;
    let progress_app = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = progress_app.emit("update-progress", Progress { downloaded, total: total.unwrap_or(0) });
            },
            || {},
        )
        .await
        .map_err(|e| format!("Installation impossible : {e}"))?;
    app.restart();
}
