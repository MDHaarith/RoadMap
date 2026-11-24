import { cubicOut } from 'svelte/easing';

// Dropdowns grow out of their trigger and fold back into it when they close.
// Svelte runs this as a script animation, so the CSS reduced-motion rule cannot stop it: check here.
export function drop(node: Element) {
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const y = node.classList.contains('above') ? 6 : -6;
  return {
    duration: still ? 0 : 170,
    easing: cubicOut,
    css: (t: number) => `opacity: ${t}; transform: translateY(${(1 - t) * y}px) scale(${0.95 + t * 0.05})`,
  };
}

// A <details> slides open and shut instead of snapping. JS because WebKitGTK (Linux) has no interpolate-size.
export function slide(el: HTMLDetailsElement) {
  let anim: Animation | undefined, closing = false;
  const onClick = (e: MouseEvent) => {
    const summary = (e.target as Element).closest('summary');
    if (summary?.parentElement !== el || matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    e.preventDefault();
    const from = el.offsetHeight, close = el.open && !closing;
    anim?.cancel();
    if (!close) el.open = true;
    const cs = getComputedStyle(el);
    // Closed height: the summary plus the box edges a closed details keeps.
    const to = close ? summary.offsetHeight + parseFloat(cs.borderTopWidth) + parseFloat(cs.borderBottomWidth) + parseFloat(cs.paddingTop) : el.offsetHeight;
    closing = close;
    el.style.overflow = 'hidden';
    anim = el.animate({ height: [`${from}px`, `${to}px`] }, { duration: 260, easing: 'cubic-bezier(.2, 0, 0, 1)' });
    anim.onfinish = () => { if (closing) el.open = false; closing = false; anim = undefined; el.style.overflow = ''; };
  };
  el.addEventListener('click', onClick);
  return { destroy: () => el.removeEventListener('click', onClick) };
}
