import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

export const COLORS = [
  { c: '#3D7BFF', n: 'Bleu' },
  { c: '#FF7A1A', n: 'Orange' },
  { c: '#2FD96B', n: 'Vert' },
  { c: '#FF3D8B', n: 'Rose' },
  { c: '#9B5CFF', n: 'Violet' },
  { c: '#22C3FF', n: 'Cyan' },
  { c: '#FFD60A', n: 'Jaune' },
  { c: '#FF5E5E', n: 'Corail' }
];

export const BGS = [
  { id: 'solid', name: 'Uni' },
  { id: 'dots', name: 'Pois' },
  { id: 'grid', name: 'Quadrillage' },
  { id: 'lines', name: 'Cahier' },
  { id: 'aurora', name: 'Bonbon' },
  { id: 'night', name: 'Néon' }
];

const label = getCurrentWebviewWindow().label;

export const store = $state({ data: null, now: Date.now(), toast: null });

// ---------- Toast avec « Annuler » ----------

let toastTimer;
/** toast('Message') ou toast('Message', () => annuler()) */
export function toast(msg, undo = null) {
  clearTimeout(toastTimer);
  store.toast = { msg, undo, key: Date.now() };
  toastTimer = setTimeout(() => (store.toast = null), undo ? 6000 : 2600);
}
export function runUndo() {
  const u = store.toast?.undo;
  store.toast = null;
  clearTimeout(toastTimer);
  if (u) u();
  return !!u;
}

// ---------- Synchronisation avec le backend ----------

export async function init() {
  store.data = await invoke('get_data');
  await listen('data-changed', (e) => {
    if (e.payload.origin !== label) store.data = e.payload.data;
  });
  await listen('list-changed', (e) => {
    if (e.payload.origin === label || !store.data) return;
    const i = store.data.lists.findIndex((l) => l.id === e.payload.list.id);
    if (i >= 0) store.data.lists[i] = e.payload.list;
    else store.data.lists.push(e.payload.list);
  });
  await listen('settings-changed', (e) => {
    if (e.payload.origin !== label && store.data) store.data.settings = e.payload.settings;
  });
  setInterval(() => (store.now = Date.now()), 20000);
}

// Envois regroupés : une rafale de clics = un seul appel au backend.
const pending = new Map();
function debounce(key, fn, ms = 250) {
  clearTimeout(pending.get(key)?.t);
  pending.set(key, { fn, t: setTimeout(() => { pending.delete(key); fn(); }, ms) });
}
/** Envoie immédiatement tout ce qui attend (avant de fermer, de tout remplacer…). */
export function flush() {
  for (const [key, p] of pending) {
    clearTimeout(p.t);
    pending.delete(key);
    p.fn();
  }
}

/** Une seule checklist a changé. */
export function commitList(list) {
  const id = list.id;
  debounce(`list:${id}`, () => {
    const l = findList(id);
    if (l) invoke('save_list', { list: $state.snapshot(l), origin: label });
  });
}
/** Les réglages ont changé. */
export function commitSettings() {
  debounce('settings', () => invoke('save_settings', { settings: $state.snapshot(store.data.settings), origin: label }));
}
/** L'ensemble a changé (ordre, suppression, annulation). */
export function commitAll() {
  for (const [key, p] of pending) {
    if (key.startsWith('list:')) {
      clearTimeout(p.t);
      pending.delete(key);
    }
  }
  debounce('all', () => invoke('save_data', { data: $state.snapshot(store.data), origin: label }), 120);
}

// ---------- Utilitaires ----------

export function uid() {
  return globalThis.crypto?.randomUUID?.() ?? `id-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export function findList(id) {
  return store.data?.lists.find((l) => l.id === id);
}

export function newList(f) {
  return {
    id: uid(),
    name: f.name.trim() || 'Ma checklist',
    color: f.color,
    bg: f.bg,
    schedule: { kind: f.kind, day: f.day, time: f.time, dom: Number(f.dom) || 1, hours: Number(f.hours) || 24 },
    items: [],
    pinned: !!f.pin,
    widget: { x: null, y: null, width: 300, onTop: false, locked: false, opacity: 1, collapsed: false },
    lastReset: Date.now(),
    completedThisPeriod: false,
    completions: 0,
    history: []
  };
}

// ---------- Actions partagées (vue, post-it, raccourcis) ----------

/** À appeler après chaque changement d'éléments : compte les checklists terminées. */
export function trackCompletion(list) {
  const all = list.items.length > 0 && list.items.every((i) => i.done);
  if (all && !list.completedThisPeriod) {
    list.completedThisPeriod = true;
    list.completions = (list.completions || 0) + 1;
  }
}

export function toggleItem(list, it) {
  it.done = !it.done;
  trackCompletion(list);
  commitList(list);
}

export function removeItem(list, it) {
  const idx = list.items.findIndex((x) => x.id === it.id);
  if (idx < 0) return;
  const snap = $state.snapshot(list.items[idx]);
  list.items.splice(idx, 1);
  commitList(list);
  toast(`« ${snap.text} » supprimé`, () => {
    const l = findList(list.id);
    if (!l) return;
    l.items.splice(Math.min(idx, l.items.length), 0, snap);
    commitList(l);
  });
}

export function moveItem(list, from, to) {
  if (from === to || to < 0 || to >= list.items.length) return;
  const [it] = list.items.splice(from, 1);
  list.items.splice(to, 0, it);
}

export function uncheckAll(list) {
  if (!list.items.some((i) => i.done)) return;
  const before = $state.snapshot(list);
  list.items.forEach((i) => (i.done = false));
  // Sans minuterie, « Tout décocher » clôt une période (pour les séries).
  if (list.schedule.kind === 'none') {
    const done = before.items.filter((i) => i.done).length;
    list.history = [...(list.history || []), { at: Date.now(), done, total: before.items.length, completed: before.completedThisPeriod }].slice(-60);
    list.completedThisPeriod = false;
  }
  commitList(list);
  toast(`« ${list.name} » a été décochée`, () => {
    const l = findList(list.id);
    if (!l) return;
    l.items = before.items;
    l.history = before.history;
    l.completedThisPeriod = before.completedThisPeriod;
    commitList(l);
  });
}

export function togglePin(list) {
  list.pinned = !list.pinned;
  commitList(list);
  toast(list.pinned ? `« ${list.name} » est posée sur le bureau` : `« ${list.name} » retirée du bureau`);
}

/** Supprime une checklist ; renvoie une fonction pour la restaurer. */
export function deleteList(list, onRestore) {
  const idx = store.data.lists.findIndex((l) => l.id === list.id);
  const snap = $state.snapshot(list);
  store.data.lists.splice(idx, 1);
  commitAll();
  toast(`« ${snap.name} » supprimée`, () => {
    store.data.lists.splice(Math.min(idx, store.data.lists.length), 0, snap);
    commitAll();
    onRestore?.(snap.id);
  });
}

// ---------- Séries ----------

export function streakInfo(list) {
  const h = list.history || [];
  let current = 0;
  for (let i = h.length - 1; i >= 0 && h[i].completed; i--) current++;
  if (list.completedThisPeriod) current++;
  let best = 0;
  let run = 0;
  for (const e of h) {
    run = e.completed ? run + 1 : 0;
    best = Math.max(best, run);
  }
  best = Math.max(best, current);
  const periods = h.length;
  const rate = periods ? Math.round((h.filter((e) => e.completed).length / periods) * 100) : null;
  return { current, best, completions: list.completions || 0, periods, rate, recent: h.slice(-14) };
}
