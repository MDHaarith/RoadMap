// Browser self-check: open Settings in Vite, then import('/tests/controls.js').then(m => m.checkSelect()).
// It restores the original choice and needs no test framework or API writes.
export async function checkSelect() {
  const pause = () => new Promise(resolve => setTimeout(resolve, 0));
  const assert = (value, message) => { if (!value) throw new Error(message); };
  const trigger = document.querySelector('#video-prefs .select-trigger');
  assert(trigger, 'Open Settings before running the Select check');
  const key = async value => {
    document.activeElement.dispatchEvent(new KeyboardEvent('keydown', { key: value, bubbles: true, cancelable: true }));
    await pause();
  };
  const list = () => document.querySelector('[role=listbox]');
  const original = trigger.textContent.trim();
  trigger.focus(); await key('ArrowDown');
  assert(trigger.getAttribute('aria-expanded') === 'true' && document.activeElement === list(), 'ArrowDown opens and focuses the list');
  assert(list().querySelector('[aria-selected=true] .check-mark'), 'The current option has a check');
  await key('End');
  assert(list().getAttribute('aria-activedescendant') === list().lastElementChild.id, 'End reaches the last option');
  await key('Escape');
  assert(!list() && document.activeElement === trigger && trigger.textContent.trim() === original, 'Escape cancels and restores focus');
  for (const opener of ['Enter', ' ', 'ArrowUp']) {
    await key(opener); assert(list(), `${opener} opens`); await key('Escape');
  }
  await key('Enter'); await key('Home'); await key('ArrowDown');
  const expected = list().querySelectorAll('[role=option]')[1].textContent.trim();
  await key('Enter');
  assert(!list() && trigger.textContent.trim() === expected && document.activeElement === trigger, 'Enter commits the keyboard choice');
  await key('Enter'); await key('h');
  assert(document.getElementById(list().getAttribute('aria-activedescendant')).textContent.trim() === 'Hidden gems', 'Typeahead finds the option');
  await key('Escape'); await key('Enter'); await key('Tab');
  assert(!list() && document.activeElement === trigger, 'Tab closes before native focus traversal');
  await key('Enter');
  document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })); await pause();
  assert(!list(), 'Outside pointer closes the list');
  trigger.focus(); await key('Enter');
  const restore = [...list().querySelectorAll('[role=option]')].find(o => o.textContent.trim() === original);
  restore.click(); await pause();
  assert(trigger.textContent.trim() === original, 'Pointer selection restores the initial value');
  return 'Select: open keys, arrows, Home/End, checkmark, typeahead, commit, cancel, Tab and outside dismissal passed';
}
