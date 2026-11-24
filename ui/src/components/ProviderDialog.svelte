<script module lang="ts">
  // The services to pick from. `model` is filled in on picking one; `found` marks something on this computer, checked before use.
  export type Service = { base: string; name: string; about: string; model?: string; found?: 'opencode' | 'claude' | 'ollama' };
  const LOCAL_TUTOR = 'Through it the tutor cannot search the course or run your tests.';
  const NO_KEY: Service[] = [
    { base: 'codex', name: 'ChatGPT', about: 'Roadmap signs in to your ChatGPT account, and your plan pays.', model: 'default' },
    { base: 'claude', name: 'Claude Code', about: `Roadmap asks the Claude Code on this computer, and your Claude plan pays. ${LOCAL_TUTOR}`, model: 'default', found: 'claude' },
    { base: 'opencode', name: 'OpenCode', about: `Roadmap asks the OpenCode on this computer, so OpenCode Zen’s free models work without an account. ${LOCAL_TUTOR}`, model: 'opencode/big-pickle', found: 'opencode' },
    { base: 'http://localhost:11434/v1', name: 'Ollama', about: 'Models running in Ollama on this computer.', found: 'ollama' }];
  const WITH_KEY: Service[] = [
    { base: 'https://opencode.ai/zen/v1', name: 'OpenCode Zen', about: 'OpenCode’s pay-as-you-go service. Get a key at opencode.ai.' },
    { base: 'https://opencode.ai/zen/go/v1', name: 'OpenCode Go', about: 'OpenCode’s subscription. Get a key at opencode.ai.' },
    { base: 'https://generativelanguage.googleapis.com/v1beta/openai', name: 'Google Gemini', about: 'Get a key at aistudio.google.com.' },
    { base: 'https://api.anthropic.com/v1', name: 'Claude API', about: 'Pay per use with a key from the Claude Console.' },
    { base: 'https://api.deepseek.com/v1', name: 'DeepSeek', about: 'Get a key at platform.deepseek.com.' },
    { base: 'https://dashscope-intl.aliyuncs.com/compatible-mode/v1', name: 'Qwen', about: 'Get a key from Alibaba Cloud Model Studio.' },
    { base: 'https://api.moonshot.ai/v1', name: 'Kimi', about: 'Get a key at platform.moonshot.ai.' },
    { base: 'https://api.z.ai/api/paas/v4', name: 'GLM', about: 'Get a key at z.ai.' },
    { base: 'https://openrouter.ai/api/v1', name: 'OpenRouter', about: 'One key for models from many services. Get it at openrouter.ai.' },
    { base: '', name: 'Another service', about: 'Any service with an OpenAI-compatible API: fill in its address and key.' }];
  const GROUPS: [string, Service[]][] = [['No key needed', NO_KEY], ['With an API key', WITH_KEY]];
  const norm = (b: string) => b.trim().replace(/\/+$/, '');
  const serviceOf = (b: string) => [...NO_KEY, ...WITH_KEY].find(x => x.base === norm(b)) ?? WITH_KEY[WITH_KEY.length - 1];
  /** A readable name for a saved base: the service's name, or its address for another service. */
  export const serviceName = (base: string) => !base ? '' : serviceOf(base).base ? serviceOf(base).name : norm(base);
  let open = $state(false);
  /** Opens the provider picker; Settings and the "no AI service" notes call it. */
  export function openProviders() { open = true; }
</script>

<script lang="ts">
  // The AI service picker: services on the left, the picked one's setup on the right, in a dialog.
  import Skeleton from './Skeleton.svelte';
  import BusyLabel from './BusyLabel.svelte';
  import Icon from './Icon.svelte';
  import { models, loadModels, modelName } from '../lib/models.svelte';
  import { api } from '../lib/api';
  import { appState, message, refreshConfig, signInChatGpt, EFFORTS } from '../lib/state.svelte';
  import { confirmAsk } from './Confirm.svelte';
  import type { Settings } from '../lib/types';
  let dialog: HTMLDialogElement, opener: HTMLElement | null = null;
  let config = $state<Settings>(), loadError = $state(''), saveError = $state(''), key = $state(''), busy = $state(false), saved_ = $state('');
  // What is saved now.
  let active = $state({ base: '', model: '', small: '' });
  const alive = true;
  // The service in the form, saved or not.
  // "Another service" has no fixed address, so an empty address means nothing is picked yet unless the learner chose it.
  let other = $state(false);
  let picked = $derived(config && (config.base || other) ? serviceOf(config.base) : undefined);
  // Unknown until checked counts as there, so nothing flashes "Not installed" while the checks run.
  const absent = (x: Service) => !!x.found && config?.[x.found] === false;
  const signedOutOf = (x: Service) => x.found === 'claude' && !absent(x) && config?.claude_auth === 'signed_out';
  const ready = (x: Service) => !absent(x) && !signedOutOf(x);
  let missing = $derived(!!picked && absent(picked));
  let signedOut = $derived(!!picked && signedOutOf(picked));
  // Something typed in the form (not just a service looked at); closing then asks first.
  let edited = $state(false);
  let changed = $derived(!!config && norm(config.base) !== norm(active.base));
  // Anything in the form that differs from what is saved; the save button shows only then.
  let dirty = $derived(!!config && (changed || !!key.trim() || config.model !== active.model || (config.small || '') !== active.small));
  const inUse = (x: Service) => !!active.base && serviceOf(active.base) === x;
  function choose(x: Service) {
    if (!config) return;
    if (x === picked && x.base) return;
    other = !x.base;
    config.base = x.base || (picked === x ? config.base : '');
    const saved = config.providers?.[x.base];
    config.model = x.base && x.base === norm(active.base) ? active.model : saved?.model || x.model || '';
    config.small = saved?.small || '';
    config.key_set = !!saved?.key_set;
    key = ''; found = undefined; fetchError = ''; saved_ = ''; edited = false;
    // Arrow keys move through the list one service at a time: fetch only for the one the learner stops on.
    clearTimeout(fetchTimer); fetchRun++; fetching = false;
    if (config.base && ready(x)) { fetching = true; fetchTimer = setTimeout(() => void fetchModels(), 300); }
  }
  let fetchTimer: ReturnType<typeof setTimeout> | undefined, fetchRun = 0;
  // The models of the service in the form (saved or not): fetched from it, each with Set as default and Remove.
  type Found = { id: string; name: string; note?: string };
  let found = $state<{ base: string; models: Found[]; removed: Found[] }>(), fetching = $state(false), fetchError = $state(''), filter = $state('');
  let shown = $derived((found?.models || []).filter(m => !filter.trim() || (m.name + ' ' + m.id).toLowerCase().includes(filter.trim().toLowerCase())));
  async function fetchModels() {
    if (!config?.base.trim()) { fetchError = 'Fill in the Base URL first.'; return; }
    const base = config.base.trim().replace(/\/+$/, '');
    const run = ++fetchRun;
    fetching = true; fetchError = '';
    // A service picked since then has its own fetch; this answer is dropped.
    try { const r = await api<{ models: Found[]; removed: Found[] }>('POST', 'api/models/fetch', { base, key }); if (run === fetchRun) { found = { base, ...r }; filter = ''; } }
    catch (e) { if (run === fetchRun) { fetchError = message(e); found = undefined; } } finally { if (run === fetchRun) fetching = false; }
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
  function load() {
    loadError = ''; config = undefined; other = false; found = undefined; key = ''; saved_ = ''; saveError = ''; fetchError = ''; edited = false;
    Promise.all([api<Settings>('GET', 'api/settings'), local()]).then(([c, l]) => { config = { ...c, ...l }; active = { base: c.base, model: c.model, small: c.small || '' }; if (c.base && ready(serviceOf(c.base))) void fetchModels(); }).catch(e => { loadError = message(e); });
  }
  $effect(() => {
    if (open && dialog && !dialog.open) { opener = document.activeElement as HTMLElement | null; dialog.showModal(); load(); }
  });
  // Claude Code's sign-in and whether Ollama runs: slower checks, so only this dialog asks.
  const local = () => api<Partial<Settings>>('GET', 'api/local').catch(() => ({}));
  async function requestClose() {
    if (edited && dirty && !(await confirmAsk({ message: 'Close without saving? What you typed here will be lost.', action: 'Close without saving', danger: true }))) return;
    close();
  }
  function close() {
    clearTimeout(fetchTimer);
    open = false;
    if (dialog.open) dialog.close();
    opener?.focus({ preventScroll: true });
  }
  let rechecking = $state(false), recheckError = $state('');
  async function recheck() {
    rechecking = true; recheckError = '';
    try { const [c, l] = await Promise.all([refreshConfig(), local()]); if (config) Object.assign(config, { codex: c.codex, chatgpt_email: c.chatgpt_email, opencode: c.opencode, claude: c.claude }, l); }
    catch (e) { recheckError = message(e); } finally { rechecking = false; }
  }
  async function checkAgain() { await recheck(); if (picked && ready(picked)) void fetchModels(); }
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
  // OpenCode's own installer page is a terminal script; this downloads the same program into ~/.opencode/bin.
  let installing = $state(false), installError = $state('');
  async function installOpenCode() {
    installing = true; installError = '';
    try { await api('POST', 'api/opencode/install'); if (config) config.opencode = true; void fetchModels(); }
    catch (e) { if (alive) installError = message(e); } finally { if (alive) installing = false; }
  }
  let effortError = $state('');
  async function setEffort(effort: string) {
    effortError = '';
    try { await api('PUT', 'api/effort', { effort }); appState.config.effort = effort; if (config) config.effort = effort; }
    catch (e) { effortError = message(e); }
  }
  async function saveSettings() {
    if (!config) return; busy = true; saved_ = ''; saveError = '';
    const switched = changed;
    try { await api('PUT', 'api/settings', { base: config.base, model: config.model, small: config.small || '', key }); if (key) config.key_set = true; key = ''; saved_ = 'Settings saved.'; edited = false; appState.config = { ...config }; active = { base: config.base, model: config.model, small: config.small || '' }; void recheck(); void loadModels(true); if (switched) close(); }
    catch (error) { saveError = message(error); } finally { busy = false; }
  }
</script>

<dialog class="providers" bind:this={dialog} aria-labelledby="providers-title" oncancel={e => { e.preventDefault(); void requestClose(); }} onkeydown={e => e.stopPropagation()} onclick={e => { if (e.target === dialog) void requestClose(); }}>
  <div class="providers-head"><h2 id="providers-title">{appState.config.base ? 'Change provider' : 'Choose a provider'}</h2><button type="button" class="btn ghost sm providers-close" aria-label="Close" onclick={() => void requestClose()}><Icon name="close" /></button></div>
  {#if loadError}<p class="err providers-error" role="alert">Your settings could not load. {loadError}</p>{:else if !config}<div class="providers-wait"><Skeleton kind="models" /></div>{:else}
    <form id="providers-form" class="svc-panel" onsubmit={e => { e.preventDefault(); void saveSettings(); }} oninput={e => { const t = e.target as HTMLInputElement; if (t.type !== 'radio' && t.type !== 'search') edited = true; }}>
      <div class="svc-list" role="radiogroup" aria-label="AI service">{#each GROUPS as [title, list] (title)}<p class="svc-head" aria-hidden="true">{title}</p>{#each list as x (x.name)}<label class="svc-row" class:on={picked === x}><input type="radio" name="service" checked={picked === x} onchange={() => choose(x)} /><span class="svc-name">{x.name}</span>{#if inUse(x)}<span class="svc-state ok">In use</span>{:else if absent(x)}<span class="svc-state">{x.found === 'ollama' ? 'Not running' : 'Not installed'}</span>{:else if signedOutOf(x)}<span class="svc-state">Signed out</span>{/if}</label>{/each}{/each}</div>
      {#if picked}<div class="svc-detail">
        <div class="svc-title"><h3>{picked.name}</h3>{#if inUse(picked)}<span class="tag ok">In use</span>{:else if picked.found && !missing && picked.found !== 'ollama'}<span class="tag">On this computer</span>{/if}</div>
        <p class="svc-about">{picked.about}{!NO_KEY.includes(picked) || !picked.base ? ' The key stays on this computer.' : ''}</p>
        {#if picked.base === 'codex' && inUse(picked)}
          {#if config.codex !== 'ready'}<div class="svc-note warn"><p><b>Not signed in.</b> {appState.signingIn ? 'Finish signing in on the page that opened in your browser.' : 'Your browser opens the ChatGPT sign-in page.'}</p>{#if recheckError}<p class="err">{recheckError}</p>{/if}<button type="button" class="btn sm" id="chatgpt-sign-in" onclick={() => void signIn()}><BusyLabel busy={appState.signingIn} idle="Sign in with ChatGPT" label="Open the page again" /></button></div>
          {:else}<div class="svc-note"><p>Signed in{config.chatgpt_email ? ` as ${config.chatgpt_email}` : ''}.</p>{#if recheckError}<p class="err">{recheckError}</p>{/if}<button type="button" class="btn secondary sm" id="chatgpt-sign-out" disabled={rechecking} onclick={() => void signOut()}><BusyLabel busy={rechecking} idle="Sign out" label="Signing out…" /></button></div>{/if}
          <fieldset class="field" id="effort-field" aria-describedby="effort-hint"><legend>Default thinking effort</legend><div class="segmented">{#each EFFORTS as [v, label]}<label><input type="radio" name="effort" value={v} checked={(config.effort || 'medium') === v} onchange={() => void setEffort(v)} />{label.replace(' effort', '')}</label>{/each}</div><p class="small" id="effort-hint">More effort is slower but more careful. You can change it for one roadmap or in the tutor.</p>{#if effortError}<p class="err" role="alert">{effortError}</p>{/if}</fieldset>
        {/if}
        {#if picked.found === 'opencode' && missing}<div class="svc-note warn"><p><b>Not installed.</b> Install it here, or from opencode.ai.</p>{#if installError}<p class="err" role="alert">{installError}</p>{/if}<button type="button" class="btn sm" disabled={installing} onclick={() => void installOpenCode()}><BusyLabel busy={installing} idle="Install OpenCode" label="Installing…" /></button></div>
        {:else if picked.found === 'claude' && missing}<div class="svc-note warn"><p><b>Not installed.</b> Install it from claude.com/code, then run <code>claude</code> once in a terminal to sign in.</p>{#if recheckError}<p class="err">{recheckError}</p>{/if}<button type="button" class="btn secondary sm" disabled={rechecking} onclick={() => void checkAgain()}><BusyLabel busy={rechecking} idle="Check again" label="Checking…" /></button></div>
        {:else if picked.found === 'ollama' && missing}<div class="svc-note warn"><p><b>Not running.</b> Start Ollama, or install it from ollama.com, then check again.</p>{#if recheckError}<p class="err">{recheckError}</p>{/if}<button type="button" class="btn secondary sm" disabled={rechecking} onclick={() => void checkAgain()}><BusyLabel busy={rechecking} idle="Check again" label="Checking…" /></button></div>
        {:else if signedOut}<div class="svc-note warn"><p><b>Not signed in.</b> Run <code>claude</code> in a terminal and sign in, then check again.</p>{#if recheckError}<p class="err">{recheckError}</p>{/if}<button type="button" class="btn secondary sm" disabled={rechecking} onclick={() => void checkAgain()}><BusyLabel busy={rechecking} idle="Check again" label="Checking…" /></button></div>
        {:else}
          {#if picked.found === 'claude' && config.claude_email}<div class="svc-note"><p>Signed in as {config.claude_email}.</p></div>{/if}
          {#if !picked.base}<label for="base">Address</label><input type="text" id="base" required bind:value={config.base} onchange={() => void fetchModels()} placeholder="https://… (most end in /v1)" />{/if}
          {#if !NO_KEY.includes(picked)}<label for="key">API key{picked.base ? '' : ' (if it needs one)'}</label><input id="key" type="password" autocomplete="off" bind:value={key} onchange={() => { if (key.trim()) void fetchModels(); }} placeholder={config.key_set ? 'Saved. Leave empty to keep it.' : 'Paste your key'} />{/if}
          <div class="models-box" aria-busy={fetching}><div class="row between models-head"><strong>Models</strong><button type="button" class="link" id="fetch-models" disabled={fetching} onclick={() => void fetchModels()}><BusyLabel busy={fetching} idle={found ? 'Refresh' : 'Fetch models'} label="Fetching…" /></button></div>
            {#if fetchError}<p class="err" role="alert">{fetchError}</p>{/if}
            {#if fetching && !found}<Skeleton kind="models" />{:else if found}
              {#if found.models.length > 10}<input type="search" class="model-filter" aria-label="Filter models" placeholder={`Filter ${found.models.length} models`} bind:value={filter} onkeydown={e => { if (e.key === 'Enter') e.preventDefault(); }} />{/if}
              {#if !found.models.length}<p class="small">{found.removed.length ? 'Every model is removed. Restore one below.' : 'This service listed no models.'}</p>{/if}
              <div class="model-list ruled">{#each shown as m (m.id)}<div class="model-row"><span class="grow"><b>{m.name}</b>{#if m.note}<small class="model-note">{m.note}</small>{:else if m.name !== m.id}<small>{m.id}</small>{/if}</span>{#if isDefault(m.id)}<span class="tag ok">Default</span>{:else}<button type="button" class="link" disabled={busy} onclick={() => void setDefault(m.id)}>{changed ? 'Use this' : 'Set as default'}</button><button type="button" class="link danger" onclick={() => void hide(m, true)}>Remove</button>{/if}</div>{/each}</div>
              {#if found.removed.length}<details class="disclosure"><summary>Removed models ({found.removed.length})</summary><div class="model-list ruled">{#each found.removed as m (m.id)}<div class="model-row"><span class="grow"><b>{m.name}</b>{#if m.name !== m.id}<small>{m.id}</small>{/if}</span><button type="button" class="link" onclick={() => void hide(m, false)}>Restore</button></div>{/each}</div></details>{/if}
            {/if}</div>
          <details class="disclosure"><summary>More options</summary>
            <label for="model">Model name</label><input id="model" list="service-models" onfocus={() => void loadModels()} required bind:value={config.model} placeholder="With ChatGPT or Claude Code, default means their own pick" /><datalist id="service-models">{#each models.items as m}<option value={m.id}>{modelName(m.id)}</option>{/each}</datalist>
            <label for="small">Small model (optional)</label><input id="small" bind:value={config.small} placeholder="Leave empty to use the model above" /><p class="small field-note">A cheaper, faster model for quizzes, video picks, marking and placement questions. Roadmaps and the tutor keep the main model.</p>
          </details>
          {#if dirty || saved_}<p class="save row"><button class="btn" disabled={busy || !config.base.trim() || !config.model.trim()}><BusyLabel {busy} idle={changed ? `Use ${picked.base ? picked.name : 'this service'}` : 'Save settings'} label="Saving…" /></button><span id="saved" role="status">{saved_}</span>{#if changed && !config.model.trim()}<span class="small">Pick a model first.</span>{/if}</p>{/if}{#if saveError}<p class="err" role="alert">The settings were not saved. {saveError}</p>{/if}
        {/if}
      </div>{:else}<div class="svc-detail svc-empty"><p>Pick a service on the left. Those without a key run on your own plan or computer.</p></div>{/if}
    </form>
  {/if}
</dialog>
