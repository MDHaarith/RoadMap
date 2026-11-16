<script lang="ts">
  import Skeleton from '../components/Skeleton.svelte';
  import { models, loadModels, modelName } from '../lib/models.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { appState, message, refreshConfig } from '../lib/state.svelte';
  import type { Settings } from '../lib/types';
  import { updates, checkUpdate, refreshUpdate } from '../lib/updates.svelte';
  import UpdateStatus from '../components/UpdateStatus.svelte';
  import Usage from '../components/Usage.svelte';
  import VideoPrefs from '../components/VideoPrefs.svelte';
  import Waiting from '../components/Waiting.svelte';
  import Icon from '../components/Icon.svelte';
  import BusyLabel from '../components/BusyLabel.svelte';
  import LoadError from '../components/LoadError.svelte';
  let config = $state<Settings>(), error = $state(''), saveError = $state(''), key = $state(''), busy = $state(false), saved = $state('');
  // What is saved now (the status line), and whether the other-service form is open.
  let active = $state({ base: '', model: '' }), other = $state(false);
  let alive = true;
  const bases: Record<string, string> = { codex: 'codex', gemini: 'https://generativelanguage.googleapis.com/v1beta/openai', openrouter: 'https://openrouter.ai/api/v1', ollama: 'http://localhost:11434/v1' };
  function load() { error = ''; api<Settings>('GET', 'api/settings').then(c => { if (!alive) return; config = c; active = { base: c.base, model: c.model }; other = c.base !== 'codex'; }).catch(e => { if (alive) error = message(e); }); }
  // The background update check starts a few seconds after launch; read the version now so About is never blank.
  onMount(() => { load(); if (!updates.version) void refreshUpdate().catch(() => {}); return () => { alive = false; }; });
  let checking = $state(false);
  async function check() { checking = true; try { await checkUpdate(true); } finally { checking = false; } }
  let rechecking = $state(false), recheckError = $state('');
  async function recheck() {
    rechecking = true; recheckError = '';
    try { const c = await refreshConfig(); if (alive && config) config.codex = c.codex; }
    catch (e) { recheckError = message(e); } finally { rechecking = false; }
  }
  async function saveSettings() {
    if (!config) return; busy = true; saved = ''; saveError = '';
    try { await api('PUT', 'api/settings', { base: config.base, model: config.model, small: config.small || '', key }); if (key) config.key_set = true; key = ''; saved = 'Settings saved.'; appState.config = { ...config }; active = { base: config.base, model: config.model }; void recheck(); }
    catch (error) { saveError = message(error); } finally { busy = false; }
  }
</script>
<div class="wrap narrow settings screen-enter"><a class="back-map" href="#/"><Icon name="back" /> Your library</a><div class="page-head"><h1>Settings</h1></div>
  {#if error}<LoadError what="Settings could not load." {error} retry={load} />{:else if !config}<Waiting skeleton="settings" inline title="Opening settings" />{:else}
  <div class="settings-rows stagger"><section class="settings-layout"><div><h2>About</h2><p>{updates.version ? `Version ${updates.version}` : 'Roadmap'}</p></div><div class="row"><button class="btn secondary sm" disabled={checking || updates.state === 'downloading'} onclick={() => void check()}><BusyLabel busy={checking} idle="Check for updates" label="Checking…" /></button><UpdateStatus /></div></section>
  <section class="settings-layout"><div><h2>AI service</h2><p>Roadmap uses it to plan lessons, prepare practice, and give you hints.</p></div><div>
    {#if active.base === 'codex' && (config.codex === 'signed_out' || config.codex === 'missing')}<div class="status-card warn"><Icon name="Tutor" /><div><strong>{config.codex === 'signed_out' ? 'Codex is signed out' : 'Codex is not installed'}</strong>{#if config.codex === 'signed_out'}<p>Open a terminal, run <code>codex login</code> and sign in with ChatGPT. Then check again.</p>{:else}<p>The Codex command was not found on this computer. Install it, run <code>codex login</code> in a terminal to sign in with ChatGPT, then check again.</p>{/if}{#if recheckError}<p class="err">{recheckError}</p>{/if}<button type="button" class="btn secondary sm" disabled={rechecking} onclick={() => void recheck()}><BusyLabel busy={rechecking} idle="Check again" label="Checking…" /></button></div></div>
    {:else if active.base === 'codex'}<div class="status-card" class:ok={config.codex === 'ready'}>{#if config.codex === 'ready'}<Icon name="check" />{:else}<Icon name="Tutor" />{/if}<div><strong>Codex with your ChatGPT sign-in</strong><p>{config.codex === 'ready' ? 'Signed in. ' : ''}Roadmap runs the Codex command on this computer and needs no key.</p></div></div>
    {:else}<div class="status-card"><Icon name="Tutor" /><div><strong>{active.base ? 'Your own AI service' : 'No AI service set'}</strong>{#if active.base}<p class="service-url">{active.base}</p><p>Model: {active.model}</p>{:else}<p>Fill in the form below, or go back to Codex.</p>{/if}</div></div>{/if}
    <details class="disclosure" bind:open={other}><summary>{active.base === 'codex' ? 'Use a different AI service' : 'Change the AI service'}</summary>
      <form id="settings" onsubmit={e => { e.preventDefault(); void saveSettings(); }}><p class="small">Any OpenAI-compatible service works. Its key stays on this computer.</p><p class="small quick-label">Quick fill</p><div class="examples">{#each [['codex', 'Codex (ChatGPT sign-in)'], ['gemini', 'Google Gemini'], ['openrouter', 'OpenRouter'], ['ollama', 'Ollama on this computer']] as [k, n]}<button type="button" onclick={() => { config!.base = bases[k]; if (k === 'codex') config!.model = 'default'; }}>{n}</button>{/each}</div><label for="base">Base URL</label><input type="text" id="base" required bind:value={config.base} placeholder="Your service’s API base URL, or codex" /><label for="model">Model name</label><input id="model" list="service-models" onfocus={() => void loadModels()} required bind:value={config.model} placeholder="With Codex, default means its own model" /><datalist id="service-models">{#each models.items as m}<option value={m.id}>{modelName(m.id)}</option>{/each}</datalist>{#if models.loading && !models.error}<Skeleton kind="models" />{/if}{#if models.error}<p class="small">{models.error}</p>{/if}<label for="small">Small model (optional)</label><input id="small" bind:value={config.small} placeholder="Leave empty to use the model above" /><p class="small field-note">A cheaper, faster model for quizzes, video picks, marking and placement questions. Roadmaps and the tutor keep the main model.</p><label for="key">API key</label><input id="key" type="password" autocomplete="off" bind:value={key} placeholder={config.key_set ? 'Saved. Leave empty to keep it.' : 'Not needed for Codex or a local Ollama model'} /><p class="save row"><button class="btn" disabled={busy}><BusyLabel {busy} idle="Save settings" label="Saving…" /></button><span id="saved" role="status">{saved}</span></p>{#if saveError}<p class="err" role="alert">The settings were not saved. {saveError}</p>{/if}</form>
    </details>
  </div></section>
  <section class="settings-layout" id="video-prefs"><div><h2>Videos</h2><p>Default preferences for video picks.</p></div><VideoPrefs initial={config.videos} /></section>
  <section class="settings-layout"><div><h2>Usage</h2><p>What this app has sent to and received from your AI service.</p></div><Usage usage={config.usage} /></section></div>{/if}
</div>
