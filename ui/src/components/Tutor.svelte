<script lang="ts">
  import Skeleton from './Skeleton.svelte';
  import { api } from '../lib/api';
  import ModelPicker from './ModelPicker.svelte';
  import EffortPicker from './EffortPicker.svelte';
  import StreamText from './StreamText.svelte';
  import { tick, untrack } from 'svelte';
  import { appState, ask, retryAsk, save, keyFor, stopAsk, clearChat, defaultEffort } from '../lib/state.svelte';
  import { CHIPS } from '../lib/format';
  import type { Course, Lesson } from '../lib/types';
  import Icon from './Icon.svelte';
  import Waiting from './Waiting.svelte';
  import Elapsed from './Elapsed.svelte';
  import AiSetup from './AiSetup.svelte';
  import { autogrow } from '../lib/autogrow';
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
  let log: HTMLDivElement;
  let key = $derived(keyFor(c, l)), msgs = $derived(appState.chats[key] || []);
  let busy = $derived(appState.busyKey === key), failed = $derived(appState.tutorErrors[key]);
  const names: Record<string, string> = { 'Give me a hint': 'Hint', 'Quiz me': 'Quiz me', 'Explain a concept': 'Explain', 'Review my work': 'Review', 'Let me teach it back': 'Teach it back' };
  const descriptions: Record<string, string> = { 'Give me a hint': 'Get a hint for your next step.', 'Quiz me': 'Answer a question about this lesson.', 'Explain a concept': 'Read an idea explained in simple words.', 'Review my work': 'Share your work and get feedback.', 'Let me teach it back': 'Explain an idea and check what you missed.' };
  const FOLLOWUPS = [['Quiz me on this', 'Quiz me on this: ask me one question at a time about what we just discussed.'], ['Simpler', 'Explain that again in simpler words.'], ['An example', 'Show me one concrete example of that.']];
  // Scroll events from our own scrolling arrive a frame later, after more of the reply may have rendered; they must not unpin the log.
  let autoAt = 0;
  function newest() { if (log) { autoAt = performance.now(); log.scrollTop = log.scrollHeight; } atBottom = true; }
  function scrolled() { const near = log.scrollHeight - log.scrollTop - log.clientHeight < 32; if (near || performance.now() - autoAt > 200) atBottom = near; }
  let lastKey = '';
  $effect(() => {
    JSON.stringify(msgs); busy; failed; appState.tutorStatus; tab;
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
  <AiSetup compact why="The tutor answers through an AI service. Add one in Settings." />
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions — Delegates clicks from the formatter’s keyboard-accessible seek buttons. -->
  <div class="chat-log">
  <div id="log" class="log" role="log" bind:this={log} aria-live="polite" aria-relevant="additions text" onscroll={scrolled} ontogglecapture={() => { if (atBottom) void tick().then(newest); }} onclick={e => { const target = (e.target as Element).closest<HTMLElement>('[data-seek]'); if (target) seek(Number(target.dataset.seek), Number(target.dataset.at)); }}>
    {#each msgs as m, i}{#if m.content}<div class={`msg message-enter ${m.role === 'user' ? 'u' : 'a'}`}>{#if m.role === 'user'}{m.content}{:else}{#if m.steps?.length === 1}<p class="tutor-steps">{m.steps[0]}</p>{:else if m.steps?.length}<details class="tutor-steps"><summary>Checked {m.steps.length} things</summary><ul>{#each m.steps as step}<li>{step}</li>{/each}</ul></details>{/if}<StreamText text={m.content} live={busy && i === msgs.length - 1} />{/if}</div>
      {#if i === msgs.length - 1 && m.role === 'assistant' && !busy && !/\(The answer was cut off\.\)$/.test(m.content) && m.content !== '(No answer.)'}<div class="tutor-followups" role="group" aria-label="Follow up on this reply">{#each FOLLOWUPS as [name, ask]}<button type="button" disabled={!!appState.busyKey || clearing} onclick={() => send(ask)}>{name}</button>{/each}</div>{/if}{/if}{/each}
    {#if !msgs.length}<div class="tutor-empty"><p class="muted">Ask a question or share what you have tried. The tutor can help you find your next step.</p><div class="ruled tutor-starters">{#each CHIPS as chip}<button type="button" disabled={!!appState.busyKey || clearing} aria-label={chip} onclick={() => send(chip)}><strong>{names[chip]}</strong><span>{descriptions[chip]}</span></button>{/each}</div></div>{/if}
    {#if failed && !busy}<div class="err tutor-error message-enter" role="alert"><p><strong>The tutor could not answer.</strong> {failed.error}</p><button type="button" class="btn secondary sm" disabled={!!appState.busyKey || clearing} onclick={() => void retryAsk()}>Try again</button></div>{/if}
    {#if busy}<div class="msg a thinking"><span role="status" aria-live="polite">{#key appState.tutorStatus}<span class="fade-in">{appState.tutorStatus || 'Preparing a reply'}</span>{/key}</span><span aria-hidden="true"><Elapsed /></span>{#if !msgs[msgs.length - 1]?.content}<Skeleton rows={3} />{/if}</div>{/if}
  </div>
  {#if !atBottom && msgs.length}<button type="button" class="btn secondary sm newest" onclick={newest}>Latest reply <span aria-hidden="true">↓</span></button>{/if}
  </div>
  {#if msgs.length}<div class="chips tutor-actions">{#each CHIPS as chip}<button type="button" class="chip" title={chip} aria-label={chip} disabled={!!appState.busyKey || clearing} onclick={() => send(chip)}>{names[chip]}</button>{/each}</div>{/if}
  <div class="composer-wrap">
  <form id="compose" onsubmit={e => { e.preventDefault(); void send(); }}><label class="sr" for="msg">Message the tutor</label><textarea id="msg" aria-describedby="compose-hint" use:autogrow={{ min: 43, max: 121, value: text }} rows="2" placeholder="Ask about this lesson…" bind:value={text} onkeydown={e => { if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); void send(); } }}></textarea><div class="composer-actions"><span class="pickers"><ModelPicker />{#if appState.config.base === 'codex'}<EffortPicker bind:value={() => appState.tutorEffort || defaultEffort(), v => appState.tutorEffort = v} />{/if}</span>{#if busy}<button type="button" class="btn secondary sm send" aria-label="Stop reply" onclick={stopAsk}><Icon name="stop" /></button>{:else}<button class="btn sm send" aria-label="Send message" disabled={!!appState.busyKey || clearing || !text.trim()}><Icon name="arrow" /></button>{/if}</div></form>
  <p class="sr" id="compose-hint">Enter to send, Shift+Enter for a new line</p>
  </div>
</div><div class="side-pane notes-pane" id="pane-notes" role="tabpanel" aria-labelledby="tab-notes" hidden={tab !== 'notes'}><div class="row"><button class="btn secondary sm" disabled={rewriting || !c.state.notes[l.id]?.trim()} onclick={rewrite}>Rewrite my notes</button>{#if previous !== undefined}<button class="link" onclick={() => { c.state.notes[l.id] = previous!; previous = undefined; save(c); }}>Undo rewrite</button>{/if}</div>{#if rewriting}<Waiting skeleton="notes" title="Rewriting your notes" hint="usually 10–30 seconds" />{/if}{#if rewriteError}<p class="err">{rewriteError}</p>{/if}<label class="sr" for="notes">My notes</label><textarea id="notes" readonly={rewriting} rows="3" placeholder="Write what you learned or want to remember." value={c.state.notes[l.id] || ''} oninput={e => { c.state.notes[l.id] = e.currentTarget.value; save(c); }}></textarea><p class="small">Your notes are saved with this lesson.</p></div></aside>
