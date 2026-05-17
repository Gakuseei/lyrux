# Changelog

All notable changes to Lyrux are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
