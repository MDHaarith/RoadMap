# Roadmap design foundation

For anyone restyling a screen. Everything lives in `src/app.css`; component `<style>` blocks are the exception (only `Chart.svelte` has one).

## The look

Clean and minimal, like Linear or Notion. Neutral greys, **one accent colour (azure)**, 1 px borders, compact spacing, small clear text. No shadows except on floating things (menus, toast, ideas panel). Do not bring back warm charcoal/amber or periwinkle: both were rejected.

## Tokens (`:root`; light only, no dark theme)

| Group | Tokens | Use |
| --- | --- | --- |
| Surfaces | `--bg` page, `--bg-subtle` quiet panels and notes, `--surface` cards, `--surface-2` wells/code/tracks, `--hover` | Hover is always `--hover`, never a darker accent. |
| Lines | `--line` default border, `--line-strong` inputs-like borders, `--field-line` text inputs | |
| Text | `--ink` main, `--ink-2` body copy on tinted areas, `--muted` meta text (AA on `--bg` and `--bg-subtle`) | |
| Accent | `--accent` fills (buttons, bars), `--accent-hover`, `--accent-ink` accent as text/links/icons, `--accent-soft` tint for "current/selected", `--accent-line` border on tint, `--on-accent` text on fills | Accent = "where you are / what to do next". Use it for nothing else. |
| Status | `--success(-soft)` done/correct, `--danger(-soft)` errors/wrong, `--warn(-soft)` safety notes only | |
| Type | `--font` system stack, `--mono`; sizes `--t-xs` 11, `--t-sm` 12, `--t-md` 13 (body), `--t-lg` 14, `--t-xl` 16, `--t-2xl` 20 | No remote fonts (CSP). |
| Space | `--s1`…`--s7` = 3, 6, 8, 10, 12, 16, 24 px | |
| Radius | `--r-sm` 5, `--r` 8 (controls), `--r-lg` 10 (cards) | |
| Layout | `--top-h` 36 px title bar, `--content` 760 px reading width | |

All text pairs were checked for WCAG AA (≥ 4.5:1) in light mode, including white on `--accent`.

## Component classes

- **Layout:** `main` (page padding), `.wrap` / `.wrap.narrow` (1440 max; reading copy 760), `.row`, `.between`, `.grow`, `.page-head`/`.intro` (title left, actions right), `.section-title`/`.library-title` (h2 + meta), `.back-map` (small back link above a title).
- **Buttons:** `.btn` primary, `.btn.secondary`, `.btn.ghost` (quiet), sizes `.btn.sm` and `.btn.big`; `.link` (text button), `.link.danger`, `.plain` (muted underlined).
- **Rule: one primary `.btn` per view.** Everything else is `secondary`, `ghost` or `.link`. If two things compete, the rare one becomes secondary or moves into a `.menu`.
- **Inputs:** plain `input`/`textarea`/`select` are styled globally; `label` is block. `.own` = input + button row. `.checks` = big checkbox rows (whole row is the label, ticked rows turn green). `.segmented` (or `#theme`, `.practice-nav`) = segmented control from radio labels or `aria-pressed` buttons; `.segmented.full` stretches it to equal columns, and a label may hold `<span>Title<small>meta</small></span>`. `.field` = fieldset with a small `legend`, `.hint` = one muted line under a control (put the chosen option's description there instead of on every option). `.examples`/`.chip` = pill quick picks.
- **Surfaces:** `.card`, `.panel` (with `h2` header), `.note` (quiet info box; `.note.warn`, `.note.setup`), `.celebrate` (success box), `.err` (error box), `.tag` (`.tag.accent`, `.tag.ok`), `.empty-state` (dashed box: icon, h2, one line, at most one button), `.status-card` (current state of a setting; `.status-card.ok` = green, nothing to do), `.disclosure` (quiet `<details>` for rare options, summary looks like a link with a chevron).
- **Progress:** `<Progress>` renders `.bar > i`; size it by its parent.
- **Menu:** `<details class="menu"><summary class="btn secondary sm">…</summary><div class="menu-list"><button>Title<small>One-line why</small></button>…</div></details>`. Close it on Escape/outside click (see `Overview.svelte`).
- **Waiting:** `<Waiting title="What is happening" hint="how long it usually takes">optional extra</Waiting>` or `<Waiting inline title="…" />`. It shows a spinner and a live m:ss clock (`<Elapsed />` alone if you only need the clock). **Every AI wait uses it.** Typical waits: tutor 10–15 s, quiz ~30 s, phase ~1 min, new roadmap 1.5–4 min.
- **Focused pages** (review, exam, search, mistakes): `.back-map` link, then `.session-head` (h1 left; a count `.session-count` or the exam `.timer` right), then `.session-bar` (thin progress). Finished states use `.score` (`.score.pass` = green).
- **Lesson parts:** `.lesson-steps` (sticky stepper), `.step-heading` (step n of N, title, one line saying what to do), `.step-footer` (sticky; back on the left, the one primary on the right), `.content-list` (numbered cards), `.option`/`.feedback`/`.score`/`.quiz-meta`/`.question` (quiz, shared with Review and Exam).
- **Roadmap parts:** `.next-card` (the up-next call to action), `.lesson-row` with state class `done | next | part | locked | outline`, `.stage-head`, `.phase-flag`, `.exam-row`, `.phase-index` (sticky side index).

## Rules followed

1. Every screen answers "what do I do now?" with one primary button, placed where the eye lands first (home hero, roadmap next-card, lesson footer).
2. Rare actions go into a "More" menu or a quiet link; they are moved, never removed.
3. State is shown with shape and colour together (check, filled ring, lock, dashed ring), never colour alone.
4. Performance first (WebKitGTK): no blur, no big shadows, no scroll-linked JS, motion only on opacity and transforms; loops only on waiting indicators, all disabled under `prefers-reduced-motion`.
5. Focus is always visible (`:focus-visible` 2 px accent outline). Keep existing ids, `aria-*` and labels; tests and keyboard code rely on them.
6. Plain, short English. Say what happens and how long it takes.

## Part 2 (done 2026-11-11)

`Creation`, `Settings`, `Usage`, `Review`, `Exam`, `Search`, `Mistakes` and `Ideas` follow this system (bottom of `app.css`). The creation form fits a 1366 × 768 window without scrolling, in both the build and the project variant. The roadmap build wait uses `<Waiting>` with a time that depends on the chosen size. Screenshots: `…/ui-shots/part2/`; the extended harness is in `…/ui-shots/part2/harness/` (same commands as below; extra mock options are listed at the top of its `mock.js`).

## Screenshot harness

`/tmp/claude-1000/-home-mdhaarith-Documents-Projects-Roadmap-app/4ef5acfd-2d66-4f1b-9723-ea1115570e73/scratchpad/ui-shots/part1/harness/`: `npm run build`, then `python3 prepare.py` (copies `dist`, injects `mock.js` + generated `data.js`), serve `site/` with `python3 -m http.server 8765 -d site`, write a list with `python3 make_shots.py [filter] [themes] [sizes]`, and shoot with `python3 shoot.py OUT_DIR shots.txt` (offscreen WebKitGTK 4.1, the app's own engine). `mock.js` documents its query options (`theme`, `empty`, `wait=…`, `do=click:…;type:…;scroll:…`).

Pages now use a 1440px maximum with existing modest margins; reading copy remains constrained. The lesson composer grows from two to six lines, uses an arrow-only send control and a session-cached native model picker. Settings offers the same models with manual entry. Notes offers rewrite, elapsed waiting and Undo. Tutor requests include the selected video.

Compact desktop scale: body 13px; secondary 12/11px; headings 14/16/20px. Controls 28px, small buttons 24px, primary large buttons 32px; radii 5/8/10px, button radii 10/12/14px. Tutor width 310px. Home uses two independently scrolling columns above 960px, stacking below it. Video keeps its 16:9 ratio and is capped at 533 × 300px.

Home keeps Today on the left and the library on the right at desktop widths.
The viewport owns no home-page scroll; each column can scroll independently.
Below 960px the columns stack and normal page scrolling returns. Library items
use 6px vertical / 10px horizontal padding, one summary line, a progress track,
and the existing Open, Pause/Resume and Delete actions. Import and New roadmap
live above the library. Empty libraries retain the creation form.

Charts use token-based panel padding and stop growing at their native 640px
width, keeping their 11px labels readable on desktop. The lesson video stops at
533px wide (300px tall); the title, chapters and first list entry remain visible
at 1366 × 768.

Density verification uses the part3 WebKitGTK harness with light-mode shots at
1366 × 768 and 1920 × 1080, plus a 900px stacked-home check. Query `lib=2|6|10`
selects library sizes; `empty=1` selects creation. Screenshots and measured page /
column dimensions are in the sibling `part3/compact/` folder. The fixture has
four active roadmaps plus one paused roadmap by default.

Final home measurements at 1366 × 768: two, default four active plus paused,
and six total roadmaps have no page or column overflow. Ten total roadmaps
keep the page fixed and scroll the library internally; its last paused item is
reachable. The 900 × 768 stacked case has no horizontal overflow.

The roadmap uses a 350px left column for its title, progress, next lesson and stage jumps, beside the lesson list. Both columns scroll independently within the window. At 1180px the left column narrows to 320px; below 960px they stack with normal page scrolling. Stage or phase jumps appear whenever there is more than one group.

## Motion

`--motion-fast` is 110ms, `--motion-enter` is 180ms, and `--motion-ease` is cubic-bezier(.2, 0, 0, 1). Screens enter only; lesson steps, sidebar panes, messages, revealed stages and forms, menus, home rows, feedback, completion marks and toast enter quietly. Progress uses scaleX from the left. Controls transition colour and press slightly. Home staggers only eight items, capped at 175ms. Resting layout stays unchanged; reduced motion disables all motion. Never animate geometry, shadows or filters, add blur, smooth scrolling or scroll effects. Only waiting indicators loop.
