<script lang="ts">
  import Skeleton from './Skeleton.svelte';
  import { api } from '../lib/api';
  import { models, loadModels, modelName } from '../lib/models.svelte';
  import { tick, untrack } from 'svelte';
  import { appState, ask, save, keyFor, stopAsk, clearChat } from '../lib/state.svelte';
  import { CHIPS } from '../lib/format';
  import type { Course, Lesson } from '../lib/types';
  import Icon from './Icon.svelte';
  import Formatted from './Formatted.svelte';
  import Waiting from './Waiting.svelte';
  import Elapsed from './Elapsed.svelte';
  import Select from './Select.svelte';
  import { confirmAsk } from './Confirm.svelte';
  let { c, l, seek }: { c: Course; l: Lesson; seek: (k: number, at: number) => void } = $props();
  let text = $state(''), tab = $state<'tutor' | 'notes'>('tutor');
  let rewriting = $state(false), rewriteError = $state(''), previous = $state<string>();
  let clearing = $state(false), atBottom = $state(true);
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
  let busy = $derived(appState.busyKey === key);
  const names: Record<string, string> = { 'Give me a hint': 'Hint', 'Quiz me': 'Quiz me', 'Explain a concept': 'Explain', 'Review my work': 'Review', 'Let me teach it back': 'Teach it back' };
  const descriptions: Record<string, string> = { 'Give me a hint': 'Get a hint for your next step.', 'Quiz me': 'Answer a question about this lesson.', 'Explain a concept': 'Read an idea explained in simple words.', 'Review my work': 'Share your work and get feedback.', 'Let me teach it back': 'Explain an idea and check what you missed.' };
  function newest() { if (log) log.scrollTop = log.scrollHeight; atBottom = true; }
  let lastKey = '';
  $effect(() => {
    JSON.stringify(msgs); busy; appState.tutorStatus; tab;
    const follow = untrack(() => atBottom) || key !== lastKey;
    lastKey = key;
    void tick().then(() => { if (follow) newest(); });
  });
  async function newChat() {
    if (clearing || appState.busyKey) return;
    if (!(await confirmAsk({ message: 'Start a new chat? This deletes this lesson’s conversation and cannot be undone.', action: 'Start new chat', danger: true }))) return;
    clearing = true;
    try { await clearChat(c, l); } finally { clearing = false; }
  }
  async function send(value = text) {
    if (!value.trim() || appState.busyKey || clearing) return;
    text = '';
    if (!await ask(value) && !text.trim()) text = value;
  }
</script>
<aside class="support" aria-label="Learning tools">
  <div class="side-tabs" role="tablist">{#each [['tutor', 'Tutor', 'Tutor'], ['notes', 'My notes', 'Notes']] as [k, icon, name]}<button type="button" role="tab" id={`tab-${k}`} aria-controls={`pane-${k}`} aria-selected={tab === k} onclick={() => tab = k as typeof tab}><Icon name={icon} />{name}</button>{/each}</div>
  <div class="side-pane" id="pane-tutor" role="tabpanel" aria-labelledby="tab-tutor" hidden={tab !== 'tutor'}>
  {#if msgs.length && !appState.busyKey}<div class="tutor-toolbar"><button type="button" class="btn ghost sm" disabled={clearing} onclick={newChat}>New chat</button></div>{/if}
  {#if !appState.config.base || !appState.config.model}<p class="small">No AI service connected. <a href="#/settings">Open Settings</a> to use the tutor.</p>{/if}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions — Delegates clicks from the formatter’s keyboard-accessible seek buttons. -->
  <div class="chat-log">
  <div id="log" class="log" role="log" bind:this={log} aria-live="polite" aria-relevant="additions text" onscroll={() => atBottom = log.scrollHeight - log.scrollTop - log.clientHeight < 32} onclick={e => { const target = (e.target as Element).closest<HTMLElement>('[data-seek]'); if (target) seek(Number(target.dataset.seek), Number(target.dataset.at)); }}>
    {#each msgs as m}{#if m.content}<div class={`msg message-enter ${m.role === 'user' ? 'u' : 'a'}`}>{#if m.role === 'user'}{m.content}{:else}<Formatted text={m.content} tutor />{/if}</div>{/if}{/each}
    {#if !msgs.length}<div class="tutor-empty"><p class="muted">Ask a question or share what you have tried. The tutor can help you find your next step.</p><div class="ruled tutor-starters">{#each CHIPS as chip}<button type="button" disabled={!!appState.busyKey || clearing} aria-label={chip} onclick={() => send(chip)}><strong>{names[chip]}</strong><span>{descriptions[chip]}</span></button>{/each}</div></div>{/if}
    {#if busy}<div class="msg a thinking"><span role="status" aria-live="polite">{appState.tutorStatus || 'Preparing a reply'}</span><span aria-hidden="true"><Elapsed /></span><Skeleton rows={3} /></div>{/if}
  </div>
  {#if !atBottom && msgs.length}<button type="button" class="btn secondary sm newest" onclick={newest}>Latest reply <span aria-hidden="true">↓</span></button>{/if}
  </div>
  {#if msgs.length}<div class="chips tutor-actions">{#each CHIPS as chip}<button type="button" class="chip" title={chip} aria-label={chip} disabled={!!appState.busyKey || clearing} onclick={() => send(chip)}>{names[chip]}</button>{/each}</div>{/if}
  <div class="composer-wrap">
  <form id="compose" onsubmit={e => { e.preventDefault(); void send(); }}><label class="sr" for="msg">Message the tutor</label><textarea id="msg" aria-describedby="compose-hint" use:grow={text} rows="2" placeholder="Ask about this lesson…" bind:value={text} onkeydown={e => { if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); void send(); } }}></textarea><div class="composer-actions"><Select ariaLabel="Tutor model" loading={models.loading && !models.error} title={models.error || (models.loading ? 'Loading models…' : '')} onopen={() => void loadModels()} value={appState.config.model} onchange={model => void chooseModel(model)} options={[{value: appState.config.model, label: modelName(appState.config.model) || 'Choose model'}, ...models.items.filter(m => m.id !== appState.config.model).map(m => ({value: m.id, label: modelName(m.id)}))]} />{#if busy}<button type="button" class="btn secondary sm send" aria-label="Stop reply" onclick={stopAsk}><Icon name="stop" /></button>{:else}<button class="btn sm send" aria-label="Send message" disabled={!!appState.busyKey || clearing || !text.trim()}><Icon name="arrow" /></button>{/if}</div></form>
  <p class="sr" id="compose-hint">Enter to send, Shift+Enter for a new line</p>
  </div>
</div><div class="side-pane notes-pane" id="pane-notes" role="tabpanel" aria-labelledby="tab-notes" hidden={tab !== 'notes'}><div class="row"><button class="btn secondary sm" disabled={rewriting || !c.state.notes[l.id]?.trim()} onclick={rewrite}>Rewrite my notes</button>{#if previous !== undefined}<button class="link" onclick={() => { c.state.notes[l.id] = previous!; previous = undefined; save(c); }}>Undo rewrite</button>{/if}</div>{#if rewriting}<Waiting skeleton="notes" title="Rewriting your notes" hint="usually 10–30 seconds" />{/if}{#if rewriteError}<p class="err">{rewriteError}</p>{/if}<label class="sr" for="notes">My notes</label><textarea id="notes" readonly={rewriting} rows="3" placeholder="Write what you learned or want to remember." value={c.state.notes[l.id] || ''} oninput={e => { c.state.notes[l.id] = e.currentTarget.value; save(c); }}></textarea><p class="small">Your notes are saved with this lesson.</p></div></aside>
