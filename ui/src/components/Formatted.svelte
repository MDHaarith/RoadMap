<script lang="ts">
  import { fmt, fmtTutor } from '../lib/format';
  let { text, tutor = false }: { text: unknown; tutor?: boolean } = $props();
  const ROW = /^\s*\|.*\|\s*$/;
  // A table cannot sit inside <p>, so a paragraph with one becomes a div.
  // Lay out one escaped paragraph line by line: markdown headings, tables and (nested) list lines become blocks; plain lines keep their breaks.
  function lines(paragraph: string) {
    const ls = paragraph.split('<br>'), out: { html: string; block: boolean }[] = [];
    for (let i = 0; i < ls.length; i++) {
      if (ROW.test(ls[i])) {
        const rows: string[][] = [];
        for (; i < ls.length && ROW.test(ls[i]); i++) rows.push(ls[i].trim().slice(1, -1).split('|').map(x => x.trim()));
        i--;
        const head = rows.length > 1 && rows[1].every(x => /^:?-+:?$/.test(x)) ? rows.splice(0, 2)[0] : null;
        const cells = (r: string[], tag: string) => '<tr>' + r.map(x => `<${tag}>${x}</${tag}>`).join('') + '</tr>';
        out.push({ block: true, html: `<table class="tutor-table">${head ? '<thead>' + cells(head, 'th') + '</thead>' : ''}<tbody>${rows.map(r => cells(r, 'td')).join('')}</tbody></table>` });
        continue;
      }
      const h = ls[i].match(/^\s*#{1,6}\s+(.+)$/);
      if (h) { out.push({ block: true, html: `<strong class="tutor-heading">${h[1]}</strong>` }); continue; }
      const li = ls[i].match(/^(\s*)([-*•]|\d+[.)])\s+(.+)$/);
      if (li) { out.push({ block: true, html: `<span class="tutor-list-line${li[1].length >= 2 ? ' nested' : ''}"><span class="tutor-list-marker">${/\d/.test(li[2]) ? li[2] : '•'}</span><span>${li[3]}</span></span>` }); continue; }
      out.push({ block: false, html: ls[i] });
    }
    return out.map((x, i) => (i && !x.block && !out[i - 1].block ? '<br>' : '') + x.html).join('');
  }
  // Add layout hooks to the escaped formatter output; keep code blocks untouched.
  let tutorHtml = $derived(tutor ? fmtTutor(text).split(/(<pre>[\s\S]*?<\/pre>)/).map(part => {
    if (part.startsWith('<pre>')) return part;
    return part.replace(/^(<br>)+|(<br>)+$/g, '').split(/(?:<br>){2,}/).filter(Boolean).map(paragraph => { const html = lines(paragraph); return html.includes('<table') ? `<div class="tutor-p">${html}</div>` : `<p>${html}</p>`; }).join('');
  }).join('') : '');
</script>
{@html tutor ? tutorHtml : fmt(text)}
