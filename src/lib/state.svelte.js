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

let toastTimer;
export function toast(msg) {
  clearTimeout(toastTimer);
  store.toast = msg;
  toastTimer = setTimeout(() => (store.toast = null), 2600);
}

export async function init() {
  store.data = await invoke('get_data');
  await listen('data-changed', (e) => {
    if (e.payload.origin !== label) store.data = e.payload.data;
  });
  setInterval(() => (store.now = Date.now()), 20000);
}

/** Envoie l'état complet au backend, qui sauvegarde et prévient les autres fenêtres. */
export function commit() {
  return invoke('save_data', { data: $state.snapshot(store.data), origin: label });
}

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
    lastReset: Date.now()
  };
}
