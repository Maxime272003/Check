/**
 * Fondu doux en haut / en bas d'une zone qui défile : les éléments s'estompent au bord
 * au lieu d'être coupés net. Le fondu n'apparaît que du côté où il reste du contenu.
 */
export function scrollFade(node) {
  node.classList.add('scroll-fade');
  let frame = 0;
  const update = () => {
    frame = 0;
    const top = node.scrollTop > 2;
    const bottom = node.scrollTop + node.clientHeight < node.scrollHeight - 2;
    node.classList.toggle('fade-top', top);
    node.classList.toggle('fade-bottom', bottom);
  };
  const schedule = () => {
    if (!frame) frame = requestAnimationFrame(update);
  };
  node.addEventListener('scroll', schedule, { passive: true });
  const ro = new ResizeObserver(schedule);
  ro.observe(node);
  const mo = new MutationObserver(schedule);
  mo.observe(node, { childList: true, subtree: true });
  update();
  return {
    destroy() {
      cancelAnimationFrame(frame);
      node.removeEventListener('scroll', schedule);
      ro.disconnect();
      mo.disconnect();
    }
  };
}
