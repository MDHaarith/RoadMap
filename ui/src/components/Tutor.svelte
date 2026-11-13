<script lang="ts">
  import { api } from '../lib/api';
  import { models, loadModels, modelName } from '../lib/models.svelte';
  import { tick } from 'svelte';
  import { appState, ask, save, keyFor } from '../lib/state.svelte';
  import { CHIPS } from '../lib/format';
  import type { Course, Lesson } from '../lib/types';
  import Icon from './Icon.svelte';
  import Formatted from './Formatted.svelte';
  import Waiting from './Waiting.svelte';
  let { c, l, seek }: { c: Course; l: Lesson; seek: (k: number, at: number) => void } = $props();
  let text = $state(''), tab = $state<'tutor' | 'notes'>('tutor');
  let rewriting = $state(false), rewriteError = $state(''), previous = $state<string>();
  async function rewrite() {
    const before = c.state.notes[l.id] || '';
    rewriting = true; rewriteError = '';
    try {
      const result = await api<{ notes: string }>('POST', 'api/courses/' + encodeURIComponent(c.id) + '/notes', { lesson: l.id, notes: before });
      previous = before; c.state.notes[l.id] = result.notes; save(c);
    } catch (e) { rewriteError = e instanceof Error ? e.message : String(e); }
    finally { rewriting = false; }
  }
  async function chooseModel(model: string) {
    try { await api('PUT', 'api/model', { model }); appState.config.model = model; }
    catch (e) { models.error = e instanceof Error ? e.message : String(e); }
  }
  function grow(node: HTMLTextAreaElement, _value: string) {
    const resize = () => { node.style.height = 'auto'; node.style.height = Math.min(121, Math.max(43, node.scrollHeight)) + 'px'; };
    const observer = new MutationObserver(resize); observer.observe(node, { attributes: true, attributeFilter: ['value'] });
    node.addEventListener('input', resize); resize();
    return { destroy() { observer.disconnect(); node.removeEventListener('input', resize); }, update() { resize(); } };
  }
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
<aside class="support" aria-label="Learning tools">
  <div class="side-tabs" role="tablist">{#each [['tutor', 'Tutor', 'Assistant'], ['notes', 'My notes', 'Notes']] as [k, icon, name]}<button type="button" role="tab" id={`tab-${k}`} aria-controls={`pane-${k}`} aria-selected={tab === k} onclick={() => tab = k as typeof tab}><Icon name={icon} />{name}</button>{/each}</div>
  <div class="side-pane" id="pane-tutor" role="tabpanel" aria-labelledby="tab-tutor" hidden={tab !== 'tutor'}>
  {#if !appState.config.base || !appState.config.model}<p class="small">No AI service connected. <a href="#/settings">Open Settings</a> to use the tutor.</p>{/if}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions — Delegates clicks from the formatter’s keyboard-accessible seek buttons. -->
  <div id="log" class="log" role="log" bind:this={log} aria-live="polite" aria-relevant="additions text" onclick={e => { const target = (e.target as Element).closest<HTMLElement>('[data-seek]'); if (target) seek(Number(target.dataset.seek), Number(target.dataset.at)); }}>
    {#each msgs as m}{#if m.content}<div class={`msg ${m.role === 'user' ? 'u' : 'a'}`}>{#if m.role === 'user'}{m.content}{:else}<Formatted text={m.content} tutor />{/if}</div>{/if}{/each}
    {#if !msgs.length}<p class="muted">A nudge, not the answer. Bring a question, a rough idea, or an attempt you’d like to improve.</p>{/if}
    {#if appState.busyKey === key && !msgs[msgs.length - 1]?.content}<div class="msg a thinking"><Waiting inline title="Tutor is thinking. Replies take about 10–15 seconds." /></div>{/if}
  </div>
  <div class="chips">{#each CHIPS as chip}<button type="button" class="chip" title={chip} aria-label={chip} onclick={() => send(chip)}><Icon name={chip} />{names[chip]}</button>{/each}</div>
  <form id="compose" onsubmit={e => { e.preventDefault(); void send(); }}><label class="sr" for="msg">Message the tutor</label><textarea id="msg" use:grow={text} rows="2" placeholder="Where are you stuck?" bind:value={text} onkeydown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); void send(); } }}></textarea><div class="composer-actions"><select aria-label="Tutor model" title={models.error} onfocus={() => void loadModels()} disabled={models.loading} value={appState.config.model} onchange={e => void chooseModel(e.currentTarget.value)}><option value={appState.config.model}>{modelName(appState.config.model) || 'Choose model'}</option>{#each models.items.filter(m => m.id !== appState.config.model) as m}<option value={m.id}>{modelName(m.id)}</option>{/each}</select><button class="btn sm send" aria-label="Send message" disabled={!!appState.busyKey || !text.trim()}><Icon name="arrow" /></button></div></form>
</div><div class="side-pane notes-pane" id="pane-notes" role="tabpanel" aria-labelledby="tab-notes" hidden={tab !== 'notes'}><div class="row"><button class="btn secondary sm" disabled={rewriting || !c.state.notes[l.id]?.trim()} onclick={rewrite}>Rewrite with AI</button>{#if previous !== undefined}<button class="link" onclick={() => { c.state.notes[l.id] = previous!; previous = undefined; save(c); }}>Undo</button>{/if}</div>{#if rewriting}<Waiting title="Tidying your notes…" hint="Usually 10–30 seconds" />{/if}{#if rewriteError}<p class="err">{rewriteError}</p>{/if}<label class="sr" for="notes">My notes</label><textarea id="notes" readonly={rewriting} rows="3" placeholder="What worked? What would you try next?" value={c.state.notes[l.id] || ''} oninput={e => { c.state.notes[l.id] = e.currentTarget.value; save(c); }}></textarea><p class="small">Saved with this lesson.</p></div></aside>
