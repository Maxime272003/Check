use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Item {
    pub id: String,
    pub text: String,
    pub done: bool,
}

/// Quand tout est décoché automatiquement.
/// `kind` : none | daily | weekly | monthly | interval
/// `day` : 0 = lundi … 6 = dimanche (weekly)
/// `time` : "HH:MM" (daily, weekly, monthly)
/// `dom` : jour du mois 1–28 (monthly)
/// `hours` : intervalle en heures (interval)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Schedule {
    pub kind: String,
    pub day: u32,
    pub time: String,
    pub dom: u32,
    pub hours: u32,
}

impl Default for Schedule {
    fn default() -> Self {
        Self { kind: "none".into(), day: 0, time: "00:00".into(), dom: 1, hours: 24 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WidgetState {
    /// Position logique sur le bureau (gérée côté Rust uniquement).
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: f64,
    pub on_top: bool,
    pub locked: bool,
    pub opacity: f64,
    pub collapsed: bool,
}

impl Default for WidgetState {
    fn default() -> Self {
        Self { x: None, y: None, width: 300.0, on_top: false, locked: false, opacity: 1.0, collapsed: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Checklist {
    pub id: String,
    pub name: String,
    pub color: String,
    pub bg: String,
    pub schedule: Schedule,
    pub items: Vec<Item>,
    pub pinned: bool,
    pub widget: WidgetState,
    /// Horodatage (ms) de la dernière remise à zéro, point de départ du calcul de la suivante.
    pub last_reset: i64,
    /// La checklist a été entièrement cochée pendant la période en cours.
    pub completed_this_period: bool,
    /// Nombre total de fois où la checklist a été entièrement cochée.
    pub completions: u32,
    /// Une entrée par période terminée (remise à zéro), la plus récente en dernier.
    pub history: Vec<HistEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HistEntry {
    pub at: i64,
    pub done: u32,
    pub total: u32,
    pub completed: bool,
}

pub const HISTORY_MAX: usize = 60;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub start_hidden: bool,
    pub close_to_tray: bool,
    pub notify_reset: bool,
    pub restore_widgets: bool,
    pub theme: String,
    pub paused: bool,
    pub last_update_check: Option<i64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            start_hidden: true,
            close_to_tray: true,
            notify_reset: true,
            restore_widgets: true,
            theme: "system".into(),
            paused: false,
            last_update_check: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppData {
    pub lists: Vec<Checklist>,
    pub settings: Settings,
}

pub fn seed(now: i64) -> AppData {
    let items = ["Découvrir Check", "Créer ma première checklist", "La poser sur le bureau"]
        .iter()
        .enumerate()
        .map(|(i, t)| Item { id: format!("seed-{i}"), text: (*t).into(), done: i == 0 })
        .collect();
    AppData {
        lists: vec![Checklist {
            id: "bienvenue".into(),
            name: "Bienvenue".into(),
            color: "#FF3D8B".into(),
            bg: "aurora".into(),
            schedule: Schedule { kind: "weekly".into(), ..Default::default() },
            items,
            pinned: false,
            widget: WidgetState::default(),
            last_reset: now,
            ..Default::default()
        }],
        settings: Settings::default(),
    }
}
