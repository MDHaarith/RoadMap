# Roadmap — a printed course book with a route line

This is the handoff for the remaining screen work. `src/app.css` owns the system. Home (`Library.svelte`, `LibraryCard.svelte`) and the roadmap (`Overview.svelte`) are the reference screens. Reuse their structure and the classes below; do not add a second styling layer or copy their screen-specific layout to unrelated pages.

## Character and density

White paper, near-black ink, the existing blue route (`--accent: #1a66c9`). Fraunces gives titles the character of a printed course book. The route with stops is the single recurring drawing. No purple, gradients, glass, blur, sparkles, emoji, blue-grey/periwinkle palettes, or charcoal/amber palettes. Light mode only.

Keep the compact scale: body **13px**, title bar **36px**, controls **28px**, small buttons **24px**, large primary buttons **32px**. Do not enlarge text, gaps or padding. Keep round corners and near-pill buttons. All counts inherit tabular numerals.

A bordered/tinted card is for **the screen's one next action** or **a clickable choice** (quiz options, pickers, plan options). Lists use rules. Explanation uses plain paragraphs or hanging numbers. Input borders, code viewers and floating-menu boundaries have a functional purpose; they are not a reason to box the surrounding section.

## Tokens

| Tokens | Purpose |
| --- | --- |
| `--bg`, `--surface` | White page and control paper, both `#ffffff`. |
| `--bg-subtle`, `--surface-2`, `--hover` | Quiet work areas, code/progress tracks, row hover. Never a generic card tray. |
| `--line`, `--line-strong`, `--field-line` | Hairline rules, stronger control/route lines, text-input borders. |
| `--ink`, `--ink-2`, `--muted` | Main `#18181b`, supporting `#3f3f46`, metadata `#595961`; no opacity on readable text. |
| `--accent`, `--accent-hover`, `--accent-ink` | Blue `#1a66c9`, hover `#1557ad`, links/current state. Keep this hue. |
| `--accent-soft`, `--accent-line`, `--on-accent` | Next-action tint `#eef4fc`, its border `#b9d1f2`, white on blue or ink. |
| `--success`, `--success-soft` | Completion/correctness `#1a7f45` and `#e9f6ee`. Use a check or label too. |
| `--danger`, `--danger-soft` | Errors/incorrect answers `#c0362c` and `#fdeeec`. |
| `--warn`, `--warn-soft` | Safety/consequence notices `#8c5a00` and `#fdf4e3`; never decoration. |
| `--player`, `--player-fg`, `--player-link` | Video-only dark surface and accessible light text/link. Not a dark theme. |
| `--font`, `--display`, `--mono` | System sans; `"Fraunces Variable", Georgia, "Times New Roman", serif`; system monospace. |
| `--t-xs` / `--t-sm` / `--t-md` / `--t-lg` / `--t-xl` / `--t-2xl` | 11 / 12 / 13 / 14 / 16 / 20px. Keep each screen's existing sizes. |
| `--s1`…`--s7` | 3 / 6 / 8 / 10 / 12 / 16 / 24px. |
| `--r-sm`, `--r`, `--r-lg`, `--r-btn`, `--r-btn-sm`, `--radius` | 5 / 8 / 10 / 12 / 10 / 8px. Large buttons use 14px. |
| `--top-h`, `--content`, `--side-w` | 36px title bar; 760px focused session content; 310px tutor (320px at the existing intermediate breakpoint). |
| `--route-line`, `--route-width`, `--route-stop` | `var(--line-strong)`, 2px, 22px; one route vocabulary. |
| `--overlay-shadow` | Floating menus, Ideas and toast only. |
| `--motion-fast`, `--motion-enter`, `--motion-slow` | **120 / 200 / 320ms**. |
| `--motion-ease`, `--motion-mark-ease` | `cubic-bezier(.2, 0, 0, 1)`; small-mark-only soft overshoot `cubic-bezier(.2, .8, .3, 1.18)`. |

Fraunces is installed as `@fontsource-variable/fraunces` (OFL) and imported once in `src/main.ts`. Vite bundles its WOFF2 files locally; CSP is `default-src 'self'`. Never load fonts from a CDN. Use `--display` **only at 14px or larger**: page `h1`, next-action titles, stage titles, large/hanging numerals and quiz letter badges. Keep 13px lesson/library row titles, 13px phase titles and 11px roadmap lesson numbers in the system sans face; numbers use tabular figures. Body, UI section headings, buttons, inputs, labels and navigation stay sans. Existing compact title sizes stay unchanged; hanging numerals and quiz letters use the 14px display minimum inside their existing fixed-width marks.

## One page frame

Use this order, with destination wording in the back link:

```svelte
<div class="wrap"> <!-- add narrow for a focused form, session or status page -->
  <a class="back-map" href="#/"><Icon name="back" /> Your library</a>
  <div class="page-head">
    <h1>Page title</h1>
    <div class="row"><!-- actions or factual count --></div>
  </div>
  <!-- content -->
</div>
```

The root Home page has no back link. A roadmap's back link says **Your library**; a lesson or stage exam links to its roadmap **by title**. No inconsistent "Back", "Go back" or "Back to…" wording. Keep accessible names used by keyboard code/tests when migrating existing markup. Move existing back-link elements without deleting IDs/ARIA attributes.

Choose the frame by its content. Long or wide pages stay anchored at the left 16px gutter and use the available width (`.wrap`, capped at 1440px): Home, roadmap overview, every lesson step, Search results, Mistakes notebook and Ideas. Keep the app title bar and lesson breadcrumb in place. Short forms and single focused tasks use `.wrap.narrow`: centre the **whole page** horizontally, including the back link and title, in one column. New roadmap (every state) and Settings cap at 860px; Review, Exam, empty states, full-page waiting and errors cap at 760px. All use the same 24px top offset inside the main gutter, never vertical dead-centre. Inside the column, text, labels and controls stay left-aligned; the title shares their left edge. Search and Mistakes switch to this focused frame only while empty or showing an error. At 1920×1080 the focused columns keep their caps; wide pages retain useful ruled rows and right-aligned metadata/actions, with reading prose capped at about 72ch (never long lines beyond about 90 characters).

`.page-head`, `.intro`, `.session-head`, `.create-head` share the title-left/actions-right layout; prefer `.page-head` for new edits. Preserve `.session-count` and `.timer` where used. Over-titles all use `.overline`/`.kicker`: 11px, uppercase, .06em tracking, weight 600.

`<TopBar>` stays on **every screen** with brand, course context/switcher/progress when available, Search, **Review with its due count (including 0)**, Ideas, Settings and window controls. It loads review counts even when opening a deep link. Do not hide Review on the roadmap, lesson, settings or mistakes page.

Desktop layouts that must remain intact:

- Home: Today left, library right; the page fits 1366×768; **only the library scrolls**. `.home-workspace`, `.today-column`, `.library-column` own this. Keep all Open, Pause/Resume, Delete, Import and New actions.
- Roadmap: `.overview-side` (350px, 320px at the intermediate breakpoint) and `.overview-main` independently scroll. Keep stage/phase jumps, lesson actions and all More actions.
- Lesson: article fills the width from the left gutter to the fixed right `.support` with Tutor and Notes tabs. The header and route span the article. On desktop the step body scrolls internally; the footer and adjacent-lesson links remain visible. Watch gives its 16:9 player the remaining width beside a 320px choices/coverage/search column, capped at 1100px wide and the body height left after its title/channel/chapters. The choices column scrolls only when needed; the footer stays visible. Other steps keep prose at 72ch, charts beside notes, files beside source, output beside the editor and quiz feedback beside choices. At smaller widths the supporting columns stack.
- Settings: the title and all sections share the centred 860px frame; retain 200px labels followed by controls and section rules across the column.
- Creation: the back link, title and whole form share the centred 860px frame in new, empty/onboarding, project, placement, building/waiting and error states. Fits 1366×768 without page scrolling, including project mode and errors.
  Level/size keep the existing black segmented controls and span the same width as the goal field.
  Both groups keep their changing explanation directly below the control, inside the centred form column.
- Review/Exam: the back link, title, progress and session content share the centred 760px frame and the same top offset as Creation/Settings.
- Search/Mistakes: left-anchored, full-width ruled rows, readable text and right-aligned row actions/metadata; empty and error states centre the whole page frame, including the header.
- Below 960px, home/roadmap stack and normal page scrolling returns. Existing tutor responsive behavior stays.

Every scrolling area uses the shared 10px classic scrollbar: a rounded grey thumb, darker on hover, on a quiet track. Keep the bar in its own space; use a stable gutter on independently scrolling panes. Do not set `scrollbar-width: thin` or `scrollbar-color` in WebKit/Chromium: those can override the classic bar and bring back overlays.

## Shared component classes

Each row names a component or a closely related family; descendants supply the styling, so keep the documented structure.

| Class/component | One-line use |
| --- | --- |
| `main`, `.wrap`, `.wrap.narrow` | Page gutter, left-anchored wide frame, whole centred 760px focus frame (860px for Creation/Settings), including back link/title. |
| `.row`, `.between`, `.grow`, `.stack` | Wrapping action row, spaced ends, flexible child, vertical spacing. |
| `.page-head`, `.intro`, `.session-head`, `.create-head`, `.head-row` | Existing header aliases; prefer page-head, title left and actions right. |
| `.back-map`, `.lesson-crumb` | Destination back link; lesson's factual location/breadcrumb. |
| `.section-title`, `.library-title`, `.route-heading` | Compact sans section heading with count/actions. |
| `.overline`, `.kicker`, `.side-title`, `.stage-label` | Single uppercase tracked over-title style. |
| `.muted`, `.small`, `.goal`, `.lead`, `.hint` | Supporting text, 12px metadata, factual goal, introduction, control explanation. |
| `.sr` | Visually hidden accessible label. |
| `.icon`, `.check-mark`, `.button-arrow` | 14px control/state SVG, normalized animated check, primary hover arrow. |
| `.count`, `.dot` | Factual tabular count; non-text route/index state dot. |
| `.btn`, `.btn.secondary`, `.btn.ghost`, `.btn.sm`, `.btn.big` | One blue primary, outlined secondary, quiet action; 24px/32px variants. Disabled buttons use muted ink on a neutral surface, never faded text. |
| `.link`, `.link.danger`, `.plain` | Text action, destructive text action, muted underlined disclosure. |
| `.own`, `.field`, `.field > legend` | Input-plus-action row; labelled group of controls. |
| `.checks`, `.check-row` | Clickable checkbox choice rows; checked state includes a real checkbox. |
| `.segmented`, `.segmented.full`, `#theme`, `.practice-nav`, `.side-tabs` | Outlined equal-width native/ARIA segments sharing one sliding ink fill; full stretches to width. |
| `.examples`, `.chips`, `.chip` | Existing compact quick-choice buttons; keep useful examples/actions. |
| `.ruled`, `.ruled-row` | Hairline-divided interactive list: direct children receive bottom rules; links/buttons/ruled-row get hover. |
| `.numbered`, `.content-list` | Plain `ol > li` explanation with automatic 01/02 hanging display numerals; never cards. |
| `.card`, `.options` | Bordered clickable choices (plan cards) or the single next-action card; options stacks choices. |
| `.panel`, `.panel h2` | Plain section with a top rule, sans heading; no surrounding box. |
| `.note`, `.note.warn`, `.note.setup`, `.change` | Unboxed factual notice, consequence notice, setup/action row, roadmap change report. |
| `.err`, `.celebrate` | Error status; unboxed completion section with a success rule. |
| `.tag`, `.tag.accent`, `.tag.ok`, `.tag.success` | Small factual/state labels, never decorative badges. |
| `.empty-state`, `.empty-route`, `<Icon name="empty-route" />` | Unboxed empty state with one small inline SVG route, heading, useful sentence and at most one primary. |
| `.status-card`, `.status-card.ok`, `.service-url` | Plain setting status with rule; success changes rule/icon, URL remains monospace. |
| `.disclosure` | Native details for rare options, with rotating chevron and visible focus. |
| `.bar > i`, `.bar.wide`, `<Progress>` | Progress track/fill, optional full width, accessible lesson count. |
| `.menu > summary + .menu-list` | Native details action menu, rounded 24px trigger and floating button list; retain outside-click/Escape handlers. |
| `.waiting`, `.waiting.inline`, `<Waiting>`, `.elapsed` | Live status and elapsed clock above a result-shaped skeleton; inline variant uses the same pattern. |
| `.home-head`, `.hero`, `.hero-empty`, `.hero-meta` | Home title row, one next-action card, caught-up variant and factual progress row. |
| `.side-actions`, `.side-action`, `.resume-list`, `.resume-row` | Home's ruled utility links and other in-progress lessons. |
| `.cards`, `.lib-card`, `.card-progress`, `.card-actions`, `.paused-cards` | Library's ruled roadmap rows and retained progress/actions; the historical names do not imply cards. |
| `.overview`, `.overview-side`, `.overview-main`, `.overview-head` | Roadmap's independently scrolling context/list columns and header. |
| `.progress-line`, `.next-card` | Roadmap factual progress and its one next lesson call to action. |
| `.needs`, `.video-status`, `.phase-index` | Prerequisite checklist, video arrival fact, stage/phase jump list. |
| `.route-board`, `.stage-head`, `.stage-count`, `.blurb` | Lesson area, collapsible stage title, factual completion count and useful description. |
| `.route-phase`, `.phase-flag`, `.phase-blurb` | Phase group, phase title/count row, useful phase description. |
| `.route`, `.route-item`, `.route-stop`, `.lesson-list` | Vertical connected route container, state-bearing row, round stop; rules bound the group, never individual route rows. |
| `.lesson-action-row`, `.lesson-row`, `.state`, `.num`, `.stop-title`, `.stop-meta` | Whole-row highlight/action boundary; link, marker, number, lesson title and textual state. |
| `.done`, `.next`, `.part`, `.locked`, `.outline` on route/lesson rows | Check/solid green, blue ring, partially filled ring, lock, dashed outline; never colour alone. |
| `.exam-row`, `.exam-row.done` | Ruled stage checkpoint with secondary action; never a second blue primary on the roadmap. |
| `.lesson-layout`, `.lesson-article`, `.lesson-title`, `.lesson-content` | Reading article, display title and changing step content. |
| `.lesson-steps`, `.step-number`, `.visited` | Sticky seven-stop horizontal route; ink visited stops and blue/bold current stop. |
| `.step-heading`, `.step-body`, `.step-footer`, `.pager`, `.finish` | Single step instruction, body, sticky actions, adjacent lessons and completion actions. |
| `.concept-copy`, `.concept-known`, `.concept-recap`, `.lookup` | Plain concept prose, known material, factual recap marker and ruled lookup link. |
| `.locked-page`, `.writing-page`, `.loading` | Compact blocked/writing/opening states; use Waiting for AI work. |
| `.support`, `.side-tabs`, `.side-pane`, `.notes-pane` | Fixed tutor, shared sliding tabs, flexible panes and notes editor. |
| `.log`, `.msg`, `.msg.u`, `.msg.a`, `.msg.thinking`, `.seek` | Scrollable conversation; pale accent user bubbles, unboxed assistant text, skeleton reply lines and video-seek control. |
| `#compose`, `.composer-actions`, `.send`, `#notes` | Existing auto-growing composer, native model picker, arrow send and flexible notes field. |
| `.video-layout`, `.video-main`, `.video-choices`, `.player-space`, `.player`, `.player-thumbnail`, `.player-placeholder`, `.player-error` | Height-bounded 16:9 player up to 1100px wide on the left, 320px choices/search on the right, accessible player states. |
| `.video-title`, `.chapters`, `.vlist`, `.vid`, `.video-number`, `.video-tools`, `.why` | Video title, chapter choices, video-choice list/row, count and controls. |
| `.file-list`, `.file-heading`, `.code-viewer`, `.code-line` | Code-file choices, filename and scrollable numbered source. |
| `.practice-start`, `.practice-nav`, `.quiz-meta`, `.question`, `.options`, `.option`, `.letter` | Practice call to action, activity tabs, factual counts and quiz choices. |
| `.correct`, `.wrong`, `.answer-mark`, `.feedback`, `.score`, `.score.pass` | Quiz result uses tint plus check/cross and spoken correctness; ruled explanation and display-numeral final score. |
| `.finish-checks`, `.finish-check` | Ruled completion checklist; native checkboxes keep keyboard/label behavior, visible checked marks use `.complete-mark`. |
| `.completion-route`, `.completion-stop`, `.completion-traveller`, `.completion-next` | Success route: start fills, dot travels once, next stop appears; all motion uses opacity/transform. |
| `.exercise`, `.stuck`, `.code-label`, `.codebox`, `.run-actions` | Coding task, error/help area, editor label/input and run controls. |
| `.output`, `.output-header`, `.output.passed`, `.output.failed`, `.install-note` | Code run output/status with skeleton lines while running. |
| `.settings-layout`, `.usage-total`, `.save` | Existing settings sections, usage count and save-action row. |
| `.session-count`, `.session-bar`, `.review-wrap`, `.review-context`, `.recall`, `.answer-row`, `.keyboard-hint` | Focus-task counts/progress, review frame/context, answer controls and keyboard instructions. |
| `.exam-intro`, `.facts`, `.timer`, `.timer.low` | Exam's single start card, rules, remaining time and urgent state. |
| `.kind-choice`, `.folder-row`, `.folder-path`, `.privacy`, `.create-go`, `.create-error` | Creation mode, folder picker/path, data consequence, submit row and error. |
| `.create-goal`, `.placement`, `.told`, `.asking`, `.build-steps` | Creation summary, placement steps/answers and build progress. |
| `.result-list`, `.result`, `.result-head`, `.result-count` | Search's ruled result collection, clickable row and metadata. |
| `.mistake`, `.mistake.fixed`, `.picked`, `.mistake-foot`, `.answer`, `.mistake-link` | Ruled mistake entries, resolved state, attempted answer, disclosure and lesson link. |
| `.ideas`, `.ideas-head`, `.stickies`, `.sticky` | Floating right panel, controls and editable note fields; no stock art. |
| `.top`, `.brand`, `.crumb-sep`, `#ctitle`, `.switcher`, `.switch-toggle`, `.switch-menu` | Shared title-bar brand, course context and accessible course switcher. |
| `.meter`, `.top-search`, `.top-link`, `.window-controls` | Shared progress/search/utility links/native window actions. |
| `#toast`, `.update-notice` | Floating status toast; in-flow update notice above content. |

### Routes

Vertical structure (see Overview):

```svelte
<div class="lesson-list route stagger">
  <div class="lesson-action-row route-item done">
    <a class="lesson-row done" href="…">
      <span class="state route-stop" aria-hidden="true"><Icon name="check" /></span>
      <span class="num">1.1</span><span class="stop-title">Title</span>
      <span class="stop-meta">Complete</span>
    </a>
    <!-- LessonActions remains inside the row boundary -->
  </div>
</div>
```

Each route-item owns an incoming/outgoing half-line centered on its stop. Both halves are green only between adjacent `.done` rows. Other segments use `--route-line`. First/last ends stop at their marker. Set `--route-x` (default 21px) if reusing this with different row padding. Keep the marker on opaque paper and above the line. A `.lesson-list.route` has one rule above and below its group and no row dividers. Do not restore per-phase cards or clip the list with `overflow: hidden`: lesson menus must remain usable.

For a horizontal route use the existing `.lesson-steps` markup: seven buttons, each containing a `.step-label` with `.step-number` and a text span. The paper-backed `.step-label` has 6px padding on both sides, keeping the line clear of the stop/label group. `aria-current="step"` selects the blue stop; `.visited` fills previous stops with ink. No tray. `showStep` chooses direction before replacing the keyed content; `visit` replaces the visited array so Svelte updates the filled stops immediately.

`Waiting` preserves the job title, timing fact, live elapsed time and optional content above a result-shaped skeleton. Tutor waiting keeps its spoken status above skeleton text lines.

### Sliding controls

`.segmented`, `#theme`, `.practice-nav`, `.side-tabs` share a 1px outline and one ink pseudo-element. Existing native `input:checked`, `aria-pressed=true` or `aria-selected=true` determines its transform. Labels/buttons must be **direct children**, equal width; radio inputs stay inside labels. Native fieldset legends remain accessible. Two, three and four segments are detected automatically. Selected text is white, including metadata; unselected text is ink. Do not animate background/colour or add a selected-button background over the sliding fill. An added selector for a fifth segment would be needed if the product ever gains one; no current control has more than four.

Native `select` is white, 28px high, normal weight, uses the same field border and one local data-URI chevron. Keep a real select. The composer picker uses the same field treatment at its existing compact text size.

## Motion vocabulary

| Class / keyframe | One-line use |
| --- | --- |
| `.screen-enter` / `arrive` | Content rises 6px and fades in over 200ms; page roots already receive it automatically. |
| `.stagger` | Direct children 1–10 enter 20ms apart (0–180ms); subsequent rows have no entry animation. |
| `--row-order` | Assigned by stagger/stepper nth-child rules; reuse for ordered small marks. |
| `.route-item::before/::after` / `route-grow` | Vertical segments grow with scaleY from their top. |
| `.route-horizontal`, `.lesson-steps::before` / `fill-progress` | Horizontal line grows with scaleX from the left. |
| `.route-stop`, `.step-number` / `stop-appear` | Stops follow the line after 120ms, in order, with small-mark overshoot. |
| `.bar > i`, `.progress-fill` | Fill from the left; scaleX uses the real fraction, 320ms for changes. |
| `.complete-mark` / `ring-pulse`, `draw-check` | One ring pulse and a small check draw; put Icon check inside a mark-sized wrapper. Done route stops already attach both. |
| `.step-forward`, `.step-back` | Newly mounted lesson content comes 12px from right/left; use a keyed step and choose the class from the direction. `Lesson.svelte` chooses the class before mounting each keyed step; backward jumps enter from the left. |
| `.message-enter`, `.msg` | New tutor messages rise 6px and fade once. |
| skeletons | Three small dots while the tutor is working, including streaming; mount only while pending and retain spoken live status. |
| `.completion-traveller` / `completion-travel` | One 400ms dot journey after the success stop fills; the next stop appears at 520ms. No loop. |
| `.popover-enter` / `unfold` | Fade/scale .98 from top right; native menus apply this already, switcher uses top left. Set transform-origin to the opening corner for other popovers. |
| `.ideas` / `ideas-enter` | Right panel enters 12px from the right, once per open. |
| Buttons / `.button-arrow` in primary | Press scales .97; primary's trailing arrow shifts 2px on hover. |
| `.skeleton-shape`, `.sk` | Paint appears after 150ms; grey blocks pulse with opacity while waiting. |
| `#toast` / `toast-arrive` | Existing centered toast rises without losing its horizontal centering. |

Hard limits: **only opacity and transform animate**. Exceptions: the dashed waiting route's stroke offset and check marks no larger than 24px (normalized pathLength=1). Never animate width, height, position, spacing, shadows, filters or colours. No blur, smooth scrolling, scroll effects or permanent loops. Only waiting indicators loop. No persistent will-change. Resting geometry is identical with motion disabled. The last `prefers-reduced-motion: reduce` rule disables all transitions/animations, including pseudo-elements; preserve it.

## Copy, access and handoff rules

1. One primary blue button per view. On the roadmap stage exams are secondary because the next lesson already owns the primary. Rare actions move to menus/disclosures, never disappear.
2. Remove helper sentences that repeat a heading. Keep facts: counts, timings, consequences, data sent, saved state. Home's repetitive review/mistakes copy is already removed.
3. No decorative icons before headings. Keep state icons, control icons, route markers and accessible labels. Lesson step headings have no decorative icons.
4. Empty states use `<Icon name="empty-route" />` (inline SVG/currentColor/accent), no dashed empty box or stock illustration. Replace remaining old empty-state icons as those screens are edited.
5. Keep all feature IDs, `aria-*`, keyboard/test labels, focus behavior and validation. Tab/checkbox/native select semantics stay native. Focus-visible remains a 2px blue outline.
6. Every text/background pair must reach 4.5:1, including small metadata, selected-control metadata and text on state tints. Do not dim readable text with opacity.
7. Home's references intentionally retain summary, progress, all course actions and responsive stacking. Roadmap references retain phase generation, project-folder connection, exports, prerequisites, stage expansion and action previews.
8. Lesson direction, tutor reply skeletons, completion marks and success-route motion are wired. Tutor quick actions are one quiet text row at 310px. Videos and charts use ruled sections; project-file selection uses ink fill. The pages agent owns its remaining screen-specific refinements; preserve its concurrent changes.

## Phase-one check record

Svelte check: 0 errors/warnings. Six tests include an AA token-pair regression. The app's WebKitGTK harness rendered all 53 states at 1366×768. Home and ten-roadmap Home have a 1366×768 document, a 692px Today column that fits its content, and a library with overflow:auto (ten-roadmap content is 1003px). The 820px stepper has no horizontal overflow. Settings selects are 28px including borders and weight 400. The bundled Latin Fraunces face is loaded; Review has its due count on direct links. Runtime text-pair checks cover Home, creation, roadmap, lesson, settings, review, mistakes and search. The remaining screen-specific polish belongs to the next agents as listed above.

## Verification

Phase-two lesson check: Svelte check reports 0 errors and warnings; all six tests pass. The supplied WebKitGTK harness rendered all 60 states at 1366×768. Runtime inspection of 33 lesson/shared states found no text contrast below 4.5:1, no Fraunces below 14px and no document overflow. The seven-stop rail measures 820px with 6px paper gaps, ink visited stops and a 700-weight current label. Backward navigation, completion and three tutor dots have runtime assertions. System reduced motion produces no active animations; its resting backward-step screenshot is pixel-identical to normal motion. Additional pictures cover project files, the code exercise and its run wait, practice wait, locked/writing/error/missing routes and lesson completion. An unknown lesson currently returns to its roadmap; this routing behavior is preserved.

Run `npm --prefix ui run check` (0 errors) and `npm --prefix ui test`. Build/render with the app's offscreen WebKitGTK harness supplied in the task; inspect images at 1366×768, especially the home variants, roadmap action edges, selectors, seven-step rail and creation modes. Check actual viewport/column dimensions, locally loaded Fraunces and text contrast. Do not commit harness files, mock data, screenshots or private fixture content to the repository. Do not edit the harness.

## Pages

Creation and Settings use `wrap narrow screen-enter`; Review and Exam use the same focused frame. Their `back-map`, `page-head` and content align inside the centred column. Search and Mistakes use the wide left-anchored frame for populated lists and the focused frame for empty and error states. Library destinations say **Your library**; the stage exam names its course. The creation cancel link keeps `cancel-new` and its existing screen action. First-roadmap onboarding keeps the complete creation form, with the small route drawing above its title; it remains a one-window task, including project sharing and an error.

Review and Exam keep `session-head`, `session-count`, the timer and the thin `session-bar`. Their finished state adds `session-score`; its scoped `.complete-mark` supplies only a 24px green ring and check layout, reusing the shared completion animation. Scores retain the display face and tabular numerals; Usage uses that face for its 14px total too. The scoped `.create-empty-route .empty-route` only left-aligns the onboarding drawing. No page-specific keyframes were added.

Search results, mistake entries, settings sections, placement history and Ideas entries use rules and `stagger` (the existing first-ten-row limit). Placement answers stay boxed choices. Ideas keeps its existing right-side entry motion; `.sticky textarea` removes the resting field box, restores a field border on hover and retains the shared focus outline. Delete stays a quiet text action beside each editable note. Mistakes keeps the green correct-answer disclosure. The title-bar switcher reuses `popover-enter` and `stagger`; Review and its count remain visible on every route.

All motion uses the documented opacity/transform vocabulary, with the existing tiny-check exception. Page styles do not change shared tokens, buttons, segments, rules, route definitions, typography or keyframes. Home's two-column layout and calm lower-left space are retained.

## Visual polish conventions

Review and Exam show the question count only in their page header; lesson Practise keeps it in the shared quiz metadata. Correct and incorrect answer marks share a 20px filled circle, with a white check or cross. Plan option changes use an aligned sign column without bullets, 13px nested lines, and success/muted/ink for additions/removals/rewrites. Roadmap options follow their request input; their closing action says Cancel. Tutor quick actions retain one quiet row, with medium ink-2 text, 24px hit areas, pill hover backgrounds and the shared focus ring. Each visited lesson stop colours its outgoing segment in ink.

## Select and menus

Use `Select.svelte` for a finite choice, never a native select popup. Props: `value`, `options: { value, label }[]`, `onchange(value)`, optional `disabled`, `id`, `ariaLabel`, `title`, `onopen()`. Keep explicit labels wired to the trigger's `id`, or provide `ariaLabel`. The 28px `.select-trigger` uses the field border and chevron. `onopen` can load options asynchronously without closing the list.

The portalled `.menu-list.select-list.popover-enter` shares white paper, line border, 8px radius, 6px inset, overlay shadow, and 30px minimum rows with action menus and Switch. Supporting text may make a row taller. Pointer/keyboard rows use `--hover`; the selected option has a check. The list fits the viewport with an 8px margin, opens above when space below is insufficient, and scrolls at 320px maximum height. Upward lists use a bottom transform origin. Enter/Space/arrows open; arrows, Home/End and typeahead navigate; Enter/Space choose; Escape cancels; Tab closes and continues focus order. Outside clicks close without stealing the clicked control's focus. Keyboard dismissal and selection return focus to the trigger. Checkbox marks, search fields and number/date field chrome use app styling, without OS decorations.

## Tutor sidebar

Retain the fixed 310px sidebar, Tutor/Notes tabs and existing IDs. `.chat-log` contains the independently scrolling `#log` and the quiet Newest pill. Follow incoming messages only when already near the bottom; preserve the learner's position while they read older turns. Empty conversations show the existing introductory sentence and five `.ruled.tutor-starters` rows; after the first message the same actions occupy one compact row above the composer. New chat uses the existing destructive-action confirmation and clears this lesson's history.

User turns use `--accent-soft`; replies stay unboxed at 12px/1.65. `Formatted` adds paragraph and hanging-line layout hooks around the existing escaped formatter output, leaving fenced code intact. Code uses `--mono` on `--surface-2` and scrolls horizontally inside its block. Video seek links use 24px pill controls. Separate turns have more space than paragraphs. `.message-enter`, `.popover-enter` and skeletons reuse the existing opacity/transform motion; reduced-motion disables it all.

While waiting, show the live service status and `Elapsed` above three skeleton reply lines; keep elapsed ticks out of screen-reader announcements. The composer remains editable, with sending blocked until the reply ends and a square Stop control in place of Send. One field container holds textarea, model Select and Send/Stop, with a shared focus ring; its keyboard instruction stays visually hidden and available through `aria-describedby`, so focus changes only the ring. Long model labels truncate with an ellipsis. Failed sends restore their text only if the learner has not started the next draft. Notes fill the available pane; Rewrite/Undo share a row, Waiting reports rewrites, and the saved-with-lesson line remains visible.

## Content loading

Use `Skeleton.svelte` for known content shapes (`home`, `overview`, `lesson`, `videos`, `question`, `rows`, `settings`, `models`, `code`, `chart`, `notes`, `plan`, `plans`/`options` for choice cards, or text lines). Keep the final content’s column widths and row/control heights. The placeholder reserves space immediately, appears after 150ms, and pulses with opacity only; reduced-motion keeps it static after the same delay. Each region has `aria-busy="true"` and visually hidden “Loading”. Errors and empty results keep their own messages.

`Waiting` keeps the job description and elapsed time above a skeleton; choose its `skeleton` prop for long AI jobs. The route shell chooses the first-load shape from the destination. Tutor replies use three text lines beneath the service status and elapsed time. `Select` accepts `loading` for model rows. Lesson steps use one instruction heading; completion is a compact title badge, and lesson navigation explicitly says “lesson”.
