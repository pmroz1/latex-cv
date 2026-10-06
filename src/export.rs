use crate::model::{Cv, Entry};

fn contact(cv: &Cv) -> Vec<&str> {
    [&cv.email, &cv.phone, &cv.location, &cv.link]
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.as_str())
        .collect()
}

fn sections(cv: &Cv) -> Vec<(&str, &Vec<Entry>)> {
    let mut v = vec![
        ("Experience", &cv.experience),
        ("Education", &cv.education),
        ("Projects", &cv.projects),
    ];
    v.extend(
        cv.sections
            .iter()
            .filter(|s| s.visible)
            .map(|s| (s.title.as_str(), &s.entries)),
    );
    v
}

/// Export as a JSON Resume (jsonresume.org) document.
pub fn json_resume(cv: &Cv) -> String {
    use serde_json::json;
    let work: Vec<_> = cv.experience.iter().map(|e| json!({"name": e.title, "location": e.location, "position": e.subtitle, "startDate": e.date, "highlights": e.bullets})).collect();
    let education: Vec<_> = cv
        .education
        .iter()
        .map(|e| json!({"institution": e.title, "area": e.subtitle, "startDate": e.date}))
        .collect();
    let projects: Vec<_> = cv.projects.iter().map(|e| json!({"name": e.title, "description": e.subtitle, "startDate": e.date, "highlights": e.bullets})).collect();
    let skills: Vec<_> = cv.skills.iter().map(|k| json!({"name": k.label, "keywords": k.value.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()).collect::<Vec<_>>()})).collect();
    let v = json!({
        "basics": {"name": cv.name, "label": cv.role, "email": cv.email, "phone": cv.phone, "url": cv.link, "summary": cv.profile, "location": {"address": cv.location}},
        "work": work, "education": education, "projects": projects, "skills": skills,
    });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// Import a JSON Resume document (best effort; unknown fields ignored).
pub fn from_json_resume(src: &str) -> Result<Cv, String> {
    use crate::model::Skill;
    let v: serde_json::Value = serde_json::from_str(src).map_err(|e| e.to_string())?;
    let st = |x: &serde_json::Value, k: &str| {
        x.get(k).and_then(|y| y.as_str()).unwrap_or("").to_string()
    };
    let list = |k: &str| {
        v.get(k)
            .and_then(|y| y.as_array())
            .cloned()
            .unwrap_or_default()
    };
    let dates = |x: &serde_json::Value| {
        let (a, b) = (st(x, "startDate"), st(x, "endDate"));
        if b.is_empty() {
            a
        } else {
            format!("{a} - {b}")
        }
    };
    let bullets = |x: &serde_json::Value| {
        x.get("highlights")
            .and_then(|h| h.as_array())
            .map(|h| {
                h.iter()
                    .filter_map(|t| t.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let b = v.get("basics").cloned().unwrap_or_default();
    let loc = b.get("location").cloned().unwrap_or_default();
    let location = [st(&loc, "city"), st(&loc, "countryCode")]
        .iter()
        .filter(|x| !x.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let location = if location.is_empty() {
        st(&loc, "address")
    } else {
        location
    };
    let mut cv = Cv {
        name: st(&b, "name"),
        role: st(&b, "label"),
        email: st(&b, "email"),
        phone: st(&b, "phone"),
        link: st(&b, "url"),
        profile: st(&b, "summary"),
        location,
        ..Cv::default()
    };
    cv.experience = list("work")
        .iter()
        .map(|w| Entry {
            title: st(w, "name"),
            location: st(w, "location"),
            subtitle: st(w, "position"),
            date: dates(w),
            bullets: bullets(w),
        })
        .collect();
    cv.education = list("education")
        .iter()
        .map(|e| Entry {
            title: st(e, "institution"),
            location: String::new(),
            subtitle: format!("{} {}", st(e, "studyType"), st(e, "area"))
                .trim()
                .to_string(),
            date: dates(e),
            bullets: vec![],
        })
        .collect();
    cv.projects = list("projects")
        .iter()
        .map(|p| Entry {
            title: st(p, "name"),
            location: String::new(),
            subtitle: st(p, "description"),
            date: dates(p),
            bullets: bullets(p),
        })
        .collect();
    cv.skills = list("skills")
        .iter()
        .map(|k| Skill {
            label: st(k, "name"),
            value: k
                .get("keywords")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default(),
        })
        .collect();
    Ok(cv)
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
        if name == "Projects" && !cv.skills.is_empty() {
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
    fn json_resume_roundtrip() {
        let cv = Cv::default();
        let back = from_json_resume(&json_resume(&cv)).unwrap();
        assert_eq!(back.name, cv.name);
        assert_eq!(back.experience.len(), cv.experience.len());
        assert_eq!(back.experience[0].bullets, cv.experience[0].bullets);
        assert_eq!(back.skills[0].value, "Rust, Python, SQL, Git");
        assert!(from_json_resume("nope").is_err());
    }
    #[test]
    fn custom_sections_exported() {
        use crate::model::Section;
        let mut cv = Cv::default();
        cv.sections.push(Section {
            title: "Awards".into(),
            visible: true,
            entries: vec![Entry {
                title: "Prize".into(),
                ..Entry::default()
            }],
        });
        assert!(markdown(&cv).contains("## Awards"));
        assert!(html(&cv).contains("<h2>Awards</h2>"));
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
