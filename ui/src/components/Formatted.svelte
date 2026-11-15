<script lang="ts">
  import { fmt, fmtTutor } from '../lib/format';
  let { text, tutor = false }: { text: unknown; tutor?: boolean } = $props();
  // Add layout hooks to the escaped formatter output; keep code blocks untouched.
  let tutorHtml = $derived(tutor ? fmtTutor(text).split(/(<pre>[\s\S]*?<\/pre>)/).map(part => {
    if (part.startsWith('<pre>')) return part;
    return part.replace(/^(<br>)+|(<br>)+$/g, '').split(/(?:<br>){2,}/).filter(Boolean).map(paragraph =>
      '<p>' + paragraph.replace(/(^|<br>)([-*•]|\d+[.)])\s+([^]*?)(?=<br>|$)/g,
        '<span class="tutor-list-line"><span class="tutor-list-marker">$2</span><span>$3</span></span>') + '</p>'
    ).join('');
  }).join('') : '');
</script>
{@html tutor ? tutorHtml : fmt(text)}
