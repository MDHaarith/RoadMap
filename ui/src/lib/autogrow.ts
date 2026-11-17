// Free-text boxes grow with their text up to `max` pixels, then scroll. Code boxes keep a fixed height instead.
// Scroll positions of the areas around the box are put back after measuring, so nothing on the page moves but the box edge.
export function autogrow(node: HTMLTextAreaElement, opts: { min?: number; max?: number; value?: string } = {}) {
  let { min = 0, max = 200 } = opts;
  const resize = () => {
    // max 0 turns growing off (a box that switches to code mode).
    if (!max) { node.style.height = node.style.overflowY = ''; return; }
    const kept: [Element, number][] = [];
    for (let el = node.parentElement; el; el = el.parentElement) if (el.scrollTop) kept.push([el, el.scrollTop]);
    const top = document.scrollingElement?.scrollTop ?? 0;
    node.style.height = 'auto';
    const border = node.offsetHeight - node.clientHeight;
    node.style.height = Math.min(max, Math.max(min, node.scrollHeight + border)) + 'px';
    node.style.overflowY = node.scrollHeight + border > max ? 'auto' : 'hidden';
    for (const [el, y] of kept) el.scrollTop = y;
    if (document.scrollingElement) document.scrollingElement.scrollTop = top;
  };
  node.addEventListener('input', resize); resize();
  return {
    update(next: typeof opts) { ({ min = 0, max = 200 } = next); resize(); },
    destroy() { node.removeEventListener('input', resize); },
  };
}
