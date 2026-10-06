use crate::model::{Cv, Entry};

pub const TEMPLATES: [&str; 7] = [
    "Classic",
    "Modern Banner",
    "Minimal",
    "Executive",
    "Compact",
    "Academic",
    "ATS-safe",
];
pub const FONTS: [&str; 6] = [
    "Template default",
    "Latin Modern",
    "Helvetica",
    "Palatino",
    "Charter",
    "Times",
];
pub const HEADINGS: [&str; 4] = [
    "Template default",
    "Accent with rule",
    "Plain bold",
    "Uppercase accent",
];
pub const SHAPES: [&str; 3] = ["Square", "Circle", "Rounded"];
pub const ACCENT_PRESETS: [(&str, [u8; 3]); 6] = [
    ("Steel", [70, 130, 180]),
    ("Navy", [25, 45, 100]),
    ("Forest", [34, 110, 70]),
    ("Crimson", [170, 30, 50]),
    ("Plum", [110, 50, 140]),
    ("Charcoal", [60, 60, 60]),
];

fn font_pkg(i: usize) -> Option<String> {
    let (pkg, family) = match i {
        1 => ("lmodern", "rmdefault"),
        2 => ("helvet", "sfdefault"),
        3 => ("palatino", "rmdefault"),
        4 => ("charter", "rmdefault"),
        5 => ("mathptmx", "rmdefault"),
        _ => return None,
    };
    Some(format!(
        "\\usepackage[T1]{{fontenc}}\n\\usepackage{{{pkg}}}\n\\renewcommand{{\\familydefault}}{{\\{family}}}"
    ))
}

fn heading_fmt(i: usize) -> Option<&'static str> {
    match i {
        1 => Some("\\titleformat{\\section}{\\large\\bfseries\\color{accent}}{}{0em}{}[\\color{accent}\\titlerule]\n\\titlespacing*{\\section}{0pt}{10pt}{5pt}"),
        2 => Some("\\titleformat{\\section}{\\large\\bfseries}{}{0em}{}\n\\titlespacing*{\\section}{0pt}{10pt}{5pt}"),
        3 => Some("\\titleformat{\\section}{\\large\\bfseries\\color{accent}}{}{0em}{\\MakeUppercase}\n\\titlespacing*{\\section}{0pt}{10pt}{5pt}"),
        _ => None,
    }
}

/// File name used for the photo inside a build directory.
pub fn photo_file_name(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg"))
        .unwrap_or_else(|| "jpg".into());
    format!("photo.{ext}")
}

/// LaTeX fragment placing the cropped, shaped photo; None when no usable photo is set.
fn photo_tex(cv: &Cv, file: &str) -> Option<String> {
    let p = &cv.photo;
    if p.path.is_empty() {
        return None;
    }
    let (w, h) = image::image_dimensions(&p.path).ok()?;
    let size = p.size_mm.clamp(10, 80) as f64;
    let zoom = p.zoom_pct.clamp(100, 400) as f64 / 100.0;
    // Scale so the shorter side covers the frame, then clip to the frame.
    let (iw, ih) = if w < h {
        (size * zoom, size * zoom * h as f64 / w as f64)
    } else {
        (size * zoom * w as f64 / h as f64, size * zoom)
    };
    let dx = p.offset_x.clamp(-100, 100) as f64 / 100.0 * (iw - size) / 2.0;
    let dy = p.offset_y.clamp(-100, 100) as f64 / 100.0 * (ih - size) / 2.0;
    let half = size / 2.0;
    let clip = match p.shape {
        0 => format!("(-{half:.2}mm,-{half:.2}mm) rectangle ({half:.2}mm,{half:.2}mm)"),
        2 => format!(
            "[rounded corners=4mm] (-{half:.2}mm,-{half:.2}mm) rectangle ({half:.2}mm,{half:.2}mm)"
        ),
        _ => format!("(0,0) circle ({half:.2}mm)"),
    };
    Some(format!(
        "\\begin{{tikzpicture}}\\clip {clip};\\node at ({dx:.2}mm,{dy:.2}mm) {{\\includegraphics[width={iw:.2}mm,height={ih:.2}mm]{{{file}}}}};\\end{{tikzpicture}}"
    ))
}

/// Escape LaTeX special characters in user text.
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => o.push_str(r"\textbackslash{}"),
            '&' | '%' | '$' | '#' | '_' | '{' | '}' => {
                o.push('\\');
                o.push(c)
            }
            '~' => o.push_str(r"\textasciitilde{}"),
            '^' => o.push_str(r"\textasciicircum{}"),
            '\n' => o.push_str(r"\\ "),
            _ => o.push(c),
        }
    }
    o
}

/// Escape text with a small markdown subset: **bold**, *italic*, [text](url).
pub fn rich(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("**") {
            if let Some(end) = r.find("**").filter(|&e| e > 0) {
                out += &format!("\\textbf{{{}}}", rich(&r[..end]));
                rest = &r[end + 2..];
                continue;
            }
        } else if let Some(r) = rest.strip_prefix('*') {
            if let Some(end) = r.find('*').filter(|&e| e > 0 && !r[..e].starts_with(' ')) {
                out += &format!("\\textit{{{}}}", esc(&r[..end]));
                rest = &r[end + 1..];
                continue;
            }
        } else if rest.starts_with('[') {
            if let Some(m) = rest.find("](") {
                if let Some(e) = rest[m..].find(')') {
                    let (text, link) = (&rest[1..m], &rest[m + 2..m + e]);
                    out += &format!("\\href{{{}}}{{{}}}", url(link), esc(text));
                    rest = &rest[m + e + 1..];
                    continue;
                }
            }
        }
        let c = rest.chars().next().unwrap();
        out += &esc(&c.to_string());
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// URLs inside \href need only a few characters escaped.
fn url(s: &str) -> String {
    s.replace('\\', "")
        .replace('%', r"\%")
        .replace('#', r"\#")
        .replace(['{', '}'], "")
}

fn contact(cv: &Cv, sep: &str) -> String {
    let mut parts = vec![];
    if !cv.email.is_empty() {
        parts.push(format!(
            r"\href{{mailto:{}}}{{{}}}",
            url(&cv.email),
            esc(&cv.email)
        ));
    }
    if !cv.phone.is_empty() {
        parts.push(esc(&cv.phone));
    }
    if !cv.location.is_empty() {
        parts.push(esc(&cv.location));
    }
    if !cv.link.is_empty() {
        parts.push(format!(r"\href{{{}}}{{{}}}", url(&cv.link), esc(&cv.link)));
    }
    parts.join(sep)
}

fn bullets(e: &Entry) -> String {
    let items: Vec<_> = e.bullets.iter().filter(|b| !b.trim().is_empty()).collect();
    if items.is_empty() {
        return String::new();
    }
    let mut s =
        String::from("\\begin{itemize}[leftmargin=1.2em,itemsep=1pt,parsep=0pt,topsep=2pt]\n");
    for b in items {
        s += &format!("  \\item {}\n", rich(b));
    }
    s + "\\end{itemize}\n"
}

fn entry(e: &Entry) -> String {
    let loc = if e.location.is_empty() {
        String::new()
    } else {
        esc(&e.location)
    };
    format!(
        "\\noindent\\begin{{tabular*}}{{\\textwidth}}{{@{{}}l@{{\\extracolsep{{\\fill}}}}r@{{}}}}\n\\textbf{{{}}} & {} \\\\\n\\textit{{{}}} & \\textit{{{}}}\n\\end{{tabular*}}\\vspace{{-2pt}}\n{}\\vspace{{4pt}}\n",
        esc(&e.title), loc, esc(&e.subtitle), esc(&e.date), bullets(e)
    )
}

fn section(title: &str, entries: &[Entry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut s = format!("\\section{{{}}}\n", title);
    for e in entries {
        s += &entry(e);
    }
    s
}

fn skills(cv: &Cv) -> String {
    let rows: Vec<_> = cv
        .skills
        .iter()
        .filter(|s| !s.label.is_empty() || !s.value.is_empty())
        .collect();
    if rows.is_empty() {
        return String::new();
    }
    let mut s = String::from(
        "\\section{Skills}\n\\begin{itemize}[leftmargin=1.2em,itemsep=1pt,parsep=0pt,topsep=2pt]\n",
    );
    for r in rows {
        s += &format!(
            "  \\item \\textbf{{{}}}: {}\n",
            esc(&r.label),
            esc(&r.value)
        );
    }
    s + "\\end{itemize}\n"
}

fn body(cv: &Cv) -> String {
    let mut s = String::new();
    if !cv.profile.trim().is_empty() {
        s += &format!("\\section{{Profile}}\n{}\n", esc(&cv.profile));
    }
    s += &section("Experience", &cv.experience);
    s += &section("Education", &cv.education);
    s += &skills(cv);
    s += &section("Projects", &cv.projects);
    for sec in cv.sections.iter().filter(|x| x.visible) {
        s += &section(&esc(&sec.title), &sec.entries);
    }
    if !cv.additional.trim().is_empty() {
        s += &format!(
            "\\section{{Additional Information}}\n{}\n",
            esc(&cv.additional)
        );
    }
    s
}

struct Style {
    font: &'static str,
    size: &'static str,
    /// (horizontal, top, bottom) margins in inches
    margins: (f64, f64, f64),
    section_fmt: &'static str,
    header: String,
}

pub fn generate(cv: &Cv) -> String {
    generate_with(cv, None)
}

/// Generate the document; `photo_file` overrides the photo path used in `\includegraphics`.
pub fn generate_with(cv: &Cv, photo_file: Option<&str>) -> String {
    let [r, g, b] = cv.accent;
    let name = esc(&cv.name);
    let role = esc(&cv.role);
    let st = match cv.template {
        // Classic: centered header, small caps coloured headings with rule
        0 => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}",
            size: "11pt",
            margins: (0.7, 0.6, 0.6),
            section_fmt: "\\titleformat{\\section}{\\large\\scshape\\color{accent}}{}{0em}{}[\\color{accent}\\titlerule]\n\\titlespacing*{\\section}{0pt}{10pt}{5pt}",
            header: format!("\\begin{{center}}\n{{\\LARGE\\bfseries {name}}}\\\\[3pt]\n{{\\color{{gray}} {role}}}\\\\[3pt]\n{{\\small {}}}\n\\end{{center}}\n", contact(cv, " $\\vert$ ")),
        },
        // Modern Banner: full-width coloured block with white name
        1 => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{helvet}\n\\renewcommand{\\familydefault}{\\sfdefault}",
            size: "10pt",
            margins: (0.7, 0.4, 0.6),
            section_fmt: "\\titleformat{\\section}{\\large\\bfseries\\color{accent}}{}{0em}{\\MakeUppercase}[\\vspace{-4pt}\\color{accent}\\titlerule]\n\\titlespacing*{\\section}{0pt}{10pt}{5pt}",
            header: format!("\\noindent\\setlength{{\\fboxsep}}{{0pt}}\\colorbox{{accent}}{{\\begin{{minipage}}{{\\textwidth}}\\vspace{{8pt}}\\hspace{{10pt}}\\begin{{minipage}}{{\\dimexpr\\textwidth-20pt\\relax}}\n{{\\color{{white}}\\fontsize{{26}}{{30}}\\selectfont\\bfseries {name}}}\\\\[2pt]\n{{\\color{{white}}\\large {role}}}\\\\[4pt]\n{{\\color{{white}}\\small {}}}\n\\end{{minipage}}\\vspace{{8pt}}\\end{{minipage}}}}\n\\vspace{{6pt}}\n", contact(cv, " \\textbullet\\ ")),
        },
        // Minimal: left-aligned, lots of whitespace, thin type
        2 => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{palatino}",
            size: "11pt",
            margins: (1.0, 0.8, 0.8),
            section_fmt: "\\titleformat{\\section}{\\normalfont\\small\\bfseries\\scshape\\color{accent}\\raggedright}{}{0em}{}\n\\titlespacing*{\\section}{0pt}{14pt}{4pt}",
            header: format!("\\noindent{{\\Huge\\bfseries {name}}}\\\\[4pt]\n{{\\large\\color{{gray}} {role}}}\\\\[4pt]\n{{\\small {}}}\n\\vspace{{4pt}}\n", contact(cv, " \\quad/\\quad ")),
        },
        // Executive: serif, double rule header, accent bold heading with bar
        3 => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{charter}",
            size: "11pt",
            margins: (0.8, 0.6, 0.6),
            section_fmt: "\\titleformat{\\section}{\\large\\bfseries\\color{accent}}{}{0em}{\\MakeUppercase}[{\\color{accent}\\titlerule[1.5pt]}]\n\\titlespacing*{\\section}{0pt}{12pt}{6pt}",
            header: format!("\\begin{{center}}\n{{\\color{{accent}}\\rule{{\\textwidth}}{{2pt}}}}\\vspace{{6pt}}\n{{\\fontsize{{24}}{{28}}\\selectfont\\bfseries\\MakeUppercase{{{name}}}}}\\\\[4pt]\n{{\\large\\itshape {role}}}\\\\[4pt]\n{{\\small {}}}\\\\[2pt]\n{{\\color{{accent}}\\rule{{\\textwidth}}{{2pt}}}}\n\\end{{center}}\n", contact(cv, " $\\cdot$ ")),
        },
        // Compact: dense, small margins, fits more on one page
        4 => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{helvet}\n\\renewcommand{\\familydefault}{\\sfdefault}",
            size: "9pt",
            margins: (0.5, 0.4, 0.4),
            section_fmt: "\\titleformat{\\section}{\\normalsize\\bfseries\\color{accent}}{}{0em}{}[\\color{accent}\\titlerule]\n\\titlespacing*{\\section}{0pt}{6pt}{3pt}",
            header: format!("\\noindent\\begin{{tabularx}}{{\\textwidth}}{{@{{}}X r@{{}}}}\n{{\\LARGE\\bfseries {name}}} & {{\\small {}}}\\\\\n{{\\color{{gray}} {role}}} &\n\\end{{tabularx}}\n", contact(cv, " $\\vert$ ")),
        },
        // Academic: serif, centred small-caps name, rules under headings
        5 => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{mathptmx}",
            size: "11pt",
            margins: (1.0, 0.8, 0.8),
            section_fmt: "\\titleformat{\\section}{\\large\\scshape\\bfseries}{}{0em}{}[\\titlerule]\n\\titlespacing*{\\section}{0pt}{12pt}{6pt}",
            header: format!("\\begin{{center}}\n{{\\Large\\scshape {name}}}\\\\[3pt]\n{{\\itshape {role}}}\\\\[3pt]\n{{\\small {}}}\n\\end{{center}}\n", contact(cv, " $\\cdot$ ")),
        },
        // ATS-safe: single column, no colour, plain text headings
        _ => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}",
            size: "11pt",
            margins: (0.8, 0.7, 0.7),
            section_fmt: "\\titleformat{\\section}{\\large\\bfseries}{}{0em}{\\MakeUppercase}\n\\titlespacing*{\\section}{0pt}{10pt}{4pt}",
            header: format!("\\noindent{{\\LARGE\\bfseries {name}}}\\\\[2pt]\n{role}\\\\[2pt]\n{}\n\\vspace{{2pt}}\n", contact(cv, " | ")),
        },
    };
    let d = &cv.design;
    let size = match d.font_size {
        10 => "10pt",
        11 => "11pt",
        12 => "12pt",
        _ => st.size,
    };
    let scale = d.margin_pct.clamp(50, 150) as f64 / 100.0;
    let margins = format!(
        "left={0:.2}in,right={0:.2}in,top={1:.2}in,bottom={2:.2}in",
        st.margins.0 * scale,
        st.margins.1 * scale,
        st.margins.2 * scale
    );
    let font = font_pkg(d.font).unwrap_or_else(|| st.font.to_string());
    let sec = heading_fmt(d.heading).unwrap_or(st.section_fmt);
    let spacing = format!(
        "\\linespread{{{:.2}}}",
        d.spacing_pct.clamp(80, 150) as f64 / 100.0
    );
    let header = match photo_tex(cv, photo_file.unwrap_or(&cv.photo.path)) {
        Some(ph) => format!(
            "\\noindent\\begin{{minipage}}[c]{{\\dimexpr\\textwidth-{}mm-6mm\\relax}}\n{}\\end{{minipage}}\\hfill{}\\par\\vspace{{4pt}}\n",
            cv.photo.size_mm.clamp(10, 80),
            st.header,
            ph
        ),
        None => st.header,
    };
    format!(
        "\\documentclass[a4paper,{size}]{{article}}\n\\usepackage[{margins}]{{geometry}}\n{font}\n\\usepackage{{xcolor}}\n\\usepackage{{titlesec}}\n\\usepackage{{enumitem}}\n\\usepackage{{tabularx}}\n\\usepackage{{graphicx}}\n\\usepackage{{tikz}}\n\\usepackage[hidelinks]{{hyperref}}\n\\definecolor{{accent}}{{RGB}}{{{r},{g},{b}}}\n\\definecolor{{gray}}{{RGB}}{{110,110,110}}\n{sec}\n{spacing}\n\\pagestyle{{empty}}\n\\setlength{{\\parindent}}{{0pt}}\n\\begin{{document}}\n{header}{body}\\end{{document}}\n",
        body = body(cv),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escapes() {
        assert_eq!(esc("R&D 50% #1 a_b"), r"R\&D 50\% \#1 a\_b");
    }
    #[test]
    fn escapes_all_specials() {
        assert_eq!(
            esc(r"\ & % $ # _ { } ~ ^"),
            r"\textbackslash{} \& \% \$ \# \_ \{ \} \textasciitilde{} \textasciicircum{}"
        );
    }
    #[test]
    fn rich_text() {
        assert_eq!(
            rich("**a_b** and *c* [x](http://u.io/#a)"),
            r"\textbf{a\_b} and \textit{c} \href{http://u.io/\#a}{x}"
        );
        assert_eq!(rich("2 * 3 * 4"), r"2 * 3 * 4");
        assert_eq!(rich("**open"), r"**open");
    }
    #[test]
    fn user_input_is_escaped_in_output() {
        let cv = Cv {
            name: "A & B".into(),
            ..Cv::default()
        };
        assert!(generate(&cv).contains(r"A \& B"));
    }
    #[test]
    fn custom_sections_rendered() {
        use crate::model::Section;
        let mut cv = Cv::default();
        let e = Entry {
            title: "Cert".into(),
            ..Entry::default()
        };
        cv.sections.push(Section {
            title: "Awards & Certs".into(),
            visible: true,
            entries: vec![e.clone()],
        });
        cv.sections.push(Section {
            title: "Hidden".into(),
            visible: false,
            entries: vec![e],
        });
        let t = generate(&cv);
        assert!(t.contains(r"\section{Awards \& Certs}"));
        assert!(!t.contains("Hidden"));
    }
    #[test]
    fn design_overrides_apply() {
        let mut cv = Cv::default();
        cv.design.font_size = 12;
        cv.design.margin_pct = 50;
        cv.design.font = 2;
        cv.design.heading = 2;
        let t = generate(&cv);
        assert!(t.contains("a4paper,12pt"));
        assert!(t.contains("left=0.35in"));
        assert!(t.contains("helvet"));
        assert!(t.contains(r"\titleformat{\section}{\large\bfseries}{}"));
    }
    #[test]
    fn missing_photo_is_ignored() {
        let mut cv = Cv::default();
        cv.photo.path = "/nonexistent/x.jpg".into();
        assert!(!generate(&cv).contains("includegraphics"));
    }
    #[test]
    fn photo_is_placed() {
        let dir = std::env::temp_dir().join("latex_cv_photo_test.png");
        image::RgbaImage::new(40, 20).save(&dir).unwrap();
        let mut cv = Cv::default();
        cv.photo.path = dir.display().to_string();
        let t = generate_with(&cv, Some("photo.png"));
        assert!(t.contains("includegraphics[width=60.00mm,height=30.00mm]{photo.png}"));
        assert!(t.contains("circle"));
        let _ = std::fs::remove_file(dir);
    }
    #[test]
    fn photo_names() {
        assert_eq!(photo_file_name("/a/b/Me.PNG"), "photo.png");
        assert_eq!(photo_file_name("x.bmp"), "photo.jpg");
    }
    #[test]
    fn all_templates_generate() {
        for t in 0..TEMPLATES.len() {
            let cv = Cv {
                template: t,
                ..Cv::default()
            };
            let s = generate(&cv);
            assert!(s.contains("\\begin{document}") && s.contains("Your Name"));
        }
    }
}
