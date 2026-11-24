<script lang="ts">
  // Tells the learner why AI features will not work yet, before a call fails: no service, a missing program, or not signed in.
  import { aiProblem, signInChatGpt, appState, message, refreshConfig } from '../lib/state.svelte';
  import BusyLabel from './BusyLabel.svelte';
  import { openProviders } from './ProviderDialog.svelte';
  let { why, compact = false }: { why: string; compact?: boolean } = $props();
  let problem = $derived(aiProblem()), error = $state(''), checking = $state(false);
  async function check() {
    error = ''; checking = true;
    try { await refreshConfig(); if (aiProblem()) error = 'Still not signed in.'; } catch (e) { error = message(e); } finally { checking = false; }
  }
  async function signIn() {
    error = '';
    try { await signInChatGpt(); } catch (e) { error = message(e); }
  }
</script>
{#if problem}
  <div class="note setup" class:compact role="status">
    <div>
      <strong>{problem === 'none' ? 'No AI service connected' : problem === 'claude_signed_out' ? 'Not signed in to Claude Code' : problem === 'missing' ? `${appState.config.base === 'claude' ? 'Claude Code' : 'OpenCode'} is not on this computer` : 'Not signed in to ChatGPT'}</strong>
      {#if problem === 'none'}<p>{why}</p>
      {:else if problem === 'missing'}<p>Install it, or pick another AI service.</p>
      {:else if problem === 'claude_signed_out'}<p>Run <code>claude</code> in a terminal and sign in, then check again. Or pick another AI service.</p>
      {:else}<p>{appState.signingIn ? 'Finish signing in on the page that opened in your browser.' : 'Sign in with your ChatGPT account. Your browser opens the sign-in page.'}</p>{/if}
      {#if error}<p class="err">{error}</p>{/if}
    </div>
    {#if problem === 'none' || problem === 'missing'}<button type="button" class="btn secondary" class:sm={compact} onclick={openProviders}>{problem === 'none' ? 'Choose a provider' : 'Change provider'}</button>
    {:else if problem === 'claude_signed_out'}<div class="row"><button type="button" class="btn secondary" class:sm={compact} disabled={checking} onclick={check}><BusyLabel busy={checking} idle="Check again" label="Checking…" /></button><button type="button" class="btn secondary" class:sm={compact} onclick={openProviders}>Change provider</button></div>
    {:else}<button type="button" class="btn secondary" class:sm={compact} onclick={signIn}><BusyLabel busy={appState.signingIn} idle="Sign in with ChatGPT" label="Open the page again" /></button>{/if}
  </div>
{/if}
