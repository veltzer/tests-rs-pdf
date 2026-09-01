# tests-rs-pdf

A bake-off of Rust PDF engines, done to choose the engine for `rsslide`.

The `compare` binary renders one SVG through krilla, svg2pdf, pdf-writer and
printpdf (plus external converters) into an output directory so the results
can be compared side by side. Sample inputs live in `samples/`.
`RECOMMENDATION.md` records the conclusion: krilla plus krilla-svg.

```bash
cargo run --release -- samples/tcp_three_way_handshake.svg out
```
