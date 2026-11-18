export const esc = (s: unknown) => String(s ?? '').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const emphasis = (s: string) => s.replace(/\*\*([^*]+)\*\*/g,'<strong>$1</strong>')
  // Single stars are emphasis only around a word, so 2*3*4 and "a * b" stay as typed.
  .replace(/(?<![\w*])\*([^*\s](?:[^*]*[^*\s])?)\*(?![\w*])/g,'<em>$1</em>')
  // Unbalanced pairs models sometimes write (**x* or *x**) still mean bold.
  .replace(/(?<![\w*])\*\*([^*\s](?:[^*]*[^*\s])?)\*(?![\w*])/g,'<strong>$1</strong>')
  .replace(/(?<![\w*])\*([^*\s](?:[^*]*[^*\s])?)\*\*(?![\w*])/g,'<strong>$1</strong>')
  // A marker left touching text on one side only is a stray: drop it, keep the text. Stars between or away from text stay.
  .replace(/(?<![^\s<>])\*+(?=[^\s*<>])|(?<=[^\s*<>])\*+(?![^\s<>])/g,'');
// Escape first, then mark up; code spans are left exactly as typed.
export const fmt = (s: unknown) => esc(s).replace(/`([^`]+)`/g,'<code>$1</code>').split(/(<code>[^<]*<\/code>)/).map((p,i)=>i%2?p:emphasis(p)).join('');
export const fmtTutor = (s: unknown) => String(s).split('```').map((p,i)=>i%2?'<pre>'+esc(p.replace(/^[a-z]*\n/,''))+'</pre>':fmt(p).replace(/\n/g,'<br>').replace(/\bvideo (\d) at (\d{1,3}):(\d\d)\b/gi,(m,n,a,b)=>`<button type="button" class="seek" data-seek="${n-1}" data-at="${a*60+ +b}">${m}</button>`)).join('');
export const enc = (s: string) => encodeURIComponent(s).replace(/'/g,'%27');
export const href = (c: string,l?: string) => '#/c/'+enc(c)+(l?'/'+enc(l):'');
const ICONS: Record<string, string>={'Study the graphs':'<path d="M4 4v16h16M8 16l4-5 3 3 5-6"/>',Watch:'<path d="m8 4 12 8-12 8z"/>','Learn first':'<path d="M9 18h6M10 22h4M12 2a6 6 0 0 0-4 10.5c1 1 1 2 1 3.5h6c0-1.5 0-2.5 1-3.5A6 6 0 0 0 12 2z"/>','Look it up yourself':'<circle cx="10" cy="10" r="6"/><path d="m15 15 6 6"/>','Read the code':'<path d="m8 7-5 5 5 5m8-10 5 5-5 5M14 4l-4 16"/>','Do it yourself':'<path d="M14 5a4 4 0 0 0 5 5l-9 10-5-1-1-4zM4 4l5 5"/>',Practice:'<path d="M4 5h16v14H4zM8 9l2 2-2 2M12 14h4"/>','Done when':'<circle cx="12" cy="12" r="9"/><path d="m7 12 3 3 7-7"/>',Tutor:'<path d="M3 4h18v13H9l-5 4v-4H3zM8 10h1m6 0h1"/>','My notes':'<path d="M5 3h14v18H5zM8 8h8M8 12h8M8 16h5"/>','Give me a hint':'<path d="M12 3a6 6 0 0 0-4 10l1 4h6l1-4a6 6 0 0 0-4-10zM9 21h6"/>','Quiz me':'<circle cx="12" cy="12" r="9"/><path d="M9 9a3 3 0 1 1 4 3c-1 .5-1 1-1 2m0 3v.1"/>','Explain a concept':'<path d="M3 4q5-2 9 1 4-3 9-1v15q-5-2-9 1-4-3-9-1zM12 5v15"/>','Review my work':'<circle cx="10" cy="10" r="6"/><path d="m15 15 6 6M7 10l2 2 3-4"/>',arrow:'<path d="M4 12h16m-6-6 6 6-6 6"/>',back:'<path d="M20 12H4m6-6-6 6 6 6"/>',plus:'<path d="M12 4v16M4 12h16"/>',copy:'<rect x="8" y="8" width="12" height="13" rx="1"/><path d="M16 8V3H3v13h5"/>',flag:'<path d="M5 22V3h14l-3 5 3 5H5"/>',Safety:'<path d="m12 3 10 18H2zM12 9v5m0 3v1"/>',lock:'<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',check:'<path d="m5 12 5 5 9-10"/>',more:'<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>',chevron:'<path d="m9 6 6 6-6 6"/>',review:'<path d="M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5"/>'};
export const icon = (n: string) => `<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS[n]||ICONS.arrow}</svg>`;
export const CHIPS=['Give me a hint','Quiz me','Explain a concept','Review my work','Let me teach it back'];
export const clock = (t: number) => Math.floor(t / 60) + ":" + String(t % 60).padStart(2, "0");
export const LANGUAGES: Record<string, string> = {python:'Python',javascript:'JavaScript',bash:'Bash',ruby:'Ruby',php:'PHP',lua:'Lua',perl:'Perl',r:'R',c:'C',cpp:'C++',rust:'Rust',go:'Go',java:'Java'};
export const languageName = (lang: string) => LANGUAGES[lang] || lang;
