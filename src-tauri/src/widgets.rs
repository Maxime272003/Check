use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::model::{AppData, Checklist};

pub const PREFIX: &str = "widget-";

pub fn label(id: &str) -> String {
    let safe: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    format!("{PREFIX}{safe}")
}

/// Le haut du post-it (sa barre de déplacement) est-il visible sur au moins un écran ?
pub fn on_screen(app: &AppHandle, x: f64, y: f64, width: f64) -> bool {
    let Ok(monitors) = app.available_monitors() else { return true };
    if monitors.is_empty() {
        return true;
    }
    let (cx, cy) = (x + width.min(160.0) / 2.0, y + 18.0);
    monitors.iter().any(|m| {
        let s = m.scale_factor();
        let p = m.position().to_logical::<f64>(s);
        let z = m.size().to_logical::<f64>(s);
        cx >= p.x && cx <= p.x + z.width && cy >= p.y && cy <= p.y + z.height - 24.0
    })
}

/// Demande à WebView2 de réduire la mémoire d'un post-it inactif (Windows uniquement).
#[cfg(windows)]
pub fn set_low_memory(w: &WebviewWindow, low: bool) {
    let _ = w.with_webview(move |wv| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
        };
        use windows_core::Interface;
        if let Ok(core) = wv.controller().CoreWebView2() {
            if let Ok(c19) = core.cast::<ICoreWebView2_19>() {
                let level = if low { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW } else { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL };
                let _ = c19.SetMemoryUsageTargetLevel(level);
            }
        }
    });
}

#[cfg(not(windows))]
pub fn set_low_memory(_w: &WebviewWindow, _low: bool) {}

fn open(app: &AppHandle, l: &Checklist) -> tauri::Result<()> {
    let url = WebviewUrl::App(format!("index.html?widget={}", l.id).into());
    let mut b = WebviewWindowBuilder::new(app, label(&l.id), url)
        .title(&l.name)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .resizable(false)
        .always_on_top(l.widget.on_top)
        .inner_size(l.widget.width + 16.0, 140.0)
        .focused(false);
    b = match (l.widget.x, l.widget.y) {
        (Some(x), Some(y)) if on_screen(app, x, y, l.widget.width) => b.position(x, y),
        _ => b.center(),
    };
    let w = b.build()?;
    set_low_memory(&w, true);
    Ok(())
}

/// Ramène sur l'écran principal les post-it devenus invisibles (écran débranché, mise à l'échelle…).
pub fn ensure_visible(app: &AppHandle) {
    for (lbl, w) in app.webview_windows() {
        if !lbl.starts_with(PREFIX) {
            continue;
        }
        let scale = w.scale_factor().unwrap_or(1.0);
        let (Ok(pos), Ok(size)) = (w.outer_position(), w.outer_size()) else { continue };
        let p = pos.to_logical::<f64>(scale);
        let z = size.to_logical::<f64>(scale);
        if !on_screen(app, p.x, p.y, z.width) {
            let _ = w.center();
        }
    }
}

/// Ouvre, ferme ou met à jour les fenêtres post-it.
/// `old` = état précédent : seules les checklists dont `pinned` a changé ouvrent/ferment une fenêtre.
/// `old = None` (démarrage) : ouvre toutes les checklists posées.
pub fn sync(app: &AppHandle, old: Option<&AppData>, new: &AppData) {
    for l in &new.lists {
        let lbl = label(&l.id);
        let was_pinned = old
            .map(|o| o.lists.iter().any(|x| x.id == l.id && x.pinned))
            .unwrap_or(false);
        match app.get_webview_window(&lbl) {
            Some(w) if !l.pinned => {
                let _ = w.destroy();
            }
            Some(w) => {
                let _ = w.set_always_on_top(l.widget.on_top);
                let _ = w.set_title(&l.name);
            }
            None if l.pinned && (old.is_none() || !was_pinned) => {
                if let Err(e) = open(app, l) {
                    eprintln!("post-it {}: {e}", l.id);
                }
            }
            None => {}
        }
    }
    for (lbl, w) in app.webview_windows() {
        if lbl.starts_with(PREFIX) && !new.lists.iter().any(|l| label(&l.id) == lbl) {
            let _ = w.destroy();
        }
    }
}
