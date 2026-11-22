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
