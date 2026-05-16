# Changelog

All notable changes to Lyrux are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
