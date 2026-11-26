<script lang="ts">
  import { modelName } from '../lib/models.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { appState, message, refreshConfig, signInChatGpt } from '../lib/state.svelte';
  import type { Settings } from '../lib/types';
  import { updates, checkUpdate, refreshUpdate } from '../lib/updates.svelte';
  import UpdateStatus from '../components/UpdateStatus.svelte';
  import VideoPrefs from '../components/VideoPrefs.svelte';
  import Waiting from '../components/Waiting.svelte';
  import Icon from '../components/Icon.svelte';
  import BusyLabel from '../components/BusyLabel.svelte';
  import LoadError from '../components/LoadError.svelte';
  import { openProviders, serviceName } from '../components/ProviderDialog.svelte';
  let config = $state<Settings>(), error = $state('');
  let alive = true;
  function load() { error = ''; api<Settings>('GET', 'api/settings').then(c => { if (alive) config = c; }).catch(e => { if (alive) error = message(e); }); }
  // The background update check starts a few seconds after launch; read the version now so About is never blank.
  onMount(() => { load(); if (!updates.version) void refreshUpdate().catch(() => {}); return () => { alive = false; }; });
  let checking = $state(false);
  async function check() { checking = true; try { await checkUpdate(true); } finally { checking = false; } }
  // The AI service summary; the provider dialog changes it.
  let c = $derived(appState.config);
  let missing = $derived((c.base === 'claude' && c.claude === false) || (c.base === 'opencode' && c.opencode === false));
  let signedOut = $derived((c.base === 'codex' && c.codex === 'signed_out') || (c.base === 'claude' && c.claude_auth === 'signed_out'));
  let signInError = $state('');
  async function signIn() {
    signInError = '';
    try { await signInChatGpt(); await refreshConfig(); } catch (e) { signInError = message(e); }
  }
  let runError = $state('');
  async function setUnsandboxed(on: boolean) {
    runError = '';
    try { await api('PUT', 'api/unsandboxed', { on }); if (config) config.unsandboxed = on; appState.config.unsandboxed = on; }
    catch (e) { runError = message(e); }
  }
</script>
<div class="wrap narrow settings screen-enter"><a class="back-map" href="#/"><Icon name="back" /> Back</a><div class="page-head"><h1>Settings</h1></div>
  {#if error}<LoadError what="Settings could not load." {error} retry={load} />{:else if !config}<Waiting skeleton="settings" inline title="Opening settings" />{:else}
  <div class="settings-rows stagger"><section class="settings-layout"><div><h2>About</h2><p>{updates.version ? `Version ${updates.version}` : 'Roadmap'}</p></div><div class="row"><button class="btn secondary sm" disabled={checking || updates.state === 'downloading'} onclick={() => void check()}><BusyLabel busy={checking} idle="Check for updates" label="Checking…" /></button><UpdateStatus /></div></section>
  <section class="settings-layout"><div><h2>AI service</h2><p>Roadmap uses it to plan lessons, prepare practice, and give you hints.</p></div><div>
    <div class="status-card ai-summary" class:ok={!!c.base && !missing && !signedOut} class:warn={!c.base || missing || signedOut}><Icon name={c.base && !missing && !signedOut ? 'check' : 'Tutor'} /><div>
      <strong>{c.base ? serviceName(c.base) : 'No AI service set'}</strong>
      <p>{#if !c.base}Choose one to plan lessons and use the tutor.{:else if signedOut}{c.base === 'claude' ? 'Not signed in. Run claude in a terminal and sign in.' : 'Not signed in to ChatGPT.'}{:else if missing}Not installed on this computer.{:else}{c.base === 'codex' && c.chatgpt_email ? `Signed in as ${c.chatgpt_email}. ` : c.base === 'claude' && c.claude_email ? `Signed in as ${c.claude_email}. ` : ''}Model: {modelName(c.model)}{/if}</p>
      {#if signInError}<p class="err" role="alert">{signInError}</p>{/if}
      <div class="row">{#if signedOut && c.base === 'codex'}<button type="button" class="btn sm" id="chatgpt-sign-in" onclick={() => void signIn()}><BusyLabel busy={appState.signingIn} idle="Sign in with ChatGPT" label="Open the page again" /></button>{/if}<button type="button" class="btn sm" class:secondary={!!c.base} onclick={openProviders}>{c.base ? 'Change provider' : 'Choose a provider'}</button></div>
    </div></div>
  </div></section>
  {#if config.os && config.os !== 'linux'}<section class="settings-layout" id="exercises"><div><h2>Exercises</h2><p>On Linux, code exercises run in a full sandbox. Here they get only some limits.</p></div><div><label class="check-row"><input type="checkbox" id="unsandboxed" checked={!!config.unsandboxed} onchange={e => void setUnsandboxed(e.currentTarget.checked)} /> Run exercises on this computer</label><p class="small">Your code and the lesson’s tests then run with a time limit and a 64 MB limit on the files they write. {config.os === 'windows' ? 'Memory and the number of programs are limited too, but they can still use the internet and change your files.' : 'They can still use the internet and change your files.'} Read an exercise’s tests before you run them.</p>{#if runError}<p class="err" role="alert">{runError}</p>{/if}</div></section>{/if}
  <section class="settings-layout" id="video-prefs"><div><h2>Videos</h2><p>Default preferences for video picks.</p></div><VideoPrefs initial={config.videos} /></section>
  <section class="settings-layout"><div><h2>Usage</h2><p>What this app has sent to and received from your AI service.</p></div><div id="usage"><p class="usage-total"><b>{((config.usage?.in || 0) + (config.usage?.out || 0)).toLocaleString('en')}</b> tokens in {(config.usage?.calls || 0).toLocaleString('en')} request{config.usage?.calls === 1 ? '' : 's'}</p><p class="small">By model, roadmap and day, with cache hits and costs.</p><a class="btn secondary sm" href="#/usage">Open usage <Icon name="arrow" /></a></div></section></div>{/if}
</div>
