#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
mod latex;
mod model;

use eframe::egui;
use model::{Cv, Entry, Skill};
use std::{fs, path::PathBuf, process::Command};

struct App {
    cv: Cv,
    path: Option<PathBuf>,
    saved: Cv,
    status: String,
    show_source: bool,
}

impl App {
    fn new() -> Self {
        let cv = Cv::default();
        App { saved: cv.clone(), cv, path: None, status: "Ready".into(), show_source: false }
    }
    fn dirty(&self) -> bool {
        self.cv != self.saved
    }
    fn save(&mut self, as_new: bool) {
        let path = if as_new || self.path.is_none() {
            rfd::FileDialog::new().add_filter("CV project", &["cvproj"]).set_file_name("my-cv.cvproj").save_file()
        } else {
            self.path.clone()
        };
        if let Some(p) = path {
            match self.cv.save(&p) {
                Ok(_) => {
                    self.status = format!("Saved {}", p.display());
                    self.saved = self.cv.clone();
                    self.path = Some(p);
                }
                Err(e) => self.status = format!("Save failed: {e}"),
            }
        }
    }
    fn open(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("CV project", &["cvproj", "json"]).pick_file() {
            match Cv::load(&p) {
                Ok(cv) => {
                    self.saved = cv.clone();
                    self.cv = cv;
                    self.status = format!("Opened {}", p.display());
                    self.path = Some(p);
                }
                Err(e) => self.status = format!("Open failed: {e}"),
            }
        }
    }
    fn export_tex(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("LaTeX", &["tex"]).set_file_name("cv.tex").save_file() {
            self.status = match fs::write(&p, latex::generate(&self.cv)) {
                Ok(_) => format!("Exported {}", p.display()),
                Err(e) => format!("Export failed: {e}"),
            };
        }
    }
    fn export_pdf(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name("cv.pdf").save_file() else { return };
        let dir = std::env::temp_dir().join(format!("latex-cv-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        if let Err(e) = fs::write(dir.join("cv.tex"), latex::generate(&self.cv)) {
            self.status = format!("Write failed: {e}");
            return;
        }
        let engines: [(&str, &[&str]); 2] =
            [("tectonic", &["cv.tex"]), ("pdflatex", &["-interaction=nonstopmode", "-halt-on-error", "cv.tex"])];
        let mut msg = "No LaTeX engine found. Install tectonic, TeX Live or MiKTeX, or use Export .tex.".to_string();
        for (exe, args) in engines {
            if let Ok(out) = Command::new(exe).args(args).current_dir(&dir).output() {
                msg = if out.status.success() {
                    match fs::copy(dir.join("cv.pdf"), &p) {
                        Ok(_) => format!("PDF written to {}", p.display()),
                        Err(e) => format!("Copy failed: {e}"),
                    }
                } else {
                    format!("{exe} failed; export .tex and check the log")
                };
                break;
            }
        }
        let _ = fs::remove_dir_all(&dir);
        self.status = msg;
    }
}

fn entries_ui(ui: &mut egui::Ui, id: &str, list: &mut Vec<Entry>, labels: [&str; 4]) {
    let mut remove = None;
    for (i, e) in list.iter_mut().enumerate() {
        ui.push_id((id, i), |ui| {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                egui::Grid::new("g").num_columns(2).show(ui, |ui| {
                    for (l, v) in labels.iter().zip([&mut e.title, &mut e.location, &mut e.subtitle, &mut e.date]) {
                        ui.label(*l);
                        ui.add(egui::TextEdit::singleline(v).desired_width(300.0));
                        ui.end_row();
                    }
                });
                let mut rb = None;
                for (j, b) in e.bullets.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::multiline(b).desired_rows(1).desired_width(340.0));
                        if ui.small_button("✖").clicked() {
                            rb = Some(j);
                        }
                    });
                }
                if let Some(j) = rb {
                    e.bullets.remove(j);
                }
                ui.horizontal(|ui| {
                    if ui.button("+ Bullet").clicked() {
                        e.bullets.push(String::new());
                    }
                    if ui.button("Remove entry").clicked() {
                        remove = Some(i);
                    }
                });
            });
        });
    }
    if let Some(i) = remove {
        list.remove(i);
    }
    if ui.button("+ Add entry").clicked() {
        list.push(Entry::default());
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let ctrl_s = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S));
        if ctrl_s {
            self.save(false);
        }
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
                    *self = App::new();
                }
                if ui.button("Open…").clicked() {
                    self.open();
                }
                if ui.button("Save").clicked() {
                    self.save(false);
                }
                if ui.button("Save as…").clicked() {
                    self.save(true);
                }
                ui.separator();
                if ui.button("Export .tex").clicked() {
                    self.export_tex();
                }
                if ui.button("Export PDF").clicked() {
                    self.export_pdf();
                }
                ui.separator();
                ui.label("Template:");
                egui::ComboBox::from_id_salt("tpl")
                    .selected_text(latex::TEMPLATES[self.cv.template.min(4)])
                    .show_ui(ui, |ui| {
                        for (i, t) in latex::TEMPLATES.iter().enumerate() {
                            ui.selectable_value(&mut self.cv.template, i, *t);
                        }
                    });
                let mut c = egui::Color32::from_rgb(self.cv.accent[0], self.cv.accent[1], self.cv.accent[2]);
                if ui.color_edit_button_srgba(&mut c).changed() {
                    self.cv.accent = [c.r(), c.g(), c.b()];
                }
            });
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            let name = self.path.as_ref().map(|p| p.display().to_string()).unwrap_or("Untitled".into());
            ui.label(format!("{}{} — {}", name, if self.dirty() { " *" } else { "" }, self.status));
        });
        egui::SidePanel::right("preview").default_width(420.0).show(ctx, |ui| {
            ui.heading("LaTeX output");
            ui.checkbox(&mut self.show_source, "Show source");
            egui::ScrollArea::both().show(ui, |ui| {
                let mut src = latex::generate(&self.cv);
                ui.add(egui::TextEdit::multiline(&mut src).code_editor().desired_width(f32::INFINITY).interactive(self.show_source));
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let cv = &mut self.cv;
                ui.collapsing("Personal", |ui| {
                    egui::Grid::new("p").num_columns(2).show(ui, |ui| {
                        for (l, v) in [
                            ("Name", &mut cv.name), ("Title", &mut cv.role), ("Email", &mut cv.email),
                            ("Phone", &mut cv.phone), ("Location", &mut cv.location), ("Link", &mut cv.link),
                        ] {
                            ui.label(l);
                            ui.add(egui::TextEdit::singleline(v).desired_width(300.0));
                            ui.end_row();
                        }
                    });
                });
                ui.collapsing("Profile", |ui| {
                    ui.add(egui::TextEdit::multiline(&mut cv.profile).desired_width(f32::INFINITY));
                });
                ui.collapsing("Experience", |ui| entries_ui(ui, "exp", &mut cv.experience, ["Company", "Location", "Role", "Dates"]));
                ui.collapsing("Education", |ui| entries_ui(ui, "edu", &mut cv.education, ["School", "Location", "Degree", "Dates"]));
                ui.collapsing("Skills", |ui| {
                    let mut rm = None;
                    for (i, s) in cv.skills.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut s.label).desired_width(110.0));
                            ui.add(egui::TextEdit::singleline(&mut s.value).desired_width(260.0));
                            if ui.small_button("✖").clicked() {
                                rm = Some(i);
                            }
                        });
                    }
                    if let Some(i) = rm {
                        cv.skills.remove(i);
                    }
                    if ui.button("+ Add skill").clicked() {
                        cv.skills.push(Skill::default());
                    }
                });
                ui.collapsing("Projects", |ui| entries_ui(ui, "prj", &mut cv.projects, ["Project", "Location", "Role/Tech", "Date"]));
                ui.collapsing("Additional", |ui| {
                    ui.add(egui::TextEdit::multiline(&mut cv.additional).desired_width(f32::INFINITY));
                });
            });
        });
    }
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]).with_title("LaTeX CV Builder"),
        ..Default::default()
    };
    eframe::run_native("LaTeX CV Builder", opts, Box::new(|_| Ok(Box::new(App::new()))))
}
