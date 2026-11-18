<script lang="ts">
  // Tells the learner why AI features will not work yet, before a call fails: no service, or not signed in to ChatGPT.
  import { aiProblem, signInChatGpt, appState, message } from '../lib/state.svelte';
  import BusyLabel from './BusyLabel.svelte';
  let { why, compact = false }: { why: string; compact?: boolean } = $props();
  let problem = $derived(aiProblem()), error = $state('');
  async function signIn() {
    error = '';
    try { await signInChatGpt(); } catch (e) { error = message(e); }
  }
</script>
{#if problem}
  <div class="note setup" class:compact role="status">
    <div>
      <strong>{problem === 'none' ? 'No AI service connected' : 'Not signed in to ChatGPT'}</strong>
      {#if problem === 'none'}<p>{why}</p>
      {:else}<p>{appState.signingIn ? 'Finish signing in on the page that opened in your browser.' : 'Sign in with your ChatGPT account. Your browser opens the sign-in page.'}</p>{/if}
      {#if error}<p class="err">{error}</p>{/if}
    </div>
    {#if problem === 'none'}<a class="btn secondary" class:sm={compact} href="#/settings">Open Settings</a>
    {:else}<button type="button" class="btn secondary" class:sm={compact} onclick={signIn}><BusyLabel busy={appState.signingIn} idle="Sign in with ChatGPT" label="Open the page again" /></button>{/if}
  </div>
{/if}
