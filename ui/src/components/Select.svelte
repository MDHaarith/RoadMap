<script lang="ts">
  import Skeleton from './Skeleton.svelte';
  import { tick } from 'svelte';
  import Icon from './Icon.svelte';
  let { value, options, onchange, disabled = false, id, ariaLabel, title, onopen, loading = false }: {
    value: string; options: { value: string; label: string }[]; onchange: (value: string) => void;
    loading?: boolean; disabled?: boolean; id?: string; ariaLabel?: string; title?: string; onopen?: () => void;
  } = $props();
  const uid = $props.id();
  let button: HTMLButtonElement;
  let list = $state<HTMLDivElement>();
  let open = $state(false), active = $state(0), above = $state(false);
  let left = $state(0), edge = $state(0), width = $state(0), height = $state(0);
  let typed = '', typedAt = 0;
  function position() {
    const r = button.getBoundingClientRect(), gap = 6, margin = 8;
    const below = Math.max(0, innerHeight - r.bottom - gap - margin), over = Math.max(0, r.top - gap - margin);
    above = below < Math.min(320, options.length * 30 + 12) && over > below;
    height = Math.min(320, above ? over : below);
    width = Math.min(innerWidth - margin * 2, Math.max(r.width, 200));
    left = Math.max(margin, Math.min(r.left, innerWidth - width - margin));
    edge = above ? innerHeight - r.top + gap : r.bottom + gap;
  }
  function visible() { list?.querySelector<HTMLElement>(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' }); }
  async function show(last = false) {
    if (disabled || open) return;
    active = options.findIndex(o => o.value === value);
    if (active < 0) active = last ? options.length - 1 : 0;
    typed = ''; position(); open = true; onopen?.();
    await tick(); list?.focus(); visible();
  }
  function close(focus = true) { open = false; if (focus) button.focus(); }
  function choose(index: number) { const option = options[index]; close(); if (option) onchange(option.value); }
  function keyboard(e: KeyboardEvent) {
    if (loading && !['Tab', 'Escape'].includes(e.key)) { e.preventDefault(); return; }
    if (e.key === 'Tab') { if (open) close(); return; }
    if (e.key === 'Escape') { if (open) { e.preventDefault(); e.stopPropagation(); close(); } return; }
    if (e.key === 'Enter' || (e.key === ' ' && !typed)) {
      e.preventDefault(); if (open) choose(active); else void show(); return;
    }
    if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) {
      e.preventDefault();
      if (!open) { void show(e.key === 'ArrowUp'); return; }
      active = e.key === 'Home' ? 0 : e.key === 'End' ? options.length - 1 : Math.max(0, Math.min(options.length - 1, active + (e.key === 'ArrowDown' ? 1 : -1)));
      visible(); return;
    }
    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault(); if (!open) void show();
      typed = Date.now() - typedAt > 700 ? e.key : typed + e.key; typedAt = Date.now();
      const search = [...typed].every(c => c === typed[0]) ? typed[0] : typed;
      const start = search.length === 1 ? active + 1 : active;
      for (let i = 0; i < options.length; i++) {
        const index = (start + i) % options.length;
        if (options[index].label.toLocaleLowerCase().startsWith(search.toLocaleLowerCase())) { active = index; visible(); break; }
      }
    }
  }
  function portal(node: HTMLDivElement) {
    document.body.append(node);
    const scroll = (e: Event) => { if (!node.contains(e.target as Node)) position(); };
    document.addEventListener('scroll', scroll, true);
    return { destroy() { document.removeEventListener('scroll', scroll, true); node.remove(); } };
  }
  $effect(() => { if (open) { options.length; position(); } });
</script>

<svelte:window onpointerdown={e => { if (open && !button.contains(e.target as Node) && !list?.contains(e.target as Node)) close(false); }} onresize={() => { if (open) position(); }} onblur={() => { if (open) close(false); }} />
<button type="button" class="select-trigger" {id} {title} {disabled} aria-label={ariaLabel} aria-haspopup="listbox" aria-expanded={open} aria-controls={`${uid}-list`} bind:this={button} onclick={() => open ? close() : void show()} onkeydown={keyboard}>
  <span>{options.find(o => o.value === value)?.label || value}</span>
  <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>
</button>
{#if open}
  <div use:portal bind:this={list} id={`${uid}-list`} class="menu-list select-list popover-enter" class:above role="listbox" tabindex="-1" aria-label={ariaLabel} aria-labelledby={ariaLabel ? undefined : id} aria-busy={loading} aria-activedescendant={!loading && options[active] ? `${uid}-${active}` : undefined} onkeydown={keyboard} onfocusout={e => { if (open && !list?.contains(e.relatedTarget as Node)) close(false); }} style:left={`${left}px`} style:top={above ? 'auto' : `${edge}px`} style:bottom={above ? `${edge}px` : 'auto'} style:width={`${width}px`} style:max-height={`${height}px`}>
    {#if loading}<Skeleton kind="models" />{:else}{#each options as option, i}
      <button type="button" tabindex="-1" id={`${uid}-${i}`} role="option" aria-selected={option.value === value} data-index={i} class:active={active === i} onpointermove={() => active = i} onclick={() => choose(i)}><span>{option.label}</span><span class="select-check">{#if option.value === value}<Icon name="check" />{/if}</span></button>
    {/each}{/if}
  </div>
{/if}
