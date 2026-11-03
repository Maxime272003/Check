//! Stockage : data.json (écriture atomique, regroupée par le ticker) + sauvegardes tournantes.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::model::{self, AppData, Checklist};
use crate::now_ms;

const BACKUPS_KEPT: usize = 10;

pub struct AppState {
    pub data: Mutex<AppData>,
    pub path: PathBuf,
    pub dirty: AtomicBool,
}

fn parse(path: &PathBuf) -> Option<AppData> {
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok())
}

impl AppState {
    /// Charge data.json ; s'il est illisible, repart de la sauvegarde la plus récente.
    pub fn load(dir: PathBuf) -> std::io::Result<(Self, bool)> {
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("data.json");
        let first_run = !path.exists();
        let data = if first_run {
            model::seed(now_ms())
        } else if let Some(d) = parse(&path) {
            d
        } else {
            eprintln!("data.json illisible : copie en data.json.bak, restauration de la dernière sauvegarde");
            let _ = std::fs::copy(&path, path.with_extension("json.bak"));
            let state = Self { data: Mutex::new(AppData::default()), path: path.clone(), dirty: AtomicBool::new(false) };
            state
                .backups()
                .iter()
                .rev()
                .find_map(parse)
                .unwrap_or_else(|| model::seed(now_ms()))
        };
        Ok((Self { data: Mutex::new(data), path, dirty: AtomicBool::new(true) }, first_run))
    }

    pub fn snapshot(&self) -> AppData {
        self.data.lock().unwrap().clone()
    }

    pub fn save_now(&self) {
        self.dirty.store(false, Ordering::SeqCst);
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
    }

    pub fn backup_dir(&self) -> PathBuf {
        self.path.parent().map(|p| p.join("backups")).unwrap_or_else(|| PathBuf::from("backups"))
    }

    fn backups(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(self.backup_dir())
            .map(|rd| {
                rd.filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("data-") && n.ends_with(".json")))
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }

    /// Copie data.json dans backups/ et ne garde que les 10 plus récentes.
    pub fn backup(&self) {
        if self.dirty.load(Ordering::SeqCst) {
            self.save_now();
        }
        if !self.path.exists() {
            return;
        }
        let dir = self.backup_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let name = format!("data-{}.json", chrono::Local::now().format("%Y-%m-%d_%H%M%S"));
        let _ = std::fs::copy(&self.path, dir.join(name));
        let all = self.backups();
        if all.len() > BACKUPS_KEPT {
            for p in &all[..all.len() - BACKUPS_KEPT] {
                let _ = std::fs::remove_file(p);
            }
        }
    }

    /// Remplace tout l'état en conservant les positions des post-it (gérées côté Rust).
    pub fn replace(&self, mut data: AppData) -> (AppData, AppData) {
        let mut cur = self.data.lock().unwrap();
        let now = now_ms();
        for l in data.lists.iter_mut() {
            if let Some(o) = cur.lists.iter().find(|o| o.id == l.id) {
                l.widget.x = o.widget.x;
                l.widget.y = o.widget.y;
            }
            if l.last_reset == 0 {
                l.last_reset = now;
            }
        }
        let old = std::mem::replace(&mut *cur, data.clone());
        self.dirty.store(true, Ordering::SeqCst);
        (old, data)
    }

    /// Met à jour (ou ajoute) une seule checklist.
    pub fn upsert_list(&self, mut list: Checklist) -> (AppData, AppData, Checklist) {
        let mut cur = self.data.lock().unwrap();
        let old = cur.clone();
        if list.last_reset == 0 {
            list.last_reset = now_ms();
        }
        match cur.lists.iter_mut().find(|l| l.id == list.id) {
            Some(l) => {
                list.widget.x = l.widget.x;
                list.widget.y = l.widget.y;
                *l = list.clone();
            }
            None => cur.lists.push(list.clone()),
        }
        self.dirty.store(true, Ordering::SeqCst);
        (old, cur.clone(), list)
    }

    /// Ajoute des checklists importées (nouvel identifiant si déjà présent, non posées sur le bureau).
    pub fn import(&self, lists: Vec<Checklist>) -> usize {
        let mut cur = self.data.lock().unwrap();
        let now = now_ms();
        let n = lists.len();
        for (i, mut l) in lists.into_iter().enumerate() {
            if l.id.is_empty() || cur.lists.iter().any(|o| o.id == l.id) {
                l.id = format!("import-{now}-{i}");
            }
            l.pinned = false;
            l.widget.x = None;
            l.widget.y = None;
            if l.last_reset == 0 {
                l.last_reset = now;
            }
            cur.lists.push(l);
        }
        self.dirty.store(true, Ordering::SeqCst);
        n
    }
}
