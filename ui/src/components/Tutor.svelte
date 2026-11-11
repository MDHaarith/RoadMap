<script lang="ts">
  import { tick } from 'svelte';
  import { appState, ask, save, keyFor } from '../lib/state.svelte';
  import { CHIPS } from '../lib/format';
  import type { Course, Lesson } from '../lib/types';
  import Icon from './Icon.svelte';
  import Formatted from './Formatted.svelte';
  let { c, l, seek }: { c: Course; l: Lesson; seek: (k: number, at: number) => void } = $props();
  let text = $state('');
  let log: HTMLDivElement;
  let key = $derived(keyFor(c, l)), msgs = $derived(appState.chats[key] || []);
  const names: Record<string, string> = { 'Give me a hint': 'Hint', 'Quiz me': 'Quiz me', 'Explain a concept': 'Explain', 'Review my work': 'Review', 'Let me teach it back': 'Teach it back' };
  $effect(() => { JSON.stringify(msgs); appState.busyKey; void tick().then(() => { if (log) log.scrollTop = log.scrollHeight; }); });
  async function send(value = text) {
    if (!value.trim() || appState.busyKey) return;
    text = '';
    if (!await ask(value)) text = value;
  }
</script>
<aside class="support" aria-label="Learning tools"><section class="panel tutor-panel"><h2><Icon name="Tutor" /> Tutor</h2>
  {#if !appState.config.base || !appState.config.model}<p class="small">No AI service connected. <a href="#/settings">Open Settings</a> to use the tutor.</p>{/if}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions — Delegates clicks from the formatter’s keyboard-accessible seek buttons. -->
  <div id="log" class="log" role="log" bind:this={log} aria-live="polite" aria-relevant="additions text" onclick={e => { const target = (e.target as Element).closest<HTMLElement>('[data-seek]'); if (target) seek(Number(target.dataset.seek), Number(target.dataset.at)); }}>
    {#each msgs as m}<div class={`msg ${m.role === 'user' ? 'u' : 'a'}`}>{#if m.role === 'user'}{m.content}{:else}<Formatted text={m.content} tutor />{/if}</div>{/each}
    {#if !msgs.length}<p class="muted">A nudge, not the answer. Bring a question, a rough idea, or an attempt you’d like to improve.</p>{/if}
    {#if appState.busyKey === key && !msgs[msgs.length - 1]?.content}<div class="msg a thinking" role="status"><Icon name="Tutor" /> Tutor is thinking…</div>{/if}
  </div>
  <div class="chips">{#each CHIPS as chip}<button type="button" class="chip" title={chip} aria-label={chip} onclick={() => send(chip)}><Icon name={chip} />{names[chip]}</button>{/each}</div>
  <form id="compose" onsubmit={e => { e.preventDefault(); void send(); }}><label class="sr" for="msg">Message the tutor</label><textarea id="msg" rows="2" placeholder="Where are you stuck?" bind:value={text} onkeydown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); void send(); } }}></textarea><button class="btn" disabled={!!appState.busyKey}>Send <Icon name="arrow" /></button></form>
</section><section class="panel notes-panel"><h2><label for="notes"><Icon name="My notes" /> My notes</label></h2><textarea id="notes" rows="5" placeholder="What worked? What would you try next?" value={c.state.notes[l.id] || ''} oninput={e => { c.state.notes[l.id] = e.currentTarget.value; save(c); }}></textarea><p class="small" style="margin:8px 0 0">Saved with this lesson.</p></section></aside>
