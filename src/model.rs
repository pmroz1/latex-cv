use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct Entry {
    pub title: String,
    pub location: String,
    pub subtitle: String,
    pub date: String,
    pub bullets: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct Skill {
    pub label: String,
    pub value: String,
}

/// User-defined section (certifications, awards, languages, publications...).
#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Section {
    pub title: String,
    pub visible: bool,
    pub entries: Vec<Entry>,
}

impl Default for Section {
    fn default() -> Self {
        Section {
            title: "New Section".into(),
            visible: true,
            entries: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Cv {
    pub version: u32,
    pub template: usize,
    pub accent: [u8; 3],
    pub name: String,
    pub role: String,
    pub email: String,
    pub phone: String,
    pub location: String,
    pub link: String,
    pub profile: String,
    pub education: Vec<Entry>,
    pub experience: Vec<Entry>,
    pub skills: Vec<Skill>,
    pub projects: Vec<Entry>,
    pub additional: String,
    pub sections: Vec<Section>,
}

impl Default for Cv {
    fn default() -> Self {
        let e = |t: &str, l: &str, s: &str, d: &str, b: &[&str]| Entry {
            title: t.into(),
            location: l.into(),
            subtitle: s.into(),
            date: d.into(),
            bullets: b.iter().map(|x| x.to_string()).collect(),
        };
        let s = |l: &str, v: &str| Skill {
            label: l.into(),
            value: v.into(),
        };
        Cv {
            version: SCHEMA_VERSION,
            template: 0,
            accent: [70, 130, 180],
            name: "Your Name".into(),
            role: "Professional Title".into(),
            email: "your.email@example.com".into(),
            phone: "+123 456 7890".into(),
            location: "City, Country".into(),
            link: "https://linkedin.com/in/yourprofile".into(),
            profile: "Short professional statement describing your strengths and goals.".into(),
            education: vec![e(
                "University Name",
                "City, Country",
                "Degree - Field of Study",
                "2018 - 2022",
                &["GPA: 3.8/4.0 | Relevant coursework: Course 1, Course 2"],
            )],
            experience: vec![
                e(
                    "Company Name",
                    "City, Country",
                    "Job Title",
                    "Jan 2023 - Present",
                    &[
                        "Accomplishment with quantified results.",
                        "Responsibility described with an action verb.",
                    ],
                ),
                e(
                    "Previous Company",
                    "City, Country",
                    "Job Title",
                    "2021 - 2022",
                    &["Accomplishment or responsibility."],
                ),
            ],
            skills: vec![
                s("Technical", "Rust, Python, SQL, Git"),
                s("Soft Skills", "Communication, Leadership"),
                s("Languages", "English (Native), Spanish (B2)"),
            ],
            projects: vec![e(
                "Project Name",
                "",
                "Role / Technologies",
                "2023",
                &["What the project does and your contribution."],
            )],
            additional: String::new(),
            sections: vec![],
        }
    }
}

impl Cv {
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, json)
    }
    pub fn load(path: &Path) -> io::Result<Self> {
        let s = fs::read_to_string(path)?;
        let mut cv: Cv =
            serde_json::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if cv.version > SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "project was created by a newer version",
            ));
        }
        // Migrations for older schema versions go here.
        cv.version = SCHEMA_VERSION;
        Ok(cv)
    }
    /// Human-readable warnings about likely problems.
    pub fn validate(&self) -> Vec<String> {
        let mut w = vec![];
        if self.name.trim().is_empty() {
            w.push("Name is empty".to_string());
        }
        if !self.email.is_empty() {
            let ok = self
                .email
                .split_once('@')
                .is_some_and(|(u, d)| !u.is_empty() && d.contains('.') && !d.ends_with('.'))
                && !self.email.contains(char::is_whitespace);
            if !ok {
                w.push(format!("Email looks invalid: {}", self.email));
            }
        }
        if !self.link.is_empty()
            && !(self.link.starts_with("http://") || self.link.starts_with("https://"))
        {
            w.push("Link should start with http:// or https://".to_string());
        }
        for (sec, list) in [
            ("Experience", &self.experience),
            ("Education", &self.education),
            ("Projects", &self.projects),
        ] {
            for (i, e) in list.iter().enumerate() {
                if e.title.trim().is_empty() {
                    w.push(format!("{sec} entry {} has no title", i + 1));
                }
            }
        }
        let lines: usize = [&self.experience, &self.education, &self.projects]
            .iter()
            .flat_map(|l| l.iter())
            .map(|e| 2 + e.bullets.len())
            .sum::<usize>()
            + self.skills.len()
            + self.profile.lines().count();
        if lines > 55 {
            w.push("CV may run past one page".to_string());
        }
        w
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join("latex_cv_roundtrip.cvproj");
        let cv = Cv::default();
        cv.save(&dir).unwrap();
        assert!(Cv::load(&dir).unwrap() == cv);
        let _ = fs::remove_file(dir);
    }
    #[test]
    fn custom_sections_roundtrip() {
        let mut cv = Cv::default();
        cv.sections.push(Section {
            title: "Awards".into(),
            ..Section::default()
        });
        let s = serde_json::to_string(&cv).unwrap();
        assert!(serde_json::from_str::<Cv>(&s).unwrap() == cv);
    }
    #[test]
    fn validation() {
        assert!(Cv::default().validate().is_empty());
        let cv = Cv {
            email: "bad".into(),
            link: "x".into(),
            name: "".into(),
            ..Cv::default()
        };
        assert_eq!(cv.validate().len(), 3);
    }
    #[test]
    fn newer_version_rejected() {
        let p = std::env::temp_dir().join("latex_cv_newer.cvproj");
        fs::write(&p, r#"{"version":999}"#).unwrap();
        assert!(Cv::load(&p).is_err());
        let _ = fs::remove_file(p);
    }
    #[test]
    fn partial_json_loads() {
        let cv: Cv = serde_json::from_str(r#"{"name":"A"}"#).unwrap();
        assert_eq!(cv.name, "A");
    }
}
