use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::apps;
use crate::model::{AppData, Checklist};

pub const PREFIX: &str = "widget-";

pub fn label(id: &str) -> String {
    let safe: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    format!("{PREFIX}{safe}")
}

/// Version en pixels réels (coordonnées du bureau virtuel, tous écrans confondus) :
/// le haut du post-it est-il sur un écran branché ?
pub fn on_screen_phys(app: &AppHandle, x: i32, y: i32) -> bool {
    let Ok(monitors) = app.available_monitors() else { return true };
    if monitors.is_empty() {
        return true;
    }
    monitors.iter().any(|m| {
        let p = m.position();
        let z = m.size();
        let (cx, cy) = (x + (80.0 * m.scale_factor()) as i32, y + (20.0 * m.scale_factor()) as i32);
        cx >= p.x && cx < p.x + z.width as i32 && cy >= p.y && cy < p.y + z.height as i32
    })
}

/// Ancien format (coordonnées logiques) : le haut du post-it est-il visible sur au moins un écran ?
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

/// Marge transparente autour du post-it dans sa fenêtre (voir Widget.svelte).
const MARGIN: f64 = 10.0;

/// Où poser un nouveau post-it : près de la souris, dans la zone de travail de l'écran,
/// en se décalant si un autre post-it occupe déjà la place (comme Pense).
fn spot_near_cursor(app: &AppHandle, width: f64, height: f64) -> Option<(f64, f64)> {
    let c = app.cursor_position().ok()?;
    let monitors = app.available_monitors().ok()?;
    let m = monitors
        .iter()
        .find(|m| {
            let p = m.position();
            let z = m.size();
            c.x >= p.x as f64 && c.x < (p.x + z.width as i32) as f64 && c.y >= p.y as f64 && c.y < (p.y + z.height as i32) as f64
        })
        .cloned()
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let s = m.scale_factor();
    let area = m.work_area();
    let ap = area.position.to_logical::<f64>(s);
    let az = area.size.to_logical::<f64>(s);
    let taken: Vec<(f64, f64)> = app
        .webview_windows()
        .iter()
        .filter(|(l, _)| l.starts_with(PREFIX))
        .filter_map(|(_, w)| w.outer_position().ok().map(|p| p.to_logical::<f64>(w.scale_factor().unwrap_or(s))))
        .map(|p| (p.x, p.y))
        .collect();
    let mut x = c.x / s - width / 2.0;
    let mut y = c.y / s - 24.0;
    for _ in 0..12 {
        if !taken.iter().any(|(tx, ty)| (tx - x).abs() < 28.0 && (ty - y).abs() < 28.0) {
            break;
        }
        x += 28.0;
        y += 28.0;
    }
    let x = x.clamp(ap.x, (ap.x + az.width - width).max(ap.x));
    let y = y.clamp(ap.y, (ap.y + az.height - height).max(ap.y));
    Some((x, y))
}

fn open(app: &AppHandle, l: &Checklist) -> tauri::Result<()> {
    // L'identifiant passe par un script d'initialisation : un `?widget=` dans l'URL ne survit pas
    // au protocole de l'app une fois compilée, et la fenêtre affichait alors l'app complète.
    let url = WebviewUrl::App("index.html".into());
    let init = format!("window.__CHECK_WIDGET__ = {};", serde_json::to_string(&l.id).unwrap_or_else(|_| "null".into()));
    let w = l.widget.width + 2.0 * MARGIN;
    let h = l.widget.height.unwrap_or(140.0) + 2.0 * MARGIN;
    let mut b = WebviewWindowBuilder::new(app, label(&l.id), url)
        .title(&l.name)
        .initialization_script(&init)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .resizable(true)
        .maximizable(false)
        .minimizable(false)
        .min_inner_size(220.0 + 2.0 * MARGIN, 60.0)
        .max_inner_size(560.0 + 2.0 * MARGIN, 1200.0)
        .always_on_top(l.widget.on_top)
        .inner_size(w, h)
        // Créée cachée : le post-it s'affiche lui-même une fois chargé et à la bonne taille.
        .visible(false)
        .focused(false);
    // Position mémorisée en pixels réels : elle désigne sans ambiguïté un écran précis, même si
    // les écrans n'ont pas la même mise à l'échelle. On la pose avant que la fenêtre s'affiche.
    let saved_phys = match (l.widget.px, l.widget.py) {
        (Some(x), Some(y)) if on_screen_phys(app, x, y) => Some(PhysicalPosition::new(x, y)),
        _ => None,
    };
    if saved_phys.is_none() {
        b = match (l.widget.x, l.widget.y) {
            (Some(x), Some(y)) if on_screen(app, x, y, l.widget.width) => b.position(x, y),
            _ => match spot_near_cursor(app, w, h) {
                Some((x, y)) => b.position(x, y),
                None => b.center(),
            },
        };
    }
    let win = b.build()?;
    if let Some(p) = saved_phys {
        let _ = win.set_position(p);
    }
    set_low_memory(&win, true);
    Ok(())
}

/// Ctrl+Alt+B et menu de l'icône : post-it cachés → on les montre ; visibles mais derrière
/// les fenêtres → on les ramène devant ; l'un d'eux a le focus → on les cache.
pub fn toggle_all(app: &AppHandle, data: &AppData) {
    let wins: Vec<WebviewWindow> = app
        .webview_windows()
        .into_iter()
        .filter(|(l, _)| l.starts_with(PREFIX))
        .map(|(_, w)| w)
        .collect();
    if wins.is_empty() {
        sync(app, None, data);
        return;
    }
    let any_hidden = wins.iter().any(|w| !w.is_visible().unwrap_or(true));
    let any_focused = wins.iter().any(|w| w.is_focused().unwrap_or(false));
    if !any_hidden && any_focused {
        for w in &wins {
            let _ = w.hide();
        }
        return;
    }
    for w in &wins {
        let on_top = data
            .lists
            .iter()
            .find(|l| label(&l.id) == w.label())
            .map(|l| l.widget.on_top)
            .unwrap_or(false);
        let _ = w.show();
        // Passe devant puis reprend son réglage « premier plan ».
        let _ = w.set_always_on_top(true);
        let _ = w.set_always_on_top(on_top);
    }
    if let Some(w) = wins.last() {
        let _ = w.set_focus();
    }
}

/// Ramène sur l'écran principal les post-it devenus invisibles (écran débranché, mise à l'échelle…).
pub fn ensure_visible(app: &AppHandle) {
    for (lbl, w) in app.webview_windows() {
        if !lbl.starts_with(PREFIX) || !w.is_visible().unwrap_or(false) {
            continue;
        }
        let Ok(pos) = w.outer_position() else { continue };
        if !on_screen_phys(app, pos.x, pos.y) {
            let _ = w.center();
        }
    }
}

/// Le post-it doit-il être affiché maintenant ? (posé, et son programme lancé s'il en a un)
pub fn wanted(app: &AppHandle, l: &Checklist) -> bool {
    l.pinned
        && match l.widget.show_with.as_deref().map(str::trim) {
            Some(exe) if !exe.is_empty() => apps::is_running(app, exe),
            _ => true,
        }
}

/// Un programme s'est lancé ou fermé : ouvre ou ferme les post-it qui lui sont liés.
pub fn apply_app_triggers(app: &AppHandle, data: &AppData) {
    for l in data.lists.iter().filter(|l| l.pinned && l.widget.show_with.as_deref().is_some_and(|s| !s.trim().is_empty())) {
        let want = wanted(app, l);
        match app.get_webview_window(&label(&l.id)) {
            Some(w) if !want => {
                let _ = w.destroy();
            }
            None if want => {
                if let Err(e) = open(app, l) {
                    eprintln!("post-it {}: {e}", l.id);
                }
            }
            _ => {}
        }
    }
}

/// Ouvre, ferme ou met à jour les fenêtres post-it.
/// `old` = état précédent : seules les checklists dont `pinned` a changé ouvrent/ferment une fenêtre.
/// `old = None` (démarrage) : ouvre toutes les checklists posées.
pub fn sync(app: &AppHandle, old: Option<&AppData>, new: &AppData) {
    for l in &new.lists {
        let lbl = label(&l.id);
        let want = wanted(app, l);
        let was_wanted = old
            .and_then(|o| o.lists.iter().find(|x| x.id == l.id))
            .map(|x| wanted(app, x))
            .unwrap_or(false);
        match app.get_webview_window(&lbl) {
            Some(w) if !want => {
                let _ = w.destroy();
            }
            Some(w) => {
                let _ = w.set_always_on_top(l.widget.on_top);
                let _ = w.set_title(&l.name);
            }
            None if want && (old.is_none() || !was_wanted) => {
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
