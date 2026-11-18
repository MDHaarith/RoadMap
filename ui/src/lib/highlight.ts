// Colours code in the exercise editor: comments, strings, numbers, keywords and called functions.
// shortcut: one regex per language, not a parser; odd cases (Rust lifetimes, heredocs) colour slightly wrong. Swap in a real highlighter if that matters.
const esc = (s: string) => s.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const C = 'auto break case char const continue default do double else enum extern float for goto if inline int long return short signed sizeof static struct switch typedef union unsigned void volatile while bool true false NULL size_t';
const KEYWORDS: Record<string, string> = {
  c: C,
  cpp: C + ' class namespace new delete public private protected template typename using virtual override this nullptr try catch throw std string vector',
  python: 'and as assert async await break class continue def del elif else except False finally for from global if import in is lambda None nonlocal not or pass raise return True try while with yield',
  javascript: 'async await break case catch class const continue default delete do else export extends false finally for function if import in instanceof let new null of return switch this throw true try typeof undefined var void while yield',
  bash: 'if then else elif fi for while until do done case esac in function return local export exit',
  ruby: 'def end if elsif else unless while until for in do return class module self nil true false and or not then yield begin rescue ensure',
  php: 'function return if else elseif foreach for while echo class public private new null true false array as',
  lua: 'and break do else elseif end false for function if in local nil not or repeat return then true until while',
  perl: 'my our sub return if elsif else unless while for foreach use last next',
  r: 'function if else for while repeat return TRUE FALSE NULL NA in',
  rust: 'as break const continue crate else enum false fn for if impl in let loop match mod move mut pub ref return self Self static struct trait true type use where while',
  go: 'break case chan const continue default defer else fallthrough for func go goto if import interface map package range return select struct switch type var true false nil',
  java: 'abstract boolean break byte case catch char class continue default do double else enum extends final finally float for if implements import int interface long new null package private protected public return short static super switch this throw throws true false try void while',
};
const HASH = ['python', 'bash', 'ruby', 'perl', 'r'];
export function highlight(code: string, lang: string): string {
  const words = new Set((KEYWORDS[lang] || '').split(' '));
  const comment = lang === 'lua' ? String.raw`--[^\n]*` : HASH.includes(lang) ? String.raw`#[^\n]*` : String.raw`\/\/[^\n]*|\/\*[\s\S]*?(?:\*\/|$)`;
  const re = new RegExp(String.raw`(${comment})|("(?:[^"\\\n]|\\.)*"?|'(?:[^'\\\n]|\\.)*'?|${'`'}(?:[^${'`'}\\]|\\.)*${'`'}?)|(\b\d[\w.]*)|(^[ \t]*#[ \t]*\w+)|([A-Za-z_]\w*)(?=\s*\()|([A-Za-z_]\w*)`, 'gm');
  let out = '', last = 0;
  for (const m of code.matchAll(re)) {
    const [t, comm, str, num, directive, call, word] = m;
    const kind = comm ? 'c' : str ? 's' : num ? 'n' : directive ? 'k' : call ? (words.has(call) ? 'k' : 'f') : words.has(word) ? 'k' : '';
    out += esc(code.slice(last, m.index)) + (kind ? `<span class="hl-${kind}">${esc(t)}</span>` : esc(t));
    last = m.index + t.length;
  }
  return out + esc(code.slice(last));
}
