# Professional polish round: UI brief

This is the brief for the UI half of the "make it professional" round. It is written to be handed as it stands to a UI agent. The backend half and the logic half are done (see "Already done" below).

Repo: `/home/mdhaarith/Documents/Projects/Roadmap_app`. UI code is in `ui/src` (`screens/`, `components/`, `lib/`, `app.css`). Read `ui/DESIGN.md` first and follow it. Do not edit anything under `src/` (Rust). Do not add dependencies.

## Fixed rules of this UI (owner decisions)

- Light mode only. Dense scale for a 1366x768 window (body 13 px). Round corners, near-pill buttons.
- Keep the current look: Fraunces headings, one blue accent, ruled lists instead of cards, the route-with-stops motif. This is a polish round, not a redesign.
- Pages start at the left gutter and use the width. Only quiz, exam and review sessions, empty states, full-page waiting and full-page errors are centred.
- Every dropdown is `ui/src/components/Select.svelte`. Never a native `<select>`.
- Waiting is shown with skeleton shapes (`components/Skeleton.svelte`, `components/Waiting.svelte`), never a spinner or a rotating icon.
- Motion uses opacity and transform only, with the existing `--motion-*` tokens, and respects `prefers-reduced-motion`.
- Text for the learner is plain and short. No emoji.

## Already done (do not redo)

- "New questions" now adds to a lesson's question bank and never replaces it. `l.practice.quiz` is the whole bank, `l.practice.from` is where the newest set starts, and `quizSet(l)` in `lib/progress.ts` returns `{ from, quiz }`. `components/Practice.svelte` already asks only the newest set and reports mistakes with the bank index.
- The tutor records what it looked up. An assistant message can carry `steps: string[]` (`lib/types.ts`), filled live by `ask()` in `lib/state.svelte.ts` and saved with the chat.
- The tutor has a new tool that searches the course, the learner's notes and earlier chats.
- The daily review no longer shows a concept in full for "explain" items. `q.q` is now a recall question or a short cue.
- Marking an "explain" answer returns a mark scheme: `s.result.points: { text: string; hit: boolean }[]`.
- `Session.picks?: number[]` exists in `lib/types.ts` for the answer review below. Nothing fills it yet.

## Part A. Answer review at the end of a quiz and a stage exam

Today the finished screen shows only a score and says "Review the answers you missed", but shows no answers.

- Record the chosen option per question in `Session.picks`: in `components/Practice.svelte` `answer(i)` and in `screens/Exam.svelte` `examAnswer(i)` (there `i` is -1 when the time ran out).
- On the finished screen (lesson quiz, exam pass, exam fail) show a "Review answers" list under the score, in the order asked. A missed question is open and shows the question, the learner's pick in the existing "wrong" treatment (or "No answer. Time ran out."), the correct option in the existing "correct" treatment, and the `why` text. A question answered right is one quiet line with a tick, and opens (`<details>`) to show the `why`.
- When a finished score comes from saved state and there are no picks (page reloaded), show no list and keep today's text.
- Build it as one small shared component so quiz and exam use the same thing.
- When `quizSet(l).from > 0`, add one quiet line on the lesson quiz's finished screen: how many questions the lesson's bank now holds, and that older ones return in the stage exam and the daily review.

## Part B. Tutor chat (`components/Tutor.svelte` and its CSS)

- Show a message's `steps` as a quiet, collapsed disclosure at the top of that reply. With one step the summary is the step itself; with more it is "Checked N things" and opens to the list. Small, muted, clearly secondary to the answer. The live status line while a reply is on its way stays as it is.
- Under the latest assistant reply only (not while a reply is on its way, not on a reply that ends with "(The answer was cut off.)" or is "(No answer.)"), show three follow-ups: "Quiz me on this", "Simpler", "An example". Clicking one calls the existing `send(...)` with exactly these texts:
  - `Quiz me on this: ask me one question at a time about what we just discussed.`
  - `Explain that again in simpler words.`
  - `Show me one concrete example of that.`
- There is already a row of starters above the composer (Hint, Quiz me, Explain, Review, Teach it back). The follow-ups must not look like a second competing button row: make them lighter than the starters, as text-style chips directly under the reply.

## Part C. Daily review (`screens/Review.svelte`)

- For `kind === 'explain'` items, show `q.q` as the main question (the `h3.question`) with the small label "Answer from memory" above it, and drop the grey `.recall` box. Keep the box for `code` items, where it holds the exercise prompt.
- Under the verdict and feedback, show `s.result.points` with the heading "What a full answer says": a tick beside each point the answer made, an empty stop beside each point it missed. No points, no list.
- The green and red feedback strip on this screen has no inner padding on the left (the text touches the edge of the colour). Give it the same padding as the feedback box in `components/QuizQuestion.svelte`.

## Part D. Polish pass over the whole UI

Work through every screen and component. Known items:

1. Skeleton audit. Grep every waiting branch in `ui/src` (`busy`, `pending`, `loading`, `running`, `installing`, `adapting`, text ending in "…"). Any wait that can last longer than about 300 ms and shows only text must show a skeleton in the shape of what will arrive: the code output panel while code or tests run, the marking result while an answer is marked, video search results, plan options, the project file list, search results, the model list. Short actions (save, pause, delete) show a busy label on the button and nothing else. A button must not change width when its label changes to the busy label. No spinner anywhere.
2. Skeletons must match the layout that replaces them, so nothing jumps when content arrives. Check home, roadmap, lesson, quiz question, videos, tutor reply, review, mistakes, search, settings.
3. Errors. Every load error states what failed and offers one retry action in the same place. Mistakes and Search show an error with no way to retry. An error must not be only a toast when the page is left empty.
4. Keyboard and focus. Every interactive element has a visible focus ring from one shared rule. Escape closes every menu, the Ideas panel and the confirm dialog. Check tab order on the lesson page.
5. Consistency. One button height scale, one radius scale, one icon size scale, spacing from the tokens. Replace inline `style="..."` attributes in `.svelte` files with classes. Delete CSS that nothing uses.
6. Long text. Long roadmap, lesson and video titles truncate or wrap cleanly, with no overflow and no horizontal scroll at 1366x768 and at the minimum window size in `tauri.conf.json`.
7. Numbers. Counts, scores and timers use tabular numerals so they do not jitter. The usage date in Settings is ambiguous ("11/8/2025"): format it as day, short month name, year.
8. Anything else that reads as unfinished: misaligned baselines, uneven gaps, a control that jumps on hover, text that touches an edge.

## Checking the work

- `npm run check --prefix ui` ends with 0 errors and 0 warnings. `npm test --prefix ui` passes.
- Render the screens in the app's real engine (WebKitGTK) at 1366x768:
  `python3 ~/.cache/roadmap-ui-shots/render_all.py /home/mdhaarith/Documents/Projects/Roadmap_app ~/.cache/roadmap-ui-shots/pro/after`
  A third argument filters by screen name. "Before" pictures of all 53 screens are in `~/.cache/roadmap-ui-shots/pro/before/`.
- The harness is in `~/.cache/roadmap-ui-shots/harness/` (`make_shots.py`, `mock.js`). Add shots for the new states: lesson quiz finished with wrong answers and the answer review, exam failed with the answer review, tutor reply with steps and follow-ups, explain review answered with a mark scheme, code running. For this, extend `mock.js`: its `chat_stream` handler can send steps with `args.onStatus.onmessage({ text, done: true })` (the status channel carries `{ text: string, done: boolean }` objects), and its `api/review/answer` reply can include `points`.
- Never copy `harness/data.js` or its content into the repository: it is made from the owner's private roadmaps.
- Open and look at every "after" picture, compare with "before", and fix what looks wrong before finishing.

## Report back

Files changed; what was done for each part (for Part D item by item, including items where nothing needed changing); which pictures were looked at and what was fixed after looking; anything left undone and why.
