//! Background LaTeX compilation and preview rendering.
use eframe::egui;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    sync::mpsc::{channel, Receiver, Sender},
    thread,
};

pub struct Job {
    pub id: u64,
    pub tex: String,
    pub photo: Option<PathBuf>,
}

pub struct Built {
    pub pdf: Vec<u8>,
    pub preview: Option<egui::ColorImage>,
    pub multi_page: bool,
}

pub struct Output {
    pub id: u64,
    pub result: Result<Built, Vec<String>>,
}

pub struct Worker {
    pub tx: Sender<Job>,
    pub rx: Receiver<Output>,
}

const NO_ENGINE: &str =
    "No LaTeX engine found. Install tectonic, TeX Live (pdflatex/xelatex) or MiKTeX, or use Export .tex.";

/// First available engine and its arguments.
fn find_engine() -> Option<(&'static str, &'static [&'static str])> {
    const ENGINES: [(&str, &[&str]); 3] = [
        ("tectonic", &["cv.tex"]),
        (
            "pdflatex",
            &["-interaction=nonstopmode", "-halt-on-error", "cv.tex"],
        ),
        (
            "xelatex",
            &["-interaction=nonstopmode", "-halt-on-error", "cv.tex"],
        ),
    ];
    ENGINES
        .into_iter()
        .find(|(exe, _)| Command::new(exe).arg("--version").output().is_ok())
}

/// Turn a LaTeX/tectonic log into short messages that point at the offending source line.
pub fn parse_errors(log: &str, tex: &str) -> Vec<String> {
    let src: Vec<&str> = tex.lines().collect();
    let snippet = |n: usize| -> String {
        let line = src.get(n.wrapping_sub(1)).map(|l| l.trim()).unwrap_or("");
        let short: String = line.chars().take(70).collect();
        format!("line {n}: {short}")
    };
    let lines: Vec<&str> = log.lines().collect();
    let mut out = vec![];
    for (i, l) in lines.iter().enumerate() {
        if let Some(msg) = l.strip_prefix("! ") {
            let n = lines[i + 1..(i + 8).min(lines.len())].iter().find_map(|x| {
                x.strip_prefix("l.")
                    .and_then(|r| r.split_whitespace().next()?.parse::<usize>().ok())
            });
            out.push(match n {
                Some(n) => format!("{} ({})", msg.trim_end_matches('.'), snippet(n)),
                None => msg.to_string(),
            });
        } else if let Some(rest) = l.strip_prefix("error: ") {
            // tectonic: "error: cv.tex:12: message"
            let mut parts = rest.splitn(3, ':');
            match (
                parts.next(),
                parts.next().and_then(|n| n.trim().parse::<usize>().ok()),
                parts.next(),
            ) {
                (Some(_), Some(n), Some(m)) => out.push(format!("{} ({})", m.trim(), snippet(n))),
                _ => out.push(rest.to_string()),
            }
        }
    }
    if out.is_empty() {
        out.push(
            lines
                .iter()
                .rev()
                .take(8)
                .rev()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string(),
        );
    }
    out.dedup();
    out
}

fn unique_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "latex-cv-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Compile `tex` to a PDF. The photo (if any) is copied into the build directory
/// under `crate::latex::photo_file_name`.
pub fn compile(tex: &str, photo: Option<&Path>) -> Result<Vec<u8>, Vec<String>> {
    let (exe, args) = find_engine().ok_or_else(|| vec![NO_ENGINE.to_string()])?;
    let dir = unique_dir();
    let res = (|| {
        fs::create_dir_all(&dir).map_err(|e| vec![format!("Cannot create build dir: {e}")])?;
        fs::write(dir.join("cv.tex"), tex).map_err(|e| vec![format!("Write failed: {e}")])?;
        if let Some(p) = photo {
            let name = crate::latex::photo_file_name(&p.to_string_lossy());
            fs::copy(p, dir.join(name)).map_err(|e| vec![format!("Cannot read photo: {e}")])?;
        }
        let out = Command::new(exe)
            .args(args)
            .current_dir(&dir)
            .output()
            .map_err(|e| vec![format!("Failed to run {exe}: {e}")])?;
        match fs::read(dir.join("cv.pdf")) {
            Ok(pdf) if out.status.success() => Ok(pdf),
            _ => {
                let log = fs::read_to_string(dir.join("cv.log")).unwrap_or_else(|_| {
                    format!(
                        "{}\n{}",
                        String::from_utf8_lossy(&out.stdout),
                        String::from_utf8_lossy(&out.stderr)
                    )
                });
                let mut errs = parse_errors(&log, tex);
                errs.insert(0, format!("{exe} failed"));
                Err(errs)
            }
        }
    })();
    let _ = fs::remove_dir_all(&dir);
    res
}

/// Rasterise page 1 with `pdftoppm` (poppler) when available; also reports whether a page 2 exists.
fn render(pdf: &[u8]) -> (Option<egui::ColorImage>, bool) {
    let dir = unique_dir();
    if fs::create_dir_all(&dir).is_err() || fs::write(dir.join("cv.pdf"), pdf).is_err() {
        return (None, false);
    }
    let page = |n: &str, out: &str| {
        Command::new("pdftoppm")
            .args([
                "-png",
                "-r",
                "110",
                "-f",
                n,
                "-l",
                n,
                "-singlefile",
                "cv.pdf",
                out,
            ])
            .current_dir(&dir)
            .output()
            .is_ok_and(|o| o.status.success())
    };
    let img = if page("1", "p1") {
        image::open(dir.join("p1.png")).ok().map(|i| {
            let rgba = i.to_rgba8();
            egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                rgba.as_raw(),
            )
        })
    } else {
        None
    };
    let multi = page("2", "p2") && dir.join("p2.png").exists();
    let _ = fs::remove_dir_all(&dir);
    (img, multi)
}

/// Start the compile thread. Queued jobs are coalesced so only the newest one is built.
pub fn spawn(ctx: egui::Context) -> Worker {
    let (tx, jobs) = channel::<Job>();
    let (out, rx) = channel::<Output>();
    thread::spawn(move || {
        while let Ok(mut job) = jobs.recv() {
            while let Ok(newer) = jobs.try_recv() {
                job = newer;
            }
            let result = compile(&job.tex, job.photo.as_deref()).map(|pdf| {
                let (preview, multi_page) = render(&pdf);
                Built {
                    pdf,
                    preview,
                    multi_page,
                }
            });
            if out.send(Output { id: job.id, result }).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    });
    Worker { tx, rx }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_pdflatex_error() {
        let log = "blah\n! Undefined control sequence.\nl.3 \\foo\n               bar\n";
        let e = parse_errors(log, "a\nb\n\\foo bar\n");
        assert_eq!(e, vec!["Undefined control sequence (line 3: \\foo bar)"]);
    }
    #[test]
    fn parses_tectonic_error() {
        let e = parse_errors("error: cv.tex:2: Missing $ inserted\n", "a\nbad line\n");
        assert_eq!(e, vec!["Missing $ inserted (line 2: bad line)"]);
    }
    #[test]
    fn falls_back_to_log_tail() {
        assert_eq!(parse_errors("something odd", ""), vec!["something odd"]);
    }
    /// Compiles every template (with a photo and custom design) when a LaTeX engine is installed.
    #[test]
    fn templates_compile_when_engine_present() {
        if find_engine().is_none() {
            return;
        }
        let img = std::env::temp_dir().join("latex_cv_build_test.png");
        image::RgbaImage::from_pixel(60, 90, image::Rgba([200, 50, 50, 255]))
            .save(&img)
            .unwrap();
        for t in 0..crate::latex::TEMPLATES.len() {
            for shape in 0..crate::latex::SHAPES.len() {
                let mut cv = crate::model::Cv {
                    template: t,
                    ..Default::default()
                };
                cv.photo.path = img.display().to_string();
                cv.photo.shape = shape;
                cv.design.font = 3;
                cv.design.heading = 1;
                let tex = crate::latex::generate_with(&cv, Some("photo.png"));
                let pdf =
                    compile(&tex, Some(&img)).unwrap_or_else(|e| panic!("template {t}: {e:?}"));
                assert!(pdf.starts_with(b"%PDF"));
                if t == 0 && shape == 0 && Command::new("pdftoppm").arg("-v").output().is_ok() {
                    let (img, multi) = render(&pdf);
                    assert!(img.is_some() && !multi);
                }
            }
        }
        let _ = fs::remove_file(img);
    }
    #[test]
    fn bad_tex_reports_line() {
        if find_engine().is_none() {
            return;
        }
        let tex = "\\documentclass{article}\n\\begin{document}\n\\undefinedcmd\n\\end{document}\n";
        let e = compile(tex, None).unwrap_err();
        assert!(e.iter().any(|m| m.contains("line 3")), "{e:?}");
    }
}
