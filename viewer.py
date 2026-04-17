#!/usr/bin/env python
"""Generate a static HTML viewer that shows each PDF in ./out/ directly,
via iframes. No rasterization. Navigate with Left/Right arrows or buttons.

    ./viewer.py && xdg-open out/viewer.html
"""
from __future__ import annotations

import html
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
OUT_DIR = ROOT / "out"


def main() -> int:
    pdfs = sorted(OUT_DIR.glob("*.pdf"))
    if not pdfs:
        print(f"no PDFs in {OUT_DIR}; run ./run.py first", file=sys.stderr)
        return 2

    html_path = OUT_DIR / "viewer.html"
    html_path.write_text(render(pdfs), encoding="utf-8")
    print(f"Wrote {html_path} ({len(pdfs)} PDFs)")
    return 0


def render(pdfs: list[Path]) -> str:
    frames = []
    for pdf in pdfs:
        name = html.escape(pdf.name)
        size = pdf.stat().st_size
        frames.append(
            f'<section class="slide">'
            f'<header>{name} <small>({size:,} bytes)</small></header>'
            f'<iframe src="{name}" title="{name}"></iframe>'
            f"</section>"
        )
    frames_html = "\n".join(frames)

    return f"""<!doctype html>
<meta charset="utf-8">
<title>PDF bake-off viewer</title>
<style>
  html, body {{ margin: 0; height: 100%; font-family: system-ui, sans-serif;
                background: #222; color: #eee; overflow: hidden; }}
  .slide {{ display: none; flex-direction: column; height: 100vh; padding: 0; }}
  .slide.active {{ display: flex; }}
  .slide header {{ padding: 0.5rem 1rem; background: #111; font-size: 1rem; }}
  .slide header small {{ color: #aaa; margin-left: 0.5rem; }}
  .slide iframe {{ flex: 1; width: 100%; border: 0; background: white; }}
  nav {{ position: fixed; bottom: 1rem; left: 50%; transform: translateX(-50%);
         background: #111cc; padding: 0.4rem 0.8rem; border-radius: 6px;
         box-shadow: 0 4px 20px #0008; z-index: 10; }}
  nav button {{ background: #333; color: #eee; border: 1px solid #555;
                padding: 0.3rem 0.7rem; border-radius: 4px; cursor: pointer; }}
  nav button:hover {{ background: #444; }}
  nav #pos {{ margin: 0 0.8rem; }}
</style>

{frames_html}

<nav>
  <button id="prev">← Prev</button>
  <span id="pos"></span>
  <button id="next">Next →</button>
</nav>

<script>
  const slides = document.querySelectorAll('.slide');
  const pos = document.getElementById('pos');
  let i = 0;
  function show(n) {{
    i = (n + slides.length) % slides.length;
    slides.forEach((s, k) => s.classList.toggle('active', k === i));
    pos.textContent = `${{i + 1}} / ${{slides.length}}`;
  }}
  document.getElementById('prev').addEventListener('click', () => show(i - 1));
  document.getElementById('next').addEventListener('click', () => show(i + 1));
  document.addEventListener('keydown', e => {{
    if (e.target.tagName === 'IFRAME') return;
    if (e.key === 'ArrowRight') show(i + 1);
    else if (e.key === 'ArrowLeft') show(i - 1);
  }});
  show(0);
</script>
"""


if __name__ == "__main__":
    raise SystemExit(main())
