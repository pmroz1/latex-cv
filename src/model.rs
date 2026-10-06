use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

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

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Cv {
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
        let s = |l: &str, v: &str| Skill { label: l.into(), value: v.into() };
        Cv {
            template: 0,
            accent: [70, 130, 180],
            name: "Your Name".into(),
            role: "Professional Title".into(),
            email: "your.email@example.com".into(),
            phone: "+123 456 7890".into(),
            location: "City, Country".into(),
            link: "https://linkedin.com/in/yourprofile".into(),
            profile: "Short professional statement describing your strengths and goals.".into(),
            education: vec![e("University Name", "City, Country", "Degree - Field of Study", "2018 - 2022",
                &["GPA: 3.8/4.0 | Relevant coursework: Course 1, Course 2"])],
            experience: vec![
                e("Company Name", "City, Country", "Job Title", "Jan 2023 - Present",
                    &["Accomplishment with quantified results.", "Responsibility described with an action verb."]),
                e("Previous Company", "City, Country", "Job Title", "2021 - 2022", &["Accomplishment or responsibility."]),
            ],
            skills: vec![
                s("Technical", "Rust, Python, SQL, Git"),
                s("Soft Skills", "Communication, Leadership"),
                s("Languages", "English (Native), Spanish (B2)"),
            ],
            projects: vec![e("Project Name", "", "Role / Technologies", "2023", &["What the project does and your contribution."])],
            additional: String::new(),
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
        serde_json::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
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
    fn partial_json_loads() {
        let cv: Cv = serde_json::from_str(r#"{"name":"A"}"#).unwrap();
        assert_eq!(cv.name, "A");
    }
}
