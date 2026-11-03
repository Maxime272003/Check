/**
 * Glisser-déposer vertical à la souris, sans dépendance.
 * Le déplacement ne démarre qu'après 5 px, pour qu'un simple clic reste un clic.
 *
 * @param {PointerEvent} e
 * @param {{ container: HTMLElement, from: number, move: (from: number, to: number) => void, end: (moved: boolean) => void, onstart?: () => void }} o
 *   container : parent dont les enfants `[data-reorder]` sont les éléments, dans l'ordre.
 */
export function dragReorder(e, { container, from, move, end, onstart }) {
  if (e.button !== 0) return;
  const sx = e.clientX;
  const sy = e.clientY;
  let started = false;
  let cur = from;

  const onMove = (ev) => {
    if (!started) {
      if (Math.abs(ev.clientY - sy) < 5 && Math.abs(ev.clientX - sx) < 5) return;
      started = true;
      document.documentElement.classList.add('reordering');
      onstart?.();
    }
    const els = [...container.querySelectorAll(':scope > [data-reorder]')];
    for (let i = 0; i < els.length; i++) {
      if (i === cur) continue;
      const r = els[i].getBoundingClientRect();
      const mid = r.top + r.height / 2;
      if ((i > cur && ev.clientY > mid && ev.clientY < r.bottom + 4) || (i < cur && ev.clientY < mid && ev.clientY > r.top - 4)) {
        move(cur, i);
        cur = i;
        break;
      }
    }
  };
  const onUp = () => {
    window.removeEventListener('pointermove', onMove);
    window.removeEventListener('pointerup', onUp);
    window.removeEventListener('pointercancel', onUp);
    document.documentElement.classList.remove('reordering');
    if (!started) return;
    // Empêche le clic qui suit le relâchement (sinon l'élément serait coché).
    const block = (ev) => {
      ev.stopPropagation();
      ev.preventDefault();
    };
    window.addEventListener('click', block, { capture: true, once: true });
    setTimeout(() => window.removeEventListener('click', block, { capture: true }), 50);
    end(cur !== from);
  };
  window.addEventListener('pointermove', onMove);
  window.addEventListener('pointerup', onUp);
  window.addEventListener('pointercancel', onUp);
}
