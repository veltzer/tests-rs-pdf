use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

use fontdb::Database;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: compare <input.svg> [out-dir=./out]");
        std::process::exit(2);
    }
    let input = PathBuf::from(&args[1]);
    let out_dir = PathBuf::from(args.get(2).map(String::as_str).unwrap_or("out"));
    std::fs::create_dir_all(&out_dir).unwrap();

    let fontdb = build_fontdb();

    println!("input: {}", input.display());
    println!("out:   {}\n", out_dir.display());

    let mut rows: Vec<Row> = Vec::new();
    rows.push(run("krilla", || bake_krilla(&input, &out_dir, &fontdb)));
    rows.push(run("svg2pdf", || bake_svg2pdf(&input, &out_dir, &fontdb)));
    rows.push(run("printpdf-0.9", || bake_printpdf(&input, &out_dir)));
    rows.push(run("rsvg-convert", || bake_rsvg(&input, &out_dir)));
    rows.push(run("marp", || bake_marp(&input, &out_dir)));
    rows.push(run("chrome-headless", || bake_chrome(&input, &out_dir)));

    println!();
    println!("{:<18} {:>10} {:>12}  {}", "tool", "wall_ms", "pdf_bytes", "status / path");
    println!("{}", "-".repeat(80));
    for r in &rows {
        let bytes = r.bytes.map(|b| b.to_string()).unwrap_or_else(|| "-".into());
        let ms = r.ms.to_string();
        let status = match &r.result {
            Ok(p) => format!("OK  {}", p.display()),
            Err(e) => format!("ERR {e}"),
        };
        println!("{:<18} {:>10} {:>12}  {}", r.name, ms, bytes, status);
    }
}

struct Row {
    name: String,
    ms: u128,
    bytes: Option<u64>,
    result: Result<PathBuf, String>,
}

fn run<F>(name: &str, f: F) -> Row
where
    F: FnOnce() -> Result<PathBuf, String>,
{
    let t0 = Instant::now();
    let result = f();
    let ms = t0.elapsed().as_millis();
    let bytes = result.as_ref().ok().and_then(|p| std::fs::metadata(p).ok().map(|m| m.len()));
    Row { name: name.to_string(), ms, bytes, result }
}

fn build_fontdb() -> Arc<Database> {
    let mut db = Database::new();
    db.load_system_fonts();
    Arc::new(db)
}

fn stem(path: &Path) -> String {
    path.file_stem().unwrap().to_string_lossy().into_owned()
}

// ── krilla ────────────────────────────────────────────────────────────────
fn bake_krilla(input: &Path, out_dir: &Path, fontdb: &Arc<Database>) -> Result<PathBuf, String> {
    use krilla::Document;
    use krilla::geom::Size;
    use krilla::page::PageSettings;
    use krilla_svg::{SurfaceExt, SvgSettings};

    let data = std::fs::read(input).map_err(|e| e.to_string())?;
    let opts = usvg::Options {
        fontdb: fontdb.clone(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_data(&data, &opts).map_err(|e| e.to_string())?;
    let size = Size::from_wh(tree.size().width(), tree.size().height())
        .ok_or_else(|| "non-positive size".to_string())?;

    let mut doc = Document::new();
    let mut page = doc.start_page_with(PageSettings::new(size));
    let mut surface = page.surface();
    surface
        .draw_svg(&tree, size, SvgSettings::default())
        .ok_or_else(|| "draw_svg returned None".to_string())?;
    surface.finish();
    page.finish();

    let pdf = doc.finish().map_err(|e| format!("{e:?}"))?;
    let out = out_dir.join(format!("{}__krilla.pdf", stem(input)));
    std::fs::write(&out, pdf).map_err(|e| e.to_string())?;
    Ok(out)
}

// ── svg2pdf ───────────────────────────────────────────────────────────────
fn bake_svg2pdf(input: &Path, out_dir: &Path, fontdb: &Arc<Database>) -> Result<PathBuf, String> {
    let data = std::fs::read(input).map_err(|e| e.to_string())?;
    let opts = usvg::Options {
        fontdb: fontdb.clone(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_data(&data, &opts).map_err(|e| e.to_string())?;
    let conv = svg2pdf::ConversionOptions::default();
    let page = svg2pdf::PageOptions::default();
    let pdf = svg2pdf::to_pdf(&tree, conv, page).map_err(|e| e.to_string())?;
    let out = out_dir.join(format!("{}__svg2pdf.pdf", stem(input)));
    std::fs::write(&out, pdf).map_err(|e| e.to_string())?;
    Ok(out)
}

// ── printpdf 0.9 ──────────────────────────────────────────────────────────
fn bake_printpdf(input: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    use printpdf::{
        Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Pt, Svg, XObjectTransform,
    };

    let svg_str = std::fs::read_to_string(input).map_err(|e| e.to_string())?;
    let mut doc = PdfDocument::new("compare");
    let mut warnings = Vec::new();
    let svg = Svg::parse(&svg_str, &mut warnings).map_err(|e| format!("{e:?}"))?;
    let xid = doc.add_xobject(&svg);

    // Page sized to roughly the SVG at 72 dpi. The SVG here is viewBox 1280x720.
    let page_w = Mm::from(Pt(1280.0));
    let page_h = Mm::from(Pt(720.0));
    let ops = vec![Op::UseXobject {
        id: xid,
        transform: XObjectTransform {
            translate_x: Some(Pt(0.0)),
            translate_y: Some(Pt(0.0)),
            scale_x: Some(1.0),
            scale_y: Some(1.0),
            ..Default::default()
        },
    }];
    let page = PdfPage::new(page_w, page_h, ops);

    let bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut Vec::new());
    let out = out_dir.join(format!("{}__printpdf.pdf", stem(input)));
    std::fs::write(&out, bytes).map_err(|e| e.to_string())?;
    Ok(out)
}

// ── rsvg-convert ──────────────────────────────────────────────────────────
fn bake_rsvg(input: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    let out = out_dir.join(format!("{}__rsvg.pdf", stem(input)));
    let status = Command::new("rsvg-convert")
        .args(["-f", "pdf", "-o"])
        .arg(&out)
        .arg(input)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("exit {status}"));
    }
    Ok(out)
}

// ── marp ──────────────────────────────────────────────────────────────────
fn bake_marp(input: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    let md_path = out_dir.join(format!("{}__marp.md", stem(input)));
    let abs_svg = std::fs::canonicalize(input).map_err(|e| e.to_string())?;
    let md = format!(
        "---\nmarp: true\n---\n\n![bg contain]({})\n",
        abs_svg.display()
    );
    std::fs::write(&md_path, md).map_err(|e| e.to_string())?;

    let out = out_dir.join(format!("{}__marp.pdf", stem(input)));
    let output = Command::new("marp")
        .args(["--pdf", "--allow-local-files", "-o"])
        .arg(&out)
        .arg(&md_path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "exit {}; stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(out)
}

// ── chrome --headless --print-to-pdf ──────────────────────────────────────
fn bake_chrome(input: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    let html_path = out_dir.join(format!("{}__chrome.html", stem(input)));
    let svg = std::fs::read_to_string(input).map_err(|e| e.to_string())?;
    let html = format!(
        "<!doctype html><meta charset=\"utf-8\"><style>\
html,body{{margin:0;padding:0}}\
svg{{display:block;width:100vw;height:100vh}}\
</style>{svg}"
    );
    std::fs::write(&html_path, html).map_err(|e| e.to_string())?;

    let out = out_dir.join(format!("{}__chrome.pdf", stem(input)));
    let abs_html = std::fs::canonicalize(&html_path).map_err(|e| e.to_string())?;
    let url = format!("file://{}", abs_html.display());
    let status = Command::new("google-chrome")
        .args([
            "--headless=new",
            "--no-sandbox",
            "--disable-gpu",
            &format!("--print-to-pdf={}", out.display()),
            "--no-pdf-header-footer",
            &url,
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("exit {status}"));
    }
    Ok(out)
}
