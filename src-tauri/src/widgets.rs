use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::model::{AppData, Checklist};

pub const PREFIX: &str = "widget-";

pub fn label(id: &str) -> String {
    let safe: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    format!("{PREFIX}{safe}")
}

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
        (Some(x), Some(y)) => b.position(x, y),
        _ => b.center(),
    };
    b.build()?;
    Ok(())
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
