# Frontend port parity — 2026-11-11

The original `../index.html` was read completely before implementation and remains in place. This implementation changes only `ui/`, stays on `feature/tauri-svelte-frontend`, and creates no commit. The concurrently added Tauri CLI dependency and `tauri` npm script are preserved.

The frontend uses Svelte 5 runes, TypeScript, and Vite, with a state-driven screen switch and no router, UI, or state-management library. All visible screens and active behaviors in the source have been ported.

## Transport and rendering

- `src/lib/api.ts`: one generic `api()` wrapper invokes `api` with `{method, path, body: body ?? {}}`. Every backend path includes `api/`, without a leading slash or URL query. Backend rejection objects become `Error` instances with `status` retained.
- `chatStream()` sends the original chat body through `chat_stream`, with a `Channel<string>` named `onPiece`. Partial answers, empty answers, failure restoration, and cut-off answers retain their original behavior.
- `src/api.rs::route()` was checked: route dispatch uses method/path/body. No old frontend API call depended on URL query parameters. YouTube and DuckDuckGo URL queries are external links, not API arguments.
- The original escape-first `esc`, `fmt`, and `fmtTutor` implementations are preserved with TypeScript annotations. `Formatted.svelte` only renders their escaped results. Other AI/user text, code, filenames, notes, search snippets, and errors use Svelte text bindings. `Icon.svelte` renders only the original fixed SVG artwork.
- `App.svelte` intercepts HTTP(S) anchor clicks, including middle-click, through the opener plugin. Iframes remain normal YouTube embeds.
- `TopBar.svelte` uses `getCurrentWindow()` for minimize, toggle maximize, and close. The Tauri drag-region attribute supplies native dragging and double-click maximize behavior, including macOS mouse-up semantics. Resize notifications update the restore/maximize button. The WebKitGTK bridge is gone.
- Startup reads `api/settings.theme` and sets `<html data-theme>`. Settings persists changes through `PUT api/theme`.

## Original function inventory

Paths in this table are relative to `ui/src/`. Rendering functions are now declarative Svelte markup unless the table names a retained function.

| Original function(s) | Current location and behavior |
| --- | --- |
| `normalize` | `lib/state.svelte.ts`: defaults progress maps, restores cached chats, removes the response-only chat field. |
| `api` | `lib/api.ts`: typed Tauri API wrapper and status-bearing errors. |
| `toast` | `lib/state.svelte.ts`, `App.svelte`: alert and original nine-second dismissal. |
| `save` | `lib/state.svelte.ts`: 400 ms debounced progress save, snapshot, error toast, flush before route changes/exports/close. |
| `renderTop` | `components/TopBar.svelte`: reactive course title, full-map link, completion meter, and switcher visibility. |
| `miniRoute` | `components/Progress.svelte`: original thin progress bar and accessible completion label. |
| `creationForm`, `resetCreation`, `setCreationKind`, `showCreation` | `screens/Creation.svelte`, `screens/Library.svelte`: fresh local form state on entry, build/project switch (segmented control above the goal box), optional project prompt, examples (a “Try:” row under the goal box), levels and duration (segmented controls; the chosen option's description shows under each), setup note, and cancel (“Back to library”, top right). |
| `pickFolder` | `screens/Creation.svelte`: `POST api/pick-folder`, busy state, chosen path, retry/error handling, and privacy copy. |
| `create` | `screens/Creation.svelte`: adaptive placement (one question at a time, answers as full-width option buttons), previous answers, typed/custom answers, Enter submission, skip, build status (a `<Waiting>` with elapsed time and the usual time for the chosen size), and navigation to the generated roadmap. A failed build now shows its error next to the build button (it was a toast). Leaving the screen releases a pending placement answer and ignores stale responses. |
| `library`, `renderLibrary` | `lib/state.svelte.ts::loadLibrary`, `screens/Library.svelte`: concurrent courses/settings/review loading, empty-library creation, Today, continuing courses, due review, active and paused cards. |
| `libraryCard` | `components/LibraryCard.svelte`: card title/summary/progress, finished state, pause/resume, confirmed deletion, and reload. |
| `adaptNow` | `lib/state.svelte.ts`: progress-driven adaptation and user-requested revision, busy guard, changed lessons, retained progress, result/error notices. `screens/Overview.svelte` retains a failed revision request for retry. |
| `expandPhase` | `lib/state.svelte.ts`: one request per course/phase, replace outline lessons, update the current lesson. |
| `writePhase` | `screens/Lesson.svelte`: writing screen, unfinished-output check, retry, and return to roadmap. |
| `stageStart` | `screens/Overview.svelte`: stage headings and index links when a phase opens a stage. |
| `nextLesson` | `lib/progress.ts`: resume the last unfinished lesson, otherwise the first pending lesson. |
| `writeEveryLesson` | `screens/Overview.svelte`: confirmed sequential phase writing, count/status, stop after the current request, and phase-specific errors. |
| `stageOpen` | `lib/progress.ts`, `screens/Overview.svelte`: saved stage expansion, next-stage default, toggling and jump-to-phase expansion. |
| `overview` | `screens/Overview.svelte`: roadmap heading, summary, next lesson, completion, progress, changes, setup needs, tools, stages, phases, lessons, added labels, locks, and phase navigation. |
| `examRow` | `components/ExamRow.svelte`: exam availability, prior score, pass/retry state, and five-question minimum. |
| `needsList`, `checkSetup` | `screens/Overview.svelte`: saved readiness checkboxes, installed-tool checks, and graceful setup-check failure. |
| `lockedPage` | `screens/Lesson.svelte`: remaining prerequisite lessons, exam link, and back link. |
| `exam`, `renderExam`, `examAnswer`, `examNext` | `screens/Exam.svelte`: shuffled pool of at most ten questions, introduction, 60-second timer per question, timeout, answer feedback, mistake recording, 70% passing threshold, best-result persistence, retry, and next-stage access. Timer is cleaned up on exit. |
| `drawRoutes` | Replaced by Svelte layout. The final source overview renders `.lesson-row` rather than `.route-stops`, so this SVG-measurement function had no matching elements and no visible effect. Its redundant resize listener is unnecessary. |
| `updateVideoStatus`, `watchVideoSearch` | `screens/Overview.svelte`: pending-video count and up to twelve 3.5-second metadata-only refreshes; local progress is never overwritten. Timer is cleaned up on exit. |
| `lessonSteps`, `renderRail` | `screens/Lesson.svelte`: conditional lookup/read steps, step numbers, current/visited states, phase/lesson position, full-map link, and progress. |
| `lesson` | `screens/Lesson.svelte`, `App.svelte`: missing/locked/outline routing, title, goal, completion tag, last-lesson save, previous/next lessons, tutor, and notes. |
| `list` | `screens/Lesson.svelte`: numbered tasks rendered with the original formatter. |
| `showStep` | `screens/Lesson.svelte`: all step bodies, heading, back/next footer, focus and scroll behavior. |
| `finishUI` | `screens/Lesson.svelte`, `lib/state.svelte.ts::finishLesson`: checklist or 80%-quiz unlock, complete/undo, achievement, next lesson, and phase-end adaptation/prefetch. |
| `loadVideos`, `showVideo`, `playVideo` | `components/Videos.svelte`: native API video loading/retry, thumbnails, explicit playback, chapter buttons, selected list, duration/channel/reason, fallback YouTube links, fresh picks, and custom topic search. |
| `renderLog` | `components/Tutor.svelte`: role-specific escaped messages, original tutor formatter, thinking status, auto-scroll, and global sending guard. |
| `ask` | `lib/state.svelte.ts`, `components/Tutor.svelte`: streaming channel, chips, send form, Enter versus Shift+Enter, partial-answer retention, and failed-message restoration. |
| `loadPractice` | `lib/state.svelte.ts`: generation/loading/error/retry, fresh questions, session reset, and saved-quiz invalidation, including responses arriving after navigation. |
| `renderPractice` | `components/Practice.svelte`: first-load invitation, busy/error states, quiz/exercise tabs, restored quiz results, retry, and fresh questions. |
| `stuckNote` | `components/Practice.svelte`: offer a smaller tutor step or added roadmap practice after three failed test runs or a quiz below 60%. |
| `quizQuestion` | `components/QuizQuestion.svelte`: shared lesson/review/exam options, answer colors, explanation, progress, next button, and review-save retry state. |
| `conceptList` | `screens/Lesson.svelte`: numbered concepts, recap labels, and previously learned references. |
| `loadFile` | `components/ProjectFiles.svelte`: same native file endpoint, first-file load, file selection, request-version guard, escaped numbered code lines, keyboard scrolling, empty files, and 404-specific message. |
| `review`, `renderReview` | `screens/Review.svelte`: loading, empty state, question context, progress, completion, and another due set. When a session finishes, the title-bar Review count is lowered and then reloaded from `GET api/review`. |
| `doItem` | `screens/Review.svelte`: explanation recall and code recall from starter, text/code editor, Tab insertion, checking/skipping, feedback/output, and next question. |
| `reviewCheck` | `screens/Review.svelte`: `POST api/review/answer`, pending guard, answer text, skip, marking, and error state. |
| `reviewAnswer`, `saveReviewAnswer`, `reviewNext` | `screens/Review.svelte`: MCQ answer recording, save retry, block advancement on failed/pending save, session scoring, and keyboard A–letter/1–number/Enter support. |
| `closeSwitcher`, `toggleSwitcher` | `components/TopBar.svelte`: other active unfinished roadmaps, loading/error/empty states, initial focus, outside click, Escape, Tab, arrows, Home, and End. |
| `renderExercise` | `components/Exercise.svelte`: exercise title/prompt, language, saved code/starter, Tab insertion, run/tests/reset, output/result styling, missing-runtime installation, install errors, and tests disclosure. |
| `answer`, `quizNext` | `components/Practice.svelte`: answer guards/scoring, mistake recording, question navigation/focus, and quiz-result save. |
| `runCode`, `installLanguage` | `lib/state.svelte.ts`: original run/install endpoints, busy guards, saved code, run/test modes, failure counts, package-install status, and rerun after installation. |
| `settings` | `screens/Settings.svelte`: appearance segments, a status card for the saved AI service (Codex = “nothing to set up”), and the service form (provider presets, base URL, model, small model, saved-key semantics, save notice) inside “Use a different AI service”, which starts open when the saved service is not Codex; usage. |
| `usageHtml` | `components/Usage.svelte`: total/input/output tokens, request count, start date, estimate note, and reset. |
| `search` | `screens/Search.svelte`, `components/TopBar.svelte`: encoded search route, query input, lesson/note/chat/idea results, snippets, empty state, and open-ideas results. |
| `mistakes` | `screens/Mistakes.svelte`: open/fixed questions, picked/correct answers, explanations, failed coding exercises, counts, and lesson links. |
| `openIdeas`, `closeIdeas`, `renderIdeas`, `saveIdeas` | `lib/state.svelte.ts`, `components/Ideas.svelte`: lazy load, panel, add/edit/delete, confirmation, 4,000-character cap, 400 ms save, focus, Escape, and close. |
| `seekVideo` | `screens/Lesson.svelte`, `components/Videos.svelte`, `components/Tutor.svelte`: escaped tutor timestamp buttons switch to Watch, select the video, start at the timestamp, and scroll to the player. |
| `route` | `lib/state.svelte.ts`, `App.svelte`: same hash routes, stale-response guard, flushed progress, loading/error screens, course reuse, and component lifecycle cleanup. Sets loading before awaiting saves so lesson-to-lesson navigation cannot redirect through an intermediate empty-current state. |

## Other original helpers and event-only features

| Original helper/handler | Current location |
| --- | --- |
| `$`, `app` | Svelte bindings/components replace the global DOM selector and imperative root renderer. DOM queries remain only for focus, scroll, copy fallback, and safe rendered timestamp buttons. |
| `esc`, `fmt`, `fmtTutor`, `enc`, `href`, `icon`, `clock` | `lib/format.ts`; formatter and SVG components retain the original output. |
| `S`, `all`, `find`, `keyFor`, `chatKey` | `c.state`, `lib/progress.ts::lessons`, array lookups, and `lib/state.svelte.ts::keyFor`. |
| `checked`, `passed`, `status`, `stageLessons`, `examPool`, `examPassed`, `examReady`, `locked` | `lib/progress.ts`; completion, already-started access, short-pool exception, and exam thresholds are preserved. |
| `CHIPS`, `ICONS`, `LANGUAGES`, `languageName`, `BASES` | `lib/format.ts`, `components/Tutor.svelte`, `screens/Settings.svelte`. All original chip actions, languages, and service presets remain. |
| `loadingArt` | The original final CSS hides this artwork. Loading text/panels remain; invisible SVG markup is omitted. |
| `runningArt` | Original animated SVG in `components/Exercise.svelte`; reduced motion still disables it. |
| `videoSearchForm` | `components/Videos.svelte`: custom query, disabled/busy button, inline error, append/select newly found video. |
| Import | `screens/Library.svelte`: same native `api/import` endpoint, busy state, cancel behavior, refresh. |
| Export/template/summary | `screens/Overview.svelte`: flush pending saves, same endpoints and template flag, path toast/error. |
| Project work-folder connect/disconnect | `screens/Overview.svelte`: same `/work` endpoint, clear flag, and folder label (handles both slash styles). |
| Change-my-roadmap form | `screens/Overview.svelte`, `lib/state.svelte.ts::adaptNow`: 600-character request, pending state, retained failed input, and progress-preserving revision. |
| Checkbox/input listeners | Local Svelte handlers for needs, lesson checks, notes, code, and ideas, with the original save debounce. |
| Copy search phrase | `screens/Lesson.svelte`: standard clipboard API, original text-area copy fallback, and notice. |
| Confirmed destructive actions | `components/LibraryCard.svelte`, `components/Ideas.svelte`, `components/Exercise.svelte`: deletion/reset confirmation text retained. Bulk phase writing also retains its token-use confirmation. |
| Lesson celebration burst | The final source CSS uses `.burst{display:none}`. Visible celebration text and next-step links remain; invisible confetti nodes/timer are omitted. |
| `windowBridge`, `window.roadmapMaximised`, `empty` | Replaced by Tauri window methods, resize notifications, and native drag regions in `components/TopBar.svelte`. |
| Transcripts | The source HTML contains no transcript viewer or transcript API call. Transcript retrieval is the backend tutor’s `read_video_transcript` tool; the same chat body and video timestamp controls preserve that interaction. No new transcript screen was invented. |

## Style parity

Superseded on 2026-11-11 by the redesign described in `DESIGN.md`. `src/app.css` no longer copies the old `index.html` style block, and the byte-equality test was removed. Moved, not removed: roadmap export/template/summary, adjust, change, connect folder and write-every-lesson are in the roadmap page’s “More” menu; the lesson step rail is a sticky stepper above the step; Mistakes and Import are secondary actions on the home page; a Review link with the due count is in the title bar.

Part 2 (2026-11-11): in the app’s own engine (offscreen WebKitGTK 4.1) at **1366 × 768**, the whole creation form fits without scrolling in the build variant, the project variant (chosen folder and privacy text) and with a build error shown. The AI service form moved into “Use a different AI service” on the Settings page.

## Verification

- `npm install` completed using Node 24.18.0 and npm 11.16.0; `package-lock.json` is included.
- `npm run check`: **0 errors, 0 warnings**.
- `npm run build`: successful Vite production build.
- `npm test`: three passing checks covering the native transport/error/stream contract, escaped rendering, and completion/stage/exam edge cases.
- `npm run dev`: started on strict port 5173. A temporary browser-only invoke mock exercised creation/placement/project selection, lesson-to-lesson routing, videos/embeds/chapters/tutor seeking, concepts/files/escaping, notes persistence, streaming chat, quizzes, code tests, completion, stage locks/exams/unlocks, review save failure/retry, all three review modes, settings/theme/key clearing, usage reset, search, ideas persistence, mistakes, library/continue cards, switcher focus/Escape, and opener calls. **No page runtime errors were recorded.**
- The temporary mock was deleted, the preview tab closed, and the dev process stopped. Port 5173 was checked for a remaining listener.
- `npm audit`: **three low-severity development dependency findings** in `@sveltejs/vite-plugin-svelte` / inspector / `deepmerge`, with no available npm fix. There were no moderate/high/critical findings. The affected package merges trusted build configuration, not user/AI runtime input, and is absent from the production application bundle.

No active source feature remains unported. The intentionally omitted code was inactive in the final source: hidden loading SVGs/confetti and the unused old SVG-route measurement listener. Live native dialogs, actual AI services, real package installation/execution, YouTube playback/network behavior, and Windows/macOS native window interaction were not integration-tested; their fixed IPC contracts and UI states were ported and mocked. Backend and Tauri configuration verification belongs to the concurrent shell work.

Pages now use a 1440px maximum with existing modest margins; reading copy remains constrained. The lesson composer grows from two to six lines, uses an arrow-only send control and a session-cached native model picker. Settings offers the same models with manual entry. Notes offers rewrite, elapsed waiting and Undo. Tutor requests include the selected video.
