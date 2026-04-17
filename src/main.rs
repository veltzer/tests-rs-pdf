use std::path::{Path, PathBuf};
use std::sync::Arc;

use fontdb::Database;
use krilla::Document;
use krilla::geom::Size;
use krilla::page::PageSettings;
use krilla_svg::{SurfaceExt, SvgSettings};
use walkdir::WalkDir;

fn main() {
    let root = std::env::args().nth(1).expect("arg1: svg root");
    let out_dir = PathBuf::from(std::env::args().nth(2).expect("arg2: out dir"));
    std::fs::create_dir_all(&out_dir).unwrap();

    let t_fonts = std::time::Instant::now();
    let mut fontdb = Database::new();
    fontdb.load_system_fonts();
    let fontdb = Arc::new(fontdb);
    eprintln!("fonts loaded in {} ms ({} faces)", t_fonts.elapsed().as_millis(), fontdb.len());

    let mut total = 0;
    let mut parse_err = 0;
    let mut render_err = 0;
    let mut ok = 0;

    use std::io::Write;
    for entry in WalkDir::new(&root) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e != "svg").unwrap_or(true) {
            continue;
        }
        total += 1;
        let t0 = std::time::Instant::now();
        eprint!("[{total}] {} ... ", path.display());
        let _ = std::io::stderr().flush();
        match render_one(path, &out_dir, &fontdb) {
            Ok(_out) => {
                eprintln!("OK ({} ms)", t0.elapsed().as_millis());
                ok += 1;
            }
            Err(Stage::Parse(e)) => {
                eprintln!("PARSE ERR: {e}");
                parse_err += 1;
            }
            Err(Stage::Render(e)) => {
                eprintln!("RENDER ERR: {e}");
                render_err += 1;
            }
        }
    }
    println!(
        "\nTotal: {total}  OK: {ok}  Parse errors: {parse_err}  Render errors: {render_err}"
    );
}

enum Stage {
    Parse(String),
    Render(String),
}

fn render_one(svg_path: &Path, out_dir: &Path, fontdb: &Arc<Database>) -> Result<PathBuf, Stage> {
    let data = std::fs::read(svg_path).map_err(|e| Stage::Parse(format!("read: {e}")))?;

    let opts = usvg::Options {
        fontdb: fontdb.clone(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_data(&data, &opts)
        .map_err(|e| Stage::Parse(format!("usvg: {e}")))?;

    let size = Size::from_wh(tree.size().width(), tree.size().height())
        .ok_or_else(|| Stage::Render(format!("non-positive SVG size")))?;

    let mut document = Document::new();
    let mut page = document.start_page_with(PageSettings::new(size));
    let mut surface = page.surface();
    surface.draw_svg(&tree, size, SvgSettings::default())
        .ok_or_else(|| Stage::Render(format!("draw_svg returned None")))?;
    surface.finish();
    page.finish();

    let pdf = document.finish()
        .map_err(|e| Stage::Render(format!("finish: {e:?}")))?;

    let rel = svg_path.strip_prefix(".").unwrap_or(svg_path);
    let file_name = rel
        .to_string_lossy()
        .replace('/', "__")
        .trim_start_matches('.')
        .trim_start_matches('_')
        .to_string();
    let out_name = file_name.trim_end_matches(".svg").to_string() + ".pdf";
    let out_path = out_dir.join(out_name);
    std::fs::write(&out_path, pdf).map_err(|e| Stage::Render(format!("write: {e}")))?;
    Ok(out_path)
}
