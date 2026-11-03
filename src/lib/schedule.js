// Même logique que src-tauri/src/schedule.rs, pour afficher « dans 3 j 4 h ».

export const DAYS = ['lundi', 'mardi', 'mercredi', 'jeudi', 'vendredi', 'samedi', 'dimanche'];
export const DAY_SHORT = ['L', 'M', 'M', 'J', 'V', 'S', 'D'];
export const KINDS = [
  { id: 'none', name: 'Jamais' },
  { id: 'daily', name: 'Chaque jour' },
  { id: 'weekly', name: 'Chaque semaine' },
  { id: 'monthly', name: 'Chaque mois' },
  { id: 'interval', name: 'Intervalle' }
];

function parseTime(t) {
  const [h, m] = String(t || '00:00').split(':').map((v) => Number(v) || 0);
  return [Math.min(23, h), Math.min(59, m)];
}

export function nextReset(s, after) {
  if (!s || s.kind === 'none') return null;
  if (s.kind === 'interval') return after + Math.max(1, Number(s.hours) || 1) * 3600000;
  const [hh, mm] = parseTime(s.time);
  const base = new Date(after);
  if (s.kind === 'daily') {
    for (let i = 0; i < 3; i++) {
      const d = new Date(base.getFullYear(), base.getMonth(), base.getDate() + i, hh, mm);
      if (d.getTime() > after) return d.getTime();
    }
  }
  if (s.kind === 'weekly') {
    const target = (Math.min(6, s.day || 0) + 1) % 7; // JS : 0 = dimanche
    for (let i = 0; i < 15; i++) {
      const d = new Date(base.getFullYear(), base.getMonth(), base.getDate() + i, hh, mm);
      if (d.getDay() === target && d.getTime() > after) return d.getTime();
    }
  }
  if (s.kind === 'monthly') {
    const dom = Math.max(1, Math.min(28, Number(s.dom) || 1));
    for (let i = 0; i < 3; i++) {
      const d = new Date(base.getFullYear(), base.getMonth() + i, dom, hh, mm);
      if (d.getTime() > after) return d.getTime();
    }
  }
  return null;
}

export function scheduleLabel(s) {
  if (!s || s.kind === 'none') return 'Remise à zéro manuelle';
  if (s.kind === 'daily') return `Chaque jour à ${s.time}`;
  if (s.kind === 'weekly') return `Chaque ${DAYS[s.day] ?? 'lundi'} à ${s.time}`;
  if (s.kind === 'monthly') return `Le ${Number(s.dom) === 1 ? '1er' : s.dom} de chaque mois à ${s.time}`;
  return `Toutes les ${s.hours} h`;
}

export function fmtIn(ts, now) {
  if (!ts) return 'Pas de minuterie';
  const m = Math.max(1, Math.ceil((ts - now) / 60000));
  const days = Math.floor(m / 1440);
  const h = Math.floor((m % 1440) / 60);
  const mi = m % 60;
  if (days > 0) return `dans ${days} j ${h} h`;
  if (h > 0) return `dans ${h} h ${mi} min`;
  return `dans ${mi} min`;
}

export function lowerFirst(t) {
  return t.charAt(0).toLowerCase() + t.slice(1);
}
