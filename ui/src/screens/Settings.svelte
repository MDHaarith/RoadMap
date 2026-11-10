<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { appState, toast, message } from '../lib/state.svelte';
  import type { Settings, Theme } from '../lib/types';
  import Usage from '../components/Usage.svelte';
  let config = $state<Settings>(), error = $state(''), key = $state(''), busy = $state(false), saved = $state('');
  let alive = true;
  const bases: Record<string, string> = { gemini: 'https://generativelanguage.googleapis.com/v1beta/openai', openrouter: 'https://openrouter.ai/api/v1', ollama: 'http://localhost:11434/v1' };
  onMount(() => { api<Settings>('GET', 'api/settings').then(c => { if (alive) config = c; }).catch(e => error = message(e)); return () => { alive = false; }; });
  async function theme(theme: Theme) {
    document.documentElement.dataset.theme = theme;
    try { await api('PUT', 'api/theme', { theme }); } catch (error) { toast(message(error)); }
  }
  async function saveSettings() {
    if (!config) return; busy = true; saved = '';
    try { await api('PUT', 'api/settings', { base: config.base, model: config.model, small: config.small || '', key }); if (key) config.key_set = true; key = ''; saved = 'Settings saved.'; appState.config = { ...config }; }
    catch (error) { toast(message(error)); } finally { busy = false; }
  }
</script>
<div class="wrap narrow"><div class="intro"><h1>Settings</h1><a href="#/">Back to roadmaps</a></div>
  {#if error}<p class="err">{error}</p>{:else if !config}<div class="loading" role="status">Opening settings…</div>{:else}
  <div class="settings-layout appearance"><div><h3>Appearance</h3><p class="muted">Light, dark, or whatever your desktop uses.</p></div><fieldset id="theme"><legend class="sr">Theme</legend>{#each [['light', 'Light'], ['dark', 'Dark'], ['system', 'Follow system']] as [k, n]}<label><input type="radio" name="theme" value={k} checked={config.theme === k} onchange={() => { config!.theme = k as Theme; void theme(k as Theme); }} /> {n}</label>{/each}</fieldset></div>
  <div class="settings-layout"><div><h3>AI service</h3><p class="muted">Connect an OpenAI-compatible AI service. Your key stays on this computer.</p><p class="small">Roadmap uses it to plan lessons, prepare practice, and give you hints.</p></div><form id="settings" onsubmit={e => { e.preventDefault(); void saveSettings(); }}><label for="base">Base URL</label><input type="url" id="base" required bind:value={config.base} placeholder="Your service’s API base URL" /><p class="small" style="margin:12px 0 0">Quick fill</p><div class="examples">{#each [['gemini', 'Google Gemini'], ['openrouter', 'OpenRouter'], ['ollama', 'Ollama on this computer']] as [k, n]}<button type="button" onclick={() => config!.base = bases[k]}>{n}</button>{/each}</div><label for="model">Model name</label><input id="model" required bind:value={config.model} /><label for="small">Small model (optional)</label><input id="small" bind:value={config.small} placeholder="Leave empty to use the model above" /><p class="small" style="margin:8px 0 0">A cheaper, faster model for quizzes, video picks, marking and placement questions. Roadmaps and the tutor keep the main model.</p><label for="key">API key</label><input id="key" type="password" autocomplete="off" bind:value={key} placeholder={config.key_set ? 'Saved. Leave empty to keep it.' : 'Not needed for a local Ollama model'} /><p class="save row"><button class="btn" disabled={busy}>Save settings</button><span id="saved" role="status">{saved}</span></p></form></div>
  <div class="settings-layout"><div><h3>Usage</h3><p class="muted">What this app has sent to and received from your AI service.</p></div><Usage usage={config.usage} /></div>{/if}
</div>
