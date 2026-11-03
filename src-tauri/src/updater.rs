use futures_util::StreamExt;
use serde::Serialize;
use std::io::Write;
use tauri::{AppHandle, Emitter};

/// Dépôt GitHub dont les releases servent de source de mises à jour.
pub const REPO: &str = "Maxime272003/Check";

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: Option<String>,
    pub available: bool,
    pub notes: String,
    pub url: Option<String>,
    pub asset_name: Option<String>,
    pub size: u64,
    pub html_url: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress {
    downloaded: u64,
    total: u64,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Check-desktop-app")
        .build()
        .map_err(|e| e.to_string())
}

fn parse_version(v: &str) -> Option<semver::Version> {
    semver::Version::parse(v.trim().trim_start_matches('v')).ok()
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    let resp = client()?
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Impossible de joindre GitHub : {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(UpdateInfo { current, ..Default::default() });
    }
    let rel: serde_json::Value = resp
        .error_for_status()
        .map_err(|e| format!("Réponse GitHub inattendue : {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let tag = rel["tag_name"].as_str().unwrap_or_default().to_string();
    let available = match (parse_version(&tag), parse_version(&current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    };
    let asset = rel["assets"].as_array().and_then(|a| {
        a.iter().find(|x| {
            x["name"].as_str().map(|n| n.ends_with("-setup.exe")).unwrap_or(false)
        })
    });
    Ok(UpdateInfo {
        current,
        latest: Some(tag.trim_start_matches('v').to_string()),
        available,
        notes: rel["body"].as_str().unwrap_or_default().to_string(),
        url: asset.and_then(|a| a["browser_download_url"].as_str().map(String::from)),
        asset_name: asset.and_then(|a| a["name"].as_str().map(String::from)),
        size: asset.and_then(|a| a["size"].as_u64()).unwrap_or(0),
        html_url: rel["html_url"].as_str().map(String::from),
    })
}

/// Télécharge l'installateur, le lance en mode passif puis quitte l'application.
#[tauri::command]
pub async fn install_update(app: AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with(&format!("https://github.com/{REPO}/releases/download/")) {
        return Err("Adresse de téléchargement refusée".into());
    }
    let resp = client()?
        .get(&url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Téléchargement impossible : {e}"))?;
    let total = resp.content_length().unwrap_or(0);
    let path = std::env::temp_dir().join("Check-update-setup.exe");
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();
    let mut last_emit = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        if downloaded - last_emit > 64 * 1024 || downloaded == total {
            last_emit = downloaded;
            let _ = app.emit("update-progress", Progress { downloaded, total });
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);

    #[cfg(windows)]
    {
        std::process::Command::new(&path)
            .args(["/P", "/UPDATE"])
            .spawn()
            .map_err(|e| format!("Impossible de lancer l'installateur : {e}"))?;
        app.exit(0);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("L'installation automatique n'est disponible que sous Windows.".into())
    }
}
