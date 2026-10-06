use crate::model::{Cv, Entry};

pub const TEMPLATES: [&str; 5] = ["Classic", "Modern Banner", "Minimal", "Executive", "Compact"];

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

/// URLs inside \href need only a few characters escaped.
fn url(s: &str) -> String {
    s.replace('\\', "").replace('%', r"\%").replace('#', r"\#").replace('{', "").replace('}', "")
}

fn contact(cv: &Cv, sep: &str) -> String {
    let mut parts = vec![];
    if !cv.email.is_empty() {
        parts.push(format!(r"\href{{mailto:{}}}{{{}}}", url(&cv.email), esc(&cv.email)));
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
    let mut s = String::from("\\begin{itemize}[leftmargin=1.2em,itemsep=1pt,parsep=0pt,topsep=2pt]\n");
    for b in items {
        s += &format!("  \\item {}\n", esc(b));
    }
    s + "\\end{itemize}\n"
}

fn entry(e: &Entry) -> String {
    let loc = if e.location.is_empty() { String::new() } else { esc(&e.location) };
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
    let rows: Vec<_> = cv.skills.iter().filter(|s| !s.label.is_empty() || !s.value.is_empty()).collect();
    if rows.is_empty() {
        return String::new();
    }
    let mut s = String::from("\\section{Skills}\n\\begin{itemize}[leftmargin=1.2em,itemsep=1pt,parsep=0pt,topsep=2pt]\n");
    for r in rows {
        s += &format!("  \\item \\textbf{{{}}}: {}\n", esc(&r.label), esc(&r.value));
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
    if !cv.additional.trim().is_empty() {
        s += &format!("\\section{{Additional Information}}\n{}\n", esc(&cv.additional));
    }
    s
}

struct Style {
    font: &'static str,
    size: &'static str,
    margins: &'static str,
    section_fmt: &'static str,
    header: String,
}

pub fn generate(cv: &Cv) -> String {
    let [r, g, b] = cv.accent;
    let name = esc(&cv.name);
    let role = esc(&cv.role);
    let st = match cv.template {
        // Classic: centered header, small caps coloured headings with rule
        0 => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}",
            size: "11pt",
            margins: "left=0.7in,right=0.7in,top=0.6in,bottom=0.6in",
            section_fmt: "\\titleformat{\\section}{\\large\\scshape\\color{accent}}{}{0em}{}[\\color{accent}\\titlerule]\n\\titlespacing*{\\section}{0pt}{10pt}{5pt}",
            header: format!("\\begin{{center}}\n{{\\LARGE\\bfseries {name}}}\\\\[3pt]\n{{\\color{{gray}} {role}}}\\\\[3pt]\n{{\\small {}}}\n\\end{{center}}\n", contact(cv, " $\\vert$ ")),
        },
        // Modern Banner: full-width coloured block with white name
        1 => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{helvet}\n\\renewcommand{\\familydefault}{\\sfdefault}",
            size: "10pt",
            margins: "left=0.7in,right=0.7in,top=0.4in,bottom=0.6in",
            section_fmt: "\\titleformat{\\section}{\\large\\bfseries\\color{accent}}{}{0em}{\\MakeUppercase}[\\vspace{-4pt}\\color{accent}\\titlerule]\n\\titlespacing*{\\section}{0pt}{10pt}{5pt}",
            header: format!("\\noindent\\setlength{{\\fboxsep}}{{0pt}}\\colorbox{{accent}}{{\\begin{{minipage}}{{\\textwidth}}\\vspace{{8pt}}\\hspace{{10pt}}\\begin{{minipage}}{{\\dimexpr\\textwidth-20pt\\relax}}\n{{\\color{{white}}\\fontsize{{26}}{{30}}\\selectfont\\bfseries {name}}}\\\\[2pt]\n{{\\color{{white}}\\large {role}}}\\\\[4pt]\n{{\\color{{white}}\\small {}}}\n\\end{{minipage}}\\vspace{{8pt}}\\end{{minipage}}}}\n\\vspace{{6pt}}\n", contact(cv, " \\textbullet\\ ")),
        },
        // Minimal: left-aligned, lots of whitespace, thin type
        2 => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{palatino}",
            size: "11pt",
            margins: "left=1in,right=1in,top=0.8in,bottom=0.8in",
            section_fmt: "\\titleformat{\\section}{\\normalfont\\small\\bfseries\\scshape\\color{accent}\\raggedright}{}{0em}{}\n\\titlespacing*{\\section}{0pt}{14pt}{4pt}",
            header: format!("\\noindent{{\\Huge\\bfseries {name}}}\\\\[4pt]\n{{\\large\\color{{gray}} {role}}}\\\\[4pt]\n{{\\small {}}}\n\\vspace{{4pt}}\n", contact(cv, " \\quad/\\quad ")),
        },
        // Executive: serif, double rule header, accent bold heading with bar
        3 => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{charter}",
            size: "11pt",
            margins: "left=0.8in,right=0.8in,top=0.6in,bottom=0.6in",
            section_fmt: "\\titleformat{\\section}{\\large\\bfseries\\color{accent}}{}{0em}{\\MakeUppercase}[{\\color{accent}\\titlerule[1.5pt]}]\n\\titlespacing*{\\section}{0pt}{12pt}{6pt}",
            header: format!("\\begin{{center}}\n{{\\color{{accent}}\\titlerule[2pt]}}\\vspace{{6pt}}\n{{\\fontsize{{24}}{{28}}\\selectfont\\bfseries\\MakeUppercase{{{name}}}}}\\\\[4pt]\n{{\\large\\itshape {role}}}\\\\[4pt]\n{{\\small {}}}\\\\[2pt]\n{{\\color{{accent}}\\titlerule[2pt]}}\n\\end{{center}}\n", contact(cv, " $\\cdot$ ")),
        },
        // Compact: dense, small margins, fits more on one page
        _ => Style {
            font: "\\usepackage[T1]{fontenc}\n\\usepackage{helvet}\n\\renewcommand{\\familydefault}{\\sfdefault}",
            size: "9pt",
            margins: "left=0.5in,right=0.5in,top=0.4in,bottom=0.4in",
            section_fmt: "\\titleformat{\\section}{\\normalsize\\bfseries\\color{accent}}{}{0em}{}[\\color{accent}\\titlerule]\n\\titlespacing*{\\section}{0pt}{6pt}{3pt}",
            header: format!("\\noindent\\begin{{tabularx}}{{\\textwidth}}{{@{{}}X r@{{}}}}\n{{\\LARGE\\bfseries {name}}} & {{\\small {}}}\\\\\n{{\\color{{gray}} {role}}} &\n\\end{{tabularx}}\n", contact(cv, " $\\vert$ ")),
        },
    };
    format!(
        "\\documentclass[a4paper,{size}]{{article}}\n\\usepackage[{margins}]{{geometry}}\n{font}\n\\usepackage{{xcolor}}\n\\usepackage{{titlesec}}\n\\usepackage{{enumitem}}\n\\usepackage{{tabularx}}\n\\usepackage[hidelinks]{{hyperref}}\n\\definecolor{{accent}}{{RGB}}{{{r},{g},{b}}}\n\\definecolor{{gray}}{{RGB}}{{110,110,110}}\n{sec}\n\\pagestyle{{empty}}\n\\setlength{{\\parindent}}{{0pt}}\n\\begin{{document}}\n{header}{body}\\end{{document}}\n",
        size = st.size, margins = st.margins, font = st.font, sec = st.section_fmt,
        header = st.header, body = body(cv),
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
    fn all_templates_generate() {
        for t in 0..TEMPLATES.len() {
            let cv = Cv { template: t, ..Cv::default() };
            let s = generate(&cv);
            assert!(s.contains("\\begin{document}") && s.contains("Your Name"));
        }
    }
}
