# Third-Party Attributions

Lyrux is licensed under **GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)**.
See [`LICENSE`](LICENSE) for the full text. This file documents the provenance of upstream code
Lyrux is derived from, plus every third-party library bundled in shipped artifacts.

---

## Upstream Provenance

Lyrux is a derivative work in a two-link chain:

```
manaflow-ai/cmux  (AGPL-3.0 at fork snapshot, Manaflow Inc., 2024-present)
        |
        v
am-will/limux     (forked 2026-03-19, no LICENSE file published)
        |
        v
Gakuseei/lyrux    (this project, 2026-present)
```

### Manaflow cmux (upstream root)

- **Project:** cmux — terminal multiplexer
- **Source:** https://github.com/manaflow-ai/cmux
- **License at the moment of fork:** AGPL-3.0 bare (cmux adopted AGPL-3.0 on 2026-02-14 and was bare AGPL-3.0 from then through 2026-03-23). cmux later relicensed to GPL-3.0-or-later on 2026-03-30, but Lyrux's derivative chain captured the AGPL-3.0 snapshot — **AGPL terms bind Lyrux**, regardless of the later upstream relicense.
- **Copyright:** Copyright (c) 2024-present Manaflow, Inc.
- **Commercial alternative:** Manaflow offers a commercial license on the cmux upstream for organizations that cannot comply with AGPL/GPL. Contact `founders@manaflow.com`. This option applies to the cmux upstream only; Lyrux contributors do not grant any separate commercial license.

### am-will Limux (intermediate)

- **Project:** Limux — Linux port of cmux
- **Source:** https://github.com/am-will/limux
- **License at upstream:** None published. Limux did not ship a LICENSE file. Because Limux is itself a derivative of AGPL-3.0 cmux, Limux inherits AGPL-3.0 by descent — Lyrux treats the Limux baseline as AGPL-3.0.
- **Copyright:** Copyright (c) 2024 am-will and Limux contributors.

### Gakuseei Lyrux (this project)

- **Source:** https://github.com/Gakuseei/lyrux (mirror: GitLab)
- **License:** AGPL-3.0-or-later (combined-work obligation inherited from cmux).
- **Copyright:** Copyright (c) 2026 Gakuseei and Lyrux contributors.

---

## Bundled libghostty

- **Project:** Ghostty — embedded terminal renderer (`libghostty.so`)
- **License:** MIT License
- **Copyright:** Copyright (c) 2024 Mitchell Hashimoto, Ghostty contributors
- **Fork shipped:** https://github.com/Gakuseei/ghostty
- **Upstream:** https://github.com/ghostty-org/ghostty
- **Vendored at:** `ghostty/` (git submodule)
- **License text in source tree:** `ghostty/LICENSE`
- **License text in distributed artifacts:** `usr/share/doc/lyrux/LICENSE-ghostty` (AppImage, .deb, .tar.gz)

Lyrux dynamically links `libghostty.so` and ships the binary inside the AppImage and the
.deb/.tar.gz packages. Per MIT terms, the upstream copyright and permission notice are
preserved verbatim in `ghostty/LICENSE` and re-shipped inside every artifact.

---

## Bundled GTK4 / GLib / Cairo / Pango / GtkSourceView 5

When Lyrux is distributed as an AppImage, `libgtksourceview-5.so.0` is bundled alongside the
binary; GTK4, GLib, Cairo, Pango, HarfBuzz, FreeType, libadwaita, WebKitGTK, GStreamer and
related libraries are resolved against the host system and are **not bundled**.

- **License:** LGPL-2.1-or-later (GtkSourceView, GLib, Pango, GTK4); LGPL-2.1-or-later
  for GTK4 (with FSF clarifications); LGPL/MPL for Cairo; MIT-style permissive for
  HarfBuzz; LGPL-2.1 / GPLv2 / FTL options for FreeType.
- **Source availability (LGPL §4/§6):**
  - GtkSourceView: https://gitlab.gnome.org/GNOME/gtksourceview
  - GLib: https://gitlab.gnome.org/GNOME/glib
  - Pango: https://gitlab.gnome.org/GNOME/pango
  - Cairo: https://gitlab.freedesktop.org/cairo/cairo
  - GTK4: https://gitlab.gnome.org/GNOME/gtk
  - libadwaita: https://gitlab.gnome.org/GNOME/libadwaita
- **Written offer:** Source for any bundled LGPL library may be obtained from the upstream
  URL above, or by writing to the Lyrux maintainer (see repository `LICENSE` for contact).
- **Re-linking:** All bundled LGPL libraries are dynamically loaded via `LD_LIBRARY_PATH`
  set in `AppRun`. End users may replace the bundled `.so` files with a modified version
  by editing the AppImage payload, satisfying LGPL §6's re-linking permission.

## Bundled FreeType

- **License:** FreeType Project License (FTL) or GPLv2 at user option
- **Source:** https://www.freetype.org/
- Shipped only as a system library dependency (not bundled in AppImage).

## Bundled HarfBuzz

- **License:** MIT-style (HarfBuzz "Old MIT")
- **Source:** https://github.com/harfbuzz/harfbuzz
- Shipped only as a system library dependency (not bundled in AppImage).

---

## Rust Crate Dependencies (transitive)

Lyrux statically links 222 third-party Rust crates. Aggregate license breakdown
(from `cargo license`):

| License | Crate count |
|---|---:|
| Apache-2.0 OR MIT | 130 |
| MIT | 61 |
| Apache-2.0 OR Apache-2.0 WITH LLVM-exception OR MIT | 14 |
| MIT OR Unlicense | 7 |
| ISC | 2 |
| Apache-2.0 (pure) | 2 |
| Zlib | 1 |
| MPL-2.0 | 1 |
| CC0-1.0 | 1 |
| Apache-2.0 WITH LLVM-exception | 1 |
| (Apache-2.0 OR MIT) AND Unicode-3.0 | 1 |
| Apache-2.0 OR LGPL-2.1-or-later OR MIT | 1 |

**No GPL or AGPL Rust crates** are pulled in by Lyrux — the AGPL obligation comes from
the cmux derivative chain, not from any Rust dependency.

The full per-crate listing (name, version, authors, repository, license, description) is
generated by `cargo license --tsv` and shipped at `THIRD-PARTY-NOTICES.txt` in the
repository root, and at `usr/share/doc/lyrux/THIRD-PARTY-NOTICES.txt` inside the AppImage,
.deb and .tar.gz artifacts. To regenerate:

```bash
cargo license --tsv > THIRD-PARTY-NOTICES.txt
```

Notable individual crates:

- `option-ext 0.2.0` — MPL-2.0. File-level copyleft. Lyrux does not modify this crate;
  source available at https://crates.io/crates/option-ext.
- `ec4rs 1.2.0`, `shell-quote 0.7.2` — pure Apache-2.0. Neither upstream ships a
  `NOTICE` file, so Apache-2.0 §4(d) notice obligation is not triggered.

---

## Bundled GtkSourceView Style Schemes

- **GtkSourceView Language Specifications** (`rust/limux-host-linux/src/editor/bundled_langs/*.lang`) — Copyright the GNOME / gtksourceview contributors, licensed LGPL-2.1-or-later. Vendored from https://gitlab.gnome.org/GNOME/gtksourceview/-/tree/master/data/language-specs because the Arch `gtksourceview5` package does not ship these data files.

- **Bundled Snippet Files (upstream subset)** (`rust/limux-host-linux/src/editor/bundled_snippets/{c,js,python,rust,xml,shebang,licenses}.snippets`) — Copyright the GNOME / gnome-builder contributors, licensed LGPL-2.1-or-later. Vendored from the gnome-builder project (`src/plugins/snippets/snippets/`) because the Arch `gtksourceview5` package does not ship a default snippet set.

- **Bundled Snippet Files (Lyrux-authored)** (`rust/limux-host-linux/src/editor/bundled_snippets/{typescript,markdown,css,html,toml,yaml,sh,go,json}.snippets`) — Original work, copyright (c) Lyrux contributors, licensed AGPL-3.0-or-later as part of the combined Lyrux work. Hand-authored using the GtkSourceView snippet XML schema.

- **Catppuccin Latte / Frappé / Macchiato / Mocha** — Palette copyright (c) Catppuccin Org, MIT License. Source: https://github.com/catppuccin/gtk. GtkSourceView XML hand-rolled by Lyrux using the documented palette values.

- **Tokyo Night / Tokyo Night Storm** — Palette derived from Tokyo Night by Enkia, MIT License, via https://github.com/folke/tokyonight.nvim. GtkSourceView XML hand-rolled by Lyrux using the documented palette values.

- **One Dark / One Light** — Palette derived from Atom One (MIT License, GitHub Inc.). GtkSourceView XML hand-rolled by Lyrux using the documented palette values.

- **Lyrux Dark / Lyrux Light / Lyrux Grey** — Original work, copyright (c) Lyrux contributors, licensed AGPL-3.0-or-later as part of the combined Lyrux work.

## Bundled Fonts

- **JetBrains Mono** — Copyright 2020 The JetBrains Mono Project Authors, SIL Open Font License 1.1. Source: https://github.com/JetBrains/JetBrainsMono. Bundled in AppImage as a fallback editor font. License text: `rust/limux-host-linux/assets/fonts/JetBrainsMono-LICENSE.txt`.

- **Lilex** — Copyright 2019 The Lilex Project Authors, SIL Open Font License 1.1. Source: https://github.com/mishamyrt/Lilex. Bundled as default editor font in AppImage. License text: `rust/limux-host-linux/assets/fonts/Lilex-LICENSE.txt`.
