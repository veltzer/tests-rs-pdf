# PDF-engine recommendation for rsslide

**TL;DR:** use **krilla** + **krilla-svg** with `SvgSettings { filter_scale: 2.0, ..default() }` and a minimal `fontdb`.

## Configuration

```rust
use std::sync::Arc;
use fontdb::Database;
use krilla_svg::SvgSettings;

fn build_fontdb() -> Arc<Database> {
    let mut db = Database::new();
    db.load_font_file("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf")
        .expect("missing DejaVuSans.ttf");
    db.load_font_file("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf")
        .expect("missing DejaVuSansMono.ttf");
    db.set_sans_serif_family("DejaVu Sans");
    db.set_monospace_family("DejaVu Sans Mono");
    db.set_serif_family("DejaVu Sans");
    Arc::new(db)
}

let svg_settings = SvgSettings {
    filter_scale: 2.0,
    ..SvgSettings::default()
};

let opts = usvg::Options {
    fontdb: fontdb.clone(),
    font_family: "sans-serif".into(),
    ..Default::default()
};
```

Usage (one page per slide):

```rust
let mut doc = krilla::Document::new();
let size = krilla::geom::Size::from_wh(w, h).unwrap();
let mut page = doc.start_page_with(krilla::page::PageSettings::new(size));
let mut surface = page.surface();
surface.draw_svg(&tree, size, svg_settings);
surface.finish();
page.finish();
let pdf_bytes = doc.finish().unwrap();
```

## Why

| option | verdict |
|---|---|
| `printpdf` 0.7 / 0.9 | ❌ emits structurally-broken PDFs on any SVG using gradients, `<filter>`, CSS vars, or markers — poppler reports malformed XObjects and bad color spaces. Root cause why rsslide's SVGs rendered blank. |
| `svg2pdf` | ✅ correct, fast (~70 ms/SVG), but single-SVG → single-PDF only. No multi-page, no mixed content. rsslide needs title + bullets + code + SVG on one page. |
| `rsvg-convert` (external) | ✅ correct, fast, but CLI dependency and no multi-page mixed content. |
| `marp` / `chrome-headless` | ✅ correct, but headless-browser path (~300–1400 ms per PDF) and external-tool dependencies. |
| **`krilla` + `krilla-svg`** | ✅ correct, full multi-page document model, pure-Rust, same SVG engine as `svg2pdf` underneath. |

## Perf tuning: `filter_scale`

`SvgSettings.filter_scale` controls the pixel-density multiplier used when
rasterising SVG `<filter>` effects (e.g. `feDropShadow`). Higher = crisper
shadows, larger PDF, slower.

Measured on `svg/courses/networking/networking-basics/01_tcp_ip/the_tcp_ip_protocol_stack.svg`
(heavy drop-shadows on four rects):

| filter_scale | wall_ms | pdf_bytes | shadow quality |
|---:|---:|---:|---|
| 4.0 (default) | 474 | 140 914 | over-crisp, wastes bytes |
| **2.0** | **125** | **60 906** | indistinguishable at slide distance |
| 1.0 | 30 | 33 653 | slightly soft at ≥200% zoom |

**2.0 is the sweet spot** — ~5× faster than default, half the file size, no
visible quality loss.

## Fonts

`fontdb::Database::load_system_fonts()` walks every font directory on the
system (~1300 faces on a typical Linux desktop) and costs ~1 s cold. For
rsslide's SVGs (which only reference `Arial, sans-serif` and
`Courier New, monospace`) a 2-face DB works identically once the generic
family aliases are set. Render time is not sensitive to DB size, but startup
is.

Two gotchas:

1. `usvg::Options.font_family` defaults to `"Times New Roman"`. If the DB
   doesn't contain that family either, text is silently dropped. Always set
   the fallback to a family that exists in the DB, or to `"sans-serif"`
   (which uses the alias).
2. usvg resolves `Arial` → `sans-serif` (generic) → whatever
   `db.set_sans_serif_family(...)` says. Set all three aliases
   (`sans-serif`, `monospace`, `serif`) to loaded families.

## Dependencies

```toml
[dependencies]
krilla = "0.7"
krilla-svg = "0.7"
usvg = "0.45"
fontdb = "0.23"
```

## Reference benchmark

One SVG (`the_tcp_ip_protocol_stack.svg`, 4 gradient boxes with drop-shadows):

| tool | wall_ms | pdf_bytes |
|---|---:|---:|
| krilla small/fs1 | 30 | 33 653 |
| svg2pdf (empty db) | 63 | 22 941 |
| svg2pdf (all fonts) | 69 | 38 729 |
| rsvg-convert | 75 | 38 056 |
| printpdf 0.9 | 96 | 77 325 ⚠ malformed |
| **krilla small/fs2** | **125** | **60 906** |
| chrome-headless | 335 | 61 548 |
| krilla small/fs4 | 474 | 140 914 |
| krilla empty/fs4 | 490 | 127 950 ⚠ text dropped |
| krilla all/fs4 | 512 | 143 971 |
| marp | 1 398 | 98 344 |

Harness: `./run.py` · Viewer: `./viewer.py && xdg-open out/viewer.html`.
