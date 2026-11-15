<script module lang="ts">
  type Question = { message: string; action: string; danger?: boolean };
  let question = $state<(Question & { resolve: (answer: boolean) => void; opener: HTMLElement | null }) | null>(null);

  export async function confirmAsk(options: Question): Promise<boolean> {
    if (question) return false;
    return new Promise(resolve => { question = { ...options, resolve, opener: document.activeElement as HTMLElement | null }; });
  }
</script>

<script lang="ts">
  let dialog: HTMLDialogElement;
  $effect(() => {
    if (question) {
      dialog.showModal();
      dialog.querySelector<HTMLButtonElement>(question.danger ? '.secondary' : '.confirm-action')?.focus();
    }
  });
  function answer(value: boolean) {
    const current = question;
    if (!current) return;
    question = null;
    dialog.close();
    current.opener?.focus({ preventScroll: true });
    current.resolve(value);
  }
  function keyboard(event: KeyboardEvent) {
    event.stopPropagation();
    if (event.key === 'Tab') {
      event.preventDefault();
      const buttons = [...dialog.querySelectorAll<HTMLButtonElement>('button')];
      const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
      buttons[(index + (event.shiftKey ? buttons.length - 1 : 1)) % buttons.length].focus();
    }
    if (event.key === 'Escape') { event.preventDefault(); answer(false); }
    if (event.key === 'Enter') { event.preventDefault(); if (!question?.danger) answer(true); }
  }
</script>

<dialog class="confirm" bind:this={dialog} role="alertdialog" aria-modal="true" aria-labelledby="confirm-message" oncancel={event => { event.preventDefault(); answer(false); }} onkeydown={keyboard} onclick={event => { event.stopPropagation(); if (event.target === dialog) answer(false); }}>
  {#if question}<div class="confirm-body">
    <p id="confirm-message">{question.message}</p>
    <div class="row"><button type="button" class="btn secondary" onclick={() => answer(false)}>Cancel</button><button type="button" class:danger={question.danger} class:link={question.danger} class:btn={!question.danger} class="confirm-action" onclick={() => answer(true)}>{question.action}</button></div>
  </div>{/if}
</dialog>
