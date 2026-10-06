use crate::model::{Cv, Entry};

fn contact(cv: &Cv) -> Vec<&str> {
    [&cv.email, &cv.phone, &cv.location, &cv.link]
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.as_str())
        .collect()
}

fn sections(cv: &Cv) -> Vec<(&'static str, &Vec<Entry>)> {
    vec![
        ("Experience", &cv.experience),
        ("Education", &cv.education),
        ("Projects", &cv.projects),
    ]
}

pub fn markdown(cv: &Cv) -> String {
    let mut s = format!(
        "# {}\n\n**{}**\n\n{}\n\n",
        cv.name,
        cv.role,
        contact(cv).join(" | ")
    );
    if !cv.profile.trim().is_empty() {
        s += &format!("## Profile\n\n{}\n\n", cv.profile);
    }
    for (name, list) in sections(cv) {
        if list.is_empty() {
            continue;
        }
        s += &format!("## {name}\n\n");
        for e in list {
            s += &format!(
                "### {} — {}\n\n*{}*, {}\n\n",
                e.title, e.location, e.subtitle, e.date
            );
            for b in e.bullets.iter().filter(|b| !b.trim().is_empty()) {
                s += &format!("- {b}\n");
            }
            s += "\n";
        }
        if name == "Education" && !cv.skills.is_empty() {
            s += "## Skills\n\n";
            for k in &cv.skills {
                s += &format!("- **{}**: {}\n", k.label, k.value);
            }
            s += "\n";
        }
    }
    if !cv.additional.trim().is_empty() {
        s += &format!("## Additional Information\n\n{}\n", cv.additional);
    }
    s
}

/// Plain text without markup, suited to ATS parsers.
pub fn text(cv: &Cv) -> String {
    let mut s = format!("{}\n{}\n{}\n\n", cv.name, cv.role, contact(cv).join(" | "));
    if !cv.profile.trim().is_empty() {
        s += &format!("PROFILE\n{}\n\n", cv.profile);
    }
    for (name, list) in sections(cv) {
        if list.is_empty() {
            continue;
        }
        s += &format!("{}\n", name.to_uppercase());
        for e in list {
            s += &format!(
                "{}, {} - {} ({})\n",
                e.title, e.location, e.subtitle, e.date
            );
            for b in e.bullets.iter().filter(|b| !b.trim().is_empty()) {
                s += &format!("- {b}\n");
            }
            s += "\n";
        }
    }
    if !cv.skills.is_empty() {
        s += "SKILLS\n";
        for k in &cv.skills {
            s += &format!("{}: {}\n", k.label, k.value);
        }
        s += "\n";
    }
    if !cv.additional.trim().is_empty() {
        s += &format!("ADDITIONAL INFORMATION\n{}\n", cv.additional);
    }
    s
}

fn h(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn html(cv: &Cv) -> String {
    let [r, g, b] = cv.accent;
    let mut s = format!(
        "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>{n}</title>\n<style>body{{font-family:sans-serif;max-width:800px;margin:2em auto}}h2{{color:rgb({r},{g},{b});border-bottom:1px solid rgb({r},{g},{b})}}</style></head><body>\n<h1>{n}</h1><p><em>{}</em></p><p>{}</p>\n",
        h(&cv.role),
        h(&contact(cv).join(" | ")),
        n = h(&cv.name)
    );
    if !cv.profile.trim().is_empty() {
        s += &format!("<h2>Profile</h2><p>{}</p>\n", h(&cv.profile));
    }
    for (name, list) in sections(cv) {
        if list.is_empty() {
            continue;
        }
        s += &format!("<h2>{name}</h2>\n");
        for e in list {
            s += &format!(
                "<h3>{} — {}</h3><p><em>{}</em>, {}</p><ul>",
                h(&e.title),
                h(&e.location),
                h(&e.subtitle),
                h(&e.date)
            );
            for b in e.bullets.iter().filter(|b| !b.trim().is_empty()) {
                s += &format!("<li>{}</li>", h(b));
            }
            s += "</ul>\n";
        }
    }
    if !cv.skills.is_empty() {
        s += "<h2>Skills</h2><ul>";
        for k in &cv.skills {
            s += &format!("<li><strong>{}</strong>: {}</li>", h(&k.label), h(&k.value));
        }
        s += "</ul>\n";
    }
    if !cv.additional.trim().is_empty() {
        s += &format!(
            "<h2>Additional Information</h2><p>{}</p>\n",
            h(&cv.additional)
        );
    }
    s + "</body></html>\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exports_contain_name() {
        let cv = Cv::default();
        assert!(markdown(&cv).starts_with("# Your Name"));
        assert!(text(&cv).starts_with("Your Name"));
        assert!(html(&cv).contains("<h1>Your Name</h1>"));
    }
    #[test]
    fn html_escapes() {
        let cv = Cv {
            name: "<b>&".into(),
            ..Cv::default()
        };
        assert!(html(&cv).contains("&lt;b&gt;&amp;"));
    }
}
