// Scroll bars are hidden, so a scroll area with nothing focusable inside could not be scrolled from the keyboard.
// Such areas, while they overflow, become one tab stop with a label; arrow keys, Page Up/Down, Home and End then scroll them.
// Areas that hold links, buttons or fields are left alone: tabbing through those already scrolls the area.
const AREAS: [string, string][] = [
  ['.step-body', 'Lesson step'],
  ['.log', 'Conversation with the tutor'],
  ['.overview-side', 'Roadmap summary'],
  ['.overview-main', 'Lessons'],
  ['.video-choices', 'Video choices'],
  ['.library-column', 'Your library'],
  ['.ideas', 'Ideas'],
  ['.code-viewer', 'Source code'],
  ['.tutor-table', 'Table'],
  ['pre', 'Code'],
];
const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), textarea, select, summary, [tabindex]:not([tabindex="-1"]):not([data-scroll-region])';

export function needsTabStop(el: Element) {
  const overflows = el.scrollHeight > el.clientHeight + 1 || el.scrollWidth > el.clientWidth + 1;
  return overflows && !el.querySelector(FOCUSABLE);
}

function mark() {
  for (const [selector, label] of AREAS) for (const el of document.querySelectorAll<HTMLElement>(selector)) {
    const ours = el.hasAttribute('data-scroll-region');
    if (!ours && el.hasAttribute('tabindex')) continue;
    if (needsTabStop(el)) {
      if (ours) continue;
      el.setAttribute('data-scroll-region', ''); el.tabIndex = 0;
      if (!el.hasAttribute('aria-label') && !el.hasAttribute('aria-labelledby')) { el.setAttribute('aria-label', label); el.setAttribute('data-scroll-label', ''); }
      if (!el.hasAttribute('role')) { el.setAttribute('role', 'region'); el.setAttribute('data-scroll-role', ''); }
    } else if (ours && document.activeElement !== el) {
      el.removeAttribute('data-scroll-region'); el.removeAttribute('tabindex');
      if (el.hasAttribute('data-scroll-label')) { el.removeAttribute('aria-label'); el.removeAttribute('data-scroll-label'); }
      if (el.hasAttribute('data-scroll-role')) { el.removeAttribute('role'); el.removeAttribute('data-scroll-role'); }
    }
  }
}

/** Keeps every scroll area reachable by keyboard while the app runs. Returns a stop function. */
export function watchScrollRegions() {
  let frame = 0;
  const later = () => { if (!frame) frame = requestAnimationFrame(() => { frame = 0; mark(); }); };
  const observer = new MutationObserver(later);
  observer.observe(document.body, { childList: true, subtree: true, characterData: true });
  addEventListener('resize', later);
  later();
  return () => { observer.disconnect(); removeEventListener('resize', later); cancelAnimationFrame(frame); };
}
