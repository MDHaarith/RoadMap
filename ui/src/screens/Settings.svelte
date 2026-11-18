<script lang="ts">
  import Skeleton from '../components/Skeleton.svelte';
  import { models, loadModels, modelName } from '../lib/models.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { appState, message, refreshConfig, signInChatGpt, EFFORTS } from '../lib/state.svelte';
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
  const bases: Record<string, string> = { codex: 'codex', opencodego: 'https://opencode.ai/zen/go/v1', opencode: 'https://opencode.ai/zen/v1', gemini: 'https://generativelanguage.googleapis.com/v1beta/openai', openrouter: 'https://openrouter.ai/api/v1', ollama: 'http://localhost:11434/v1' };
  const fills = [['codex', 'ChatGPT (sign in)'], ['opencodego', 'OpenCode Go'], ['opencode', 'OpenCode Zen'], ['gemini', 'Google Gemini'], ['openrouter', 'OpenRouter'], ['ollama', 'Ollama on this computer']];
  // The models of the service in the form (saved or not): fetched from it, each with Set as default and Remove.
  type Found = { id: string; name: string; note?: string };
  let found = $state<{ base: string; models: Found[]; removed: Found[] }>(), fetching = $state(false), fetchError = $state(''), filter = $state('');
  let shown = $derived((found?.models || []).filter(m => !filter.trim() || (m.name + ' ' + m.id).toLowerCase().includes(filter.trim().toLowerCase())));
  async function fetchModels() {
    if (!config?.base.trim()) { fetchError = 'Fill in the Base URL first.'; return; }
    const base = config.base.trim().replace(/\/+$/, '');
    fetching = true; fetchError = '';
    try { const r = await api<{ models: Found[]; removed: Found[] }>('POST', 'api/models/fetch', { base, key }); if (alive) { found = { base, ...r }; filter = ''; } }
    catch (e) { if (alive) { fetchError = message(e); found = undefined; } } finally { if (alive) fetching = false; }
  }
  const isDefault = (id: string) => !!found && found.base === active.base.replace(/\/+$/, '') && id === active.model;
  async function setDefault(id: string) { if (!config) return; config.model = id; await saveSettings(); }
  async function hide(m: Found, hidden: boolean) {
    if (!found) return; const base = found.base;
    try {
      await api('PUT', 'api/models/hidden', { base, model: m.id, hidden });
      if (found?.base !== base) return;
      found.models = hidden ? found.models.filter(x => x.id !== m.id) : [...found.models, m].sort((a, b) => a.name.localeCompare(b.name));
      found.removed = hidden ? [...found.removed, m] : found.removed.filter(x => x.id !== m.id);
      void loadModels(true);
    } catch (e) { fetchError = message(e); }
  }
  function load() { error = ''; api<Settings>('GET', 'api/settings').then(c => { if (!alive) return; config = c; active = { base: c.base, model: c.model }; other = c.base !== 'codex'; void fetchModels(); }).catch(e => { if (alive) error = message(e); }); }
  // The background update check starts a few seconds after launch; read the version now so About is never blank.
  onMount(() => { load(); if (!updates.version) void refreshUpdate().catch(() => {}); return () => { alive = false; }; });
  let checking = $state(false);
  async function check() { checking = true; try { await checkUpdate(true); } finally { checking = false; } }
  let rechecking = $state(false), recheckError = $state('');
  async function recheck() {
    rechecking = true; recheckError = '';
    try { const c = await refreshConfig(); if (alive && config) { config.codex = c.codex; config.chatgpt_email = c.chatgpt_email; } }
    catch (e) { recheckError = message(e); } finally { rechecking = false; }
  }
  async function signIn() {
    recheckError = '';
    try { await signInChatGpt(); await recheck(); if (alive && active.base === 'codex') void fetchModels(); }
    catch (e) { if (alive) recheckError = message(e); }
  }
  async function signOut() {
    rechecking = true; recheckError = '';
    try { await api('POST', 'api/chatgpt/logout'); } catch (e) { recheckError = message(e); }
    rechecking = false; await recheck(); found = undefined;
  }
  let effortError = $state('');
  async function setEffort(effort: string) {
    effortError = '';
    try { await api('PUT', 'api/effort', { effort }); appState.config.effort = effort; if (config) config.effort = effort; }
    catch (e) { effortError = message(e); }
  }
  async function saveSettings() {
    if (!config) return; busy = true; saved = ''; saveError = '';
    try { await api('PUT', 'api/settings', { base: config.base, model: config.model, small: config.small || '', key }); if (key) config.key_set = true; key = ''; saved = 'Settings saved.'; appState.config = { ...config }; active = { base: config.base, model: config.model }; void recheck(); void loadModels(true); }
    catch (error) { saveError = message(error); } finally { busy = false; }
  }
</script>
<div class="wrap narrow settings screen-enter"><a class="back-map" href="#/"><Icon name="back" /> Your library</a><div class="page-head"><h1>Settings</h1></div>
  {#if error}<LoadError what="Settings could not load." {error} retry={load} />{:else if !config}<Waiting skeleton="settings" inline title="Opening settings" />{:else}
  <div class="settings-rows stagger"><section class="settings-layout"><div><h2>About</h2><p>{updates.version ? `Version ${updates.version}` : 'Roadmap'}</p></div><div class="row"><button class="btn secondary sm" disabled={checking || updates.state === 'downloading'} onclick={() => void check()}><BusyLabel busy={checking} idle="Check for updates" label="Checking…" /></button><UpdateStatus /></div></section>
  <section class="settings-layout"><div><h2>AI service</h2><p>Roadmap uses it to plan lessons, prepare practice, and give you hints.</p></div><div>
    {#if active.base === 'codex' && config.codex !== 'ready'}<div class="status-card warn"><Icon name="Tutor" /><div><strong>Not signed in to ChatGPT</strong><p>{appState.signingIn ? 'Finish signing in on the page that opened in your browser.' : 'Roadmap signs in to your ChatGPT account itself. No key, and nothing else to install.'}</p>{#if recheckError}<p class="err">{recheckError}</p>{/if}<button type="button" class="btn sm" id="chatgpt-sign-in" onclick={() => void signIn()}><BusyLabel busy={appState.signingIn} idle="Sign in with ChatGPT" label="Open the page again" /></button></div></div>
    {:else if active.base === 'codex'}<div class="status-card ok"><Icon name="check" /><div><strong>Signed in to ChatGPT</strong><p>{config.chatgpt_email ? `As ${config.chatgpt_email}. ` : ''}Roadmap asks ChatGPT directly and needs no key.</p>{#if recheckError}<p class="err">{recheckError}</p>{/if}<button type="button" class="btn secondary sm" id="chatgpt-sign-out" disabled={rechecking} onclick={() => void signOut()}><BusyLabel busy={rechecking} idle="Sign out" label="Signing out…" /></button></div></div>
    {:else}<div class="status-card"><Icon name="Tutor" /><div><strong>{active.base ? 'Your own AI service' : 'No AI service set'}</strong>{#if active.base}<p class="service-url">{active.base}</p><p>Model: {active.model}</p>{:else}<p>Fill in the form below, or go back to ChatGPT.</p>{/if}</div></div>{/if}
    {#if active.base === 'codex'}<fieldset class="field" id="effort-field" aria-describedby="effort-hint"><legend>Default thinking effort</legend><div class="segmented">{#each EFFORTS as [v, label]}<label><input type="radio" name="effort" value={v} checked={config.effort === v} onchange={() => void setEffort(v)} />{label.replace(' effort', '')}</label>{/each}</div><p class="small" id="effort-hint">How hard ChatGPT thinks when building a roadmap and in the tutor. More effort is slower but more careful. You can change it for one roadmap or in the tutor.</p>{#if effortError}<p class="err" role="alert">{effortError}</p>{/if}</fieldset>{/if}
    <div class="models-box" aria-busy={fetching}><div class="row between models-head"><strong>Models{found && found.base !== active.base.replace(/\/+$/, '') ? ' at ' + found.base : ''}</strong><button type="button" class="btn secondary sm" id="fetch-models" disabled={fetching} onclick={() => void fetchModels()}><BusyLabel busy={fetching} idle="Fetch models" label="Fetching…" /></button></div>
      {#if fetchError}<p class="err" role="alert">{fetchError}</p>{/if}
      {#if fetching && !found}<Skeleton kind="models" />{:else if found}
        {#if found.models.length > 10}<input type="search" class="model-filter" aria-label="Filter models" placeholder={`Filter ${found.models.length} models`} bind:value={filter} />{/if}
        {#if !found.models.length}<p class="small">{found.removed.length ? 'Every model is removed. Restore one below.' : 'This service listed no models.'}</p>{/if}
        <div class="model-list ruled">{#each shown as m (m.id)}<div class="model-row"><span class="grow"><b>{m.name}</b>{#if m.name !== m.id}<small>{m.id}</small>{/if}</span>{#if isDefault(m.id)}<span class="tag ok">Default</span>{:else}<button type="button" class="link" disabled={busy} onclick={() => void setDefault(m.id)}>Set as default</button><button type="button" class="link danger" onclick={() => void hide(m, true)}>Remove</button>{/if}</div>{/each}</div>
        {#if found.removed.length}<details class="disclosure"><summary>Removed models ({found.removed.length})</summary><div class="model-list ruled">{#each found.removed as m (m.id)}<div class="model-row"><span class="grow"><b>{m.name}</b>{#if m.name !== m.id}<small>{m.id}</small>{/if}</span><button type="button" class="link" onclick={() => void hide(m, false)}>Restore</button></div>{/each}</div></details>{/if}
      {/if}</div>
    <details class="disclosure" bind:open={other}><summary>{active.base === 'codex' ? 'Use a different AI service' : 'Change the AI service'}</summary>
      <form id="settings" onsubmit={e => { e.preventDefault(); void saveSettings(); }}><p class="small">Any OpenAI-compatible service works. Its key stays on this computer.</p><p class="small quick-label">Quick fill</p><div class="examples">{#each fills as [k, n]}<button type="button" onclick={() => { config!.base = bases[k]; if (k === 'codex') config!.model = 'default'; void fetchModels(); }}>{n}</button>{/each}</div><label for="base">Base URL</label><input type="text" id="base" required bind:value={config.base} onchange={() => void fetchModels()} placeholder="Your service’s API base URL, or codex for ChatGPT" /><label for="model">Model name</label><input id="model" list="service-models" onfocus={() => void loadModels()} required bind:value={config.model} placeholder="With ChatGPT, default means its first model" /><datalist id="service-models">{#each models.items as m}<option value={m.id}>{modelName(m.id)}</option>{/each}</datalist>{#if models.loading && !models.error}<Skeleton kind="models" />{/if}{#if models.error}<p class="small">{models.error}</p>{/if}<label for="small">Small model (optional)</label><input id="small" bind:value={config.small} placeholder="Leave empty to use the model above" /><p class="small field-note">A cheaper, faster model for quizzes, video picks, marking and placement questions. Roadmaps and the tutor keep the main model.</p><label for="key">API key</label><input id="key" type="password" autocomplete="off" bind:value={key} onchange={() => { if (key.trim()) void fetchModels(); }} placeholder={config.key_set ? 'Saved. Leave empty to keep it.' : 'Not needed for ChatGPT or a local Ollama model'} /><p class="save row"><button class="btn" disabled={busy}><BusyLabel {busy} idle="Save settings" label="Saving…" /></button><span id="saved" role="status">{saved}</span></p>{#if saveError}<p class="err" role="alert">The settings were not saved. {saveError}</p>{/if}</form>
    </details>
  </div></section>
  <section class="settings-layout" id="video-prefs"><div><h2>Videos</h2><p>Default preferences for video picks.</p></div><VideoPrefs initial={config.videos} /></section>
  <section class="settings-layout"><div><h2>Usage</h2><p>What this app has sent to and received from your AI service.</p></div><Usage usage={config.usage} /></section></div>{/if}
</div>
