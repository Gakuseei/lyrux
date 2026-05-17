# Changelog

All notable changes to Lyrux are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.1] - 2026-05-17

Editor polish: theme, minimap, sticky scroll, image viewer, drag-and-drop rebuild.

### Added
- Minimap viewport indicator — translucent slice on the minimap tracks the visible buffer range; updates on scroll, edit, resize. Native click-to-jump preserved.
- Sticky scroll header is now clickable: click jumps the view to that header line, places the cursor there, focuses the editor. Pointer cursor + hover highlight.
- Image viewer fits oversized images to the viewport on open (Ctrl+0 re-fits, Ctrl+1 = 100%, Ctrl+wheel / Ctrl+/- = manual zoom and disables auto-fit).
- DnD: cursor-preview icon falls back to a guaranteed-symbolic icon when the resolved name is missing — no more broken-icon during drag.
- DnD: pane drop overlay shows four corner indicators + a centered "Open here as a new tab" label, with a dashed accent border.

### Changed
- `lyrux-grey` theme palette desaturated to a true grey identity — keyword/type/function/string/number/accent now grey-tinted variants that blend into the `@window_bg_color` chrome instead of fighting it.
- `lyrux-grey` line-numbers gutter background equals the editor background — gutter dissolves into the surrounding pane.
- Minimap is now overlaid (right-aligned, `halign=End`) on top of the editor's scrolled view instead of sitting in a horizontal row. Pane width no longer creeps when an editor tab opens.
- File drops on a terminal pane now open a new editor tab in that pane (previously: pasted the path as text into the terminal).
- File drops on an open editor view now open a new tab in the pane (previously: GTK4 TextView pasted file contents into the active buffer).
- Drop overlay copy: "Open here as a new tab" / "Open as a new tab".

### Fixed
- Hidden the unused line-marks gutter that rendered as a thin dark column to the left of line numbers across all themes.
- Editor font CSS leak into the minimap widget — class scoped to `.lyrux-editor-buffer`, Map widget unaffected.

## [0.6.0] - 2026-05-17

Vim mode + perf polish + Round-2 audit fix loop.

### Added
- Real Vim mode via native input-method context (Settings → Editor → Vim mode). `:w` saves, `:q` closes, `:wq` save+close. Other vim commands handled natively (search /pattern, replace :s/x/y/g, motion).
- Status bar shows vim mode state (NORMAL / INSERT / VISUAL / `:cmd`) when vim is on.
- 13 editor commands exposed in shortcut config (Goto-Line, Find, Replace, Toggle-Comment, Duplicate, Delete, Move, Select-Next) — now visible and remappable in Keybinds editor.
- DnD: drag-source supports MOVE in addition to COPY.

### Changed
- Quick-open file walker now runs on a worker thread (cancel-on-new-query); no UI freeze on large repos.
- Settings disk-save debounced 200ms (was: write per change).
- Editor font CSS now uses a single global provider (was: per-tab provider leaked on tab close).
- Sort menu shows active radio marker.
- Inline-rename popover for files anchors to the selected row instead of the panel header.
- Dirty-comparison adds char-count pre-check for cheap fast-path (was: full string fetch on every keystroke).

### Fixed
- `limux-perf:` log prefix replaced with `lyrux-perf:` per logging rule.
- Remaining user-facing "Limux" mentions rebranded to "Lyrux".
- Recent-files MRU now pushes only on successful file open (was: pollutes with binary / not-found / too-large paths).
- CHANGELOG honesty: clarified Ctrl+L still bound in browser scope.
- ThemeMode default switched from System to Manual to preserve upgraded user theme choices.

### Performance
- Dirty-compare per keystroke: 1 MB file = 0 alloc when char_count differs (fast path).
- 50KB JS heap saved per tab via single global CssProvider.

## [0.5.0] - 2026-05-17

Editor polish v2 + file manager polish + theme system + snippets. 43 commits.

### Added
- New "Lyrux Grey" editor theme matching app chrome — now default.
- Bundled Lilex font as new default — denser typography.
- Native completion + 16 bundled snippet sets (Rust, Python, JS, TS, C, HTML, CSS, Markdown, etc.).
- Minimap right-edge code overview (toggleable).
- Recent Files section in Ctrl+P quick-open.
- Reopen Closed Tab via Ctrl+Alt+T.
- Pin/Unpin tab keyboard shortcut Ctrl+Alt+P.
- Horizontal scroll for tab strip when many tabs.
- Find/Replace bar full rebuild: match count, prev/next, case/word/regex toggles, regex-error feedback, smart-case, auto-fill from selection, Ctrl+Alt+Enter Replace All.
- Word-wrap toggle via command palette + status bar click target.
- File manager: inline-rename popover for new file/folder/rename.
- File manager: previously-stubbed Open-in-Terminal, Copy Path, Copy Relative Path, Expand All now functional.
- File manager: keyboard accels (Delete, F2, Ctrl+C/X/V/D/N/Shift+N, F5, Enter, Ctrl+Enter).
- File manager: per-extension file-type icons (code, image, archive, pdf, audio, video).
- File manager: four sort modes (name asc/desc, modified date, size).
- File manager: size + modified-time columns (toggleable).
- DnD: real drag-source on file_panel rows + cursor file-icon + per-pane drop target + blue-tint hover overlay + "Open here" label.
- User themes — drop `.xml` into `~/.config/lyrux/themes/`.
- System color-scheme auto-sync (light/dark theme follows system).
- Save All Ctrl+Alt+S.
- Editor: goto-line moved from Ctrl+L to Ctrl+G (standard); the former Ctrl+L still focuses browser address bar in browser tabs.
- F3 / Shift+F3 find-next / prev (standard).
- Ctrl+` toggles or focuses terminal.
- Ctrl+, opens Settings; Ctrl+Shift+, opens Keybinds editor.
- Command palette accels auto-derive from current shortcut config.
- Vim mode toggle in Settings — real `VimIMContext` integration with status-bar mode display and `:w`/`:q`/`:wq` routed to native save/close actions.

### Changed
- Default editor theme: "Lyrux Grey" (was "Lyrux Dark").
- Default font: "Lilex" (was "JetBrains Mono"; JBM still bundled as fallback).
- Default font-size: 12pt (was 13pt) + line-height 1.3 (was 1.5) for denser display.
- Default `wrap_lines: false` (developer convention; toggleable in Settings).
- File extension → language ID map fixed (`.jsx/.tsx` finally render with syntax highlighting; `.py` uses python3 schema; `.h` uses chdr).
- Dirty marker compares full buffer text against saved state (clears on revert).
- Default `theme_mode: Manual` for new installs; existing user theme choices preserved on upgrade. Opt-in to system-sync via Settings.

### Fixed
- 3 critical Rc-cycle leaks (FileMonitor, sticky-scroll, status-bar) — same family as the 2026-05-15 memory-leak hunt.
- Dirty marker no longer stays after buffer reverts to saved content.
- Close-tab button on "file deleted on disk" banner actually closes the tab.
- Find-bar regex-error now shows red border with tooltip.

### Performance
- Settings broadcast debounced 50ms (was unthrottled).
- Snippet/scheme/language registration gated by `OnceLock`.
- Find-in-Files result lines capped at 240 chars.

## [0.4.0] - 2026-05-17

Skipped 0.3 — editor polish and productivity sprints both landed in one cycle.

### Added
- Quick-open file picker via Ctrl+P — fuzzy workspace search.
- Command Palette via Ctrl+Shift+P.
- Find in Files via Ctrl+Shift+F — ripgrep-backed.
- Per-tab status bar: line:col, language, indent, encoding, EOL.
- Match-highlight on cursor word across the visible buffer.
- Sticky scroll for function/class/section headers (Rust, TS, JS, Python, Markdown, Go).
- Indent guides (BackgroundPattern Grid).
- `.editorconfig` support: `indent_style`, `indent_size`, `tab_width`, `insert_final_newline`, `trim_trailing_whitespace`.
- Auto-pair brackets and quotes (`""`, `''`, `` `` ``, `()`, `[]`, `{}`, `<>` in markup) with smart-skip and smart-delete.
- Comment + list continuation on Enter (`//`, `#`, `-`, `*`, numbered `1.` auto-increment, JSDoc `*`).
- Strip trailing whitespace + ensure final newline on save (configurable).
- Save-As dialog for untitled tabs on Ctrl+S.
- Save-As warns before saving text into an image filename.
- In-tab error banner on save failure.
- Per-TabKind icon in tab strip (editor / terminal / browser / image / keybinds visually distinct).
- FontDialogButton replaces freetext font entry — monospace-filtered.
- Bundled JetBrains Mono as default editor font (OFL-1.1, shipped inside the AppImage).
- New shortcuts: Ctrl+P (Quick-open), Ctrl+Shift+P (Command palette), Ctrl+Shift+F (Find in Files), Ctrl+Shift+D (duplicate line).

### Changed
- Soft-wrap default is now ON (was OFF).
- Ctrl+D rebound to select-next-occurrence (was duplicate-line); duplicate-line moved to Ctrl+Shift+D.
- Default font: `JetBrains Mono` (was `monospace`) with a multi-font fallback chain.
- Current-line color in Lyrux Dark bumped for visibility (`#15151A` → `#1A1B22`).
- Gutter background distinguished from buffer background across all bundled schemes.
- Buffer rendering: 6 px / 12 px padding + line-height 1.5.
- Scheme `def:*` style coverage expanded — Markdown headings, bold, italic, links, lists, and code fences now render with color across all 10 bundled schemes.
- Watcher auto-reload preserves cursor + scroll position.
- File-change banner clears stale tab marker on auto-reload (suppress flag).
- Scheme + language registration gated by `OnceLock` — no disk thrash per editor open or settings change.
- Surface-find-hide shortcut moved from Ctrl+Shift+F to Ctrl+Shift+Alt+F (freed Ctrl+Shift+F for Find-in-Files).

### Fixed
- Swap-file leak in workspace-switch — previous swap file is discarded before writing the new one.
- Untitled tab Ctrl+S no longer silent no-op (opens FileDialog).
- External file-change race that left dirty-marker stale after auto-reload.
- Documented shortcut for editor-toggle-current-pane is now Ctrl+Shift+E (CHANGELOG previously said Ctrl+E).

### Performance
- Editor pane idle: ~70 MB target with 10 open tabs (heaptrack-measured separately).

### Notes
- Full multi-cursor (Alt+click add-cursor, Ctrl+Shift+L select-all-occurrences) deferred — sourceview5 0.11 has no native multi-cursor API. Ctrl+D wraps to the next single occurrence for now.
- HTML / XML block-comment toggle (Ctrl+/) not yet — only `//` and `#` languages currently.
- Heaptrack measurement of the polish sprint still TODO post-build.

## [0.2.0] — 2026-05-16

### Added
- Code editor pane via GtkSourceView 5. Open via editor icon in pane header (Ctrl+Shift+E), via file-panel double-click on text files, or by DnD onto a pane.
- Image viewer pane for PNG/JPG/WebP/GIF/SVG/BMP with Ctrl+scroll zoom and Ctrl+0 fit-to-window.
- 10 bundled style schemes (Lyrux Dark/Light, Catppuccin Latte/Frappé/Macchiato/Mocha, Tokyo Night / Storm, One Dark/Light).
- Editor settings panel: theme, font, font size, tab width, line-numbers, whitespace, wrap, auto-indent, current-line, bracket-match.
- Workspace persistence: editor tabs (path + cursor + scroll + dirty buffer via swap file) and image-viewer tabs (path + zoom) round-trip across workspace switches.
- External file-change detection: clean buffers auto-reload, dirty buffers show a reload banner.
- Editor-scoped keybinds: Ctrl+S, Ctrl+F, Ctrl+H, Ctrl+G, Ctrl+L, Ctrl+/, Ctrl+D select-next-occurrence, Ctrl+Shift+D duplicate-line, Ctrl+Shift+K delete-line, Alt+Up/Down move-line, Ctrl+W close-tab.

### Changed
- File-panel single-click on a file now routes to editor (text), image viewer (images), or no-op (other binaries) instead of being inert.

### Performance
- 10 editor tabs idle: ~70 MB target, measure post-ship (heaptrack-measured).

### Notes
- Vim mode not in MVP — shown in settings as "coming soon".
- Multi-cursor cursors not yet — Ctrl+D currently moves the primary selection to the next occurrence (no secondary cursors). Native multi-cursor in sourceview5 0.11 is unavailable; deferred. Alt+Click add-cursor and Ctrl+Shift+L select-all-occurrences also deferred.
- Cross-file search (Ctrl+Shift+F), diff view, hex viewer: tracked as separate issues.
