#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
mod export;
mod latex;
mod model;

use eframe::egui;
use model::{Cv, Entry, Section, Skill};
use std::{fs, path::PathBuf, process::Command};

struct App {
    cv: Cv,
    path: Option<PathBuf>,
    saved: Cv,
    status: String,
    show_source: bool,
    undo: Vec<Cv>,
    redo: Vec<Cv>,
    last: Cv,
    pending: Option<Pending>,
    allow_close: bool,
    last_autosave: std::time::Instant,
    recover: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Pending {
    New,
    Open,
    Close,
}

fn autosave_path() -> PathBuf {
    std::env::temp_dir().join("latex-cv-autosave.cvproj")
}

impl App {
    fn new() -> Self {
        let cv = Cv::default();
        App {
            saved: cv.clone(),
            last: cv.clone(),
            cv,
            path: None,
            status: "Ready".into(),
            show_source: false,
            undo: vec![],
            redo: vec![],
            pending: None,
            allow_close: false,
            last_autosave: std::time::Instant::now(),
            recover: autosave_path().exists(),
        }
    }
    fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo
                .push(std::mem::replace(&mut self.cv, prev.clone()));
            self.last = prev;
        }
    }
    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo
                .push(std::mem::replace(&mut self.cv, next.clone()));
            self.last = next;
        }
    }
    fn record_history(&mut self) {
        if self.cv != self.last {
            self.undo
                .push(std::mem::replace(&mut self.last, self.cv.clone()));
            if self.undo.len() > 200 {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
    }
    fn autosave(&mut self) {
        if self.dirty() && self.last_autosave.elapsed().as_secs() >= 30 {
            let _ = self.cv.save(&autosave_path());
            self.last_autosave = std::time::Instant::now();
        }
    }
    fn request(&mut self, action: Pending) {
        if self.dirty() {
            self.pending = Some(action);
        } else {
            self.run(action);
        }
    }
    fn run(&mut self, action: Pending) {
        match action {
            Pending::New => {
                *self = App {
                    recover: false,
                    ..App::new()
                }
            }
            Pending::Open => self.open(),
            Pending::Close => self.allow_close = true,
        }
    }
    fn import_json_resume(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("JSON Resume", &["json"])
            .pick_file()
        {
            match fs::read_to_string(&p)
                .map_err(|e| e.to_string())
                .and_then(|t| export::from_json_resume(&t))
            {
                Ok(cv) => {
                    self.cv = cv;
                    self.status = format!("Imported {}", p.display());
                }
                Err(e) => self.status = format!("Import failed: {e}"),
            }
        }
    }
    fn export_text(&mut self, name: &str, ext: &str, f: fn(&Cv) -> String) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter(name, &[ext])
            .set_file_name(format!("cv.{ext}"))
            .save_file()
        {
            self.status = match fs::write(&p, f(&self.cv)) {
                Ok(_) => format!("Exported {}", p.display()),
                Err(e) => format!("Export failed: {e}"),
            };
        }
    }
    fn dirty(&self) -> bool {
        self.cv != self.saved
    }
    fn save(&mut self, as_new: bool) {
        let path = if as_new || self.path.is_none() {
            rfd::FileDialog::new()
                .add_filter("CV project", &["cvproj"])
                .set_file_name("my-cv.cvproj")
                .save_file()
        } else {
            self.path.clone()
        };
        if let Some(p) = path {
            match self.cv.save(&p) {
                Ok(_) => {
                    self.status = format!("Saved {}", p.display());
                    let _ = fs::remove_file(autosave_path());
                    self.saved = self.cv.clone();
                    self.path = Some(p);
                }
                Err(e) => self.status = format!("Save failed: {e}"),
            }
        }
    }
    fn open(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("CV project", &["cvproj", "json"])
            .pick_file()
        {
            match Cv::load(&p) {
                Ok(cv) => {
                    self.saved = cv.clone();
                    self.last = cv.clone();
                    self.undo.clear();
                    self.redo.clear();
                    self.cv = cv;
                    self.status = format!("Opened {}", p.display());
                    self.path = Some(p);
                }
                Err(e) => self.status = format!("Open failed: {e}"),
            }
        }
    }
    fn export_tex(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("LaTeX", &["tex"])
            .set_file_name("cv.tex")
            .save_file()
        {
            self.status = match fs::write(&p, latex::generate(&self.cv)) {
                Ok(_) => format!("Exported {}", p.display()),
                Err(e) => format!("Export failed: {e}"),
            };
        }
    }
    fn export_pdf(&mut self) {
        let Some(p) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("cv.pdf")
            .save_file()
        else {
            return;
        };
        let dir = std::env::temp_dir().join(format!("latex-cv-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        if let Err(e) = fs::write(dir.join("cv.tex"), latex::generate(&self.cv)) {
            self.status = format!("Write failed: {e}");
            return;
        }
        let engines: [(&str, &[&str]); 2] = [
            ("tectonic", &["cv.tex"]),
            (
                "pdflatex",
                &["-interaction=nonstopmode", "-halt-on-error", "cv.tex"],
            ),
        ];
        let mut msg =
            "No LaTeX engine found. Install tectonic, TeX Live or MiKTeX, or use Export .tex."
                .to_string();
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

enum Op {
    Remove,
    Up,
    Down,
    Dup,
}

fn apply<T: Clone>(list: &mut Vec<T>, i: usize, op: Op) {
    match op {
        Op::Remove => {
            list.remove(i);
        }
        Op::Up if i > 0 => list.swap(i, i - 1),
        Op::Down if i + 1 < list.len() => list.swap(i, i + 1),
        Op::Dup => list.insert(i + 1, list[i].clone()),
        _ => {}
    }
}

fn move_buttons(ui: &mut egui::Ui) -> Option<Op> {
    let mut op = None;
    if ui.small_button("⬆").on_hover_text("Move up").clicked() {
        op = Some(Op::Up);
    }
    if ui.small_button("⬇").on_hover_text("Move down").clicked() {
        op = Some(Op::Down);
    }
    if ui.small_button("⧉").on_hover_text("Duplicate").clicked() {
        op = Some(Op::Dup);
    }
    if ui.small_button("✖").on_hover_text("Delete").clicked() {
        op = Some(Op::Remove);
    }
    op
}

fn entries_ui(ui: &mut egui::Ui, id: &str, list: &mut Vec<Entry>, labels: [&str; 4]) {
    let mut action = None;
    for (i, e) in list.iter_mut().enumerate() {
        ui.push_id((id, i), |ui| {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("#{}", i + 1));
                    if let Some(op) = move_buttons(ui) {
                        action = Some((i, op));
                    }
                });
                egui::Grid::new("g").num_columns(2).show(ui, |ui| {
                    for (l, v) in labels.iter().zip([
                        &mut e.title,
                        &mut e.location,
                        &mut e.subtitle,
                        &mut e.date,
                    ]) {
                        ui.label(*l);
                        ui.add(egui::TextEdit::singleline(v).desired_width(300.0));
                        ui.end_row();
                    }
                });
                let mut ba = None;
                for (j, b) in e.bullets.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::multiline(b)
                                .desired_rows(1)
                                .desired_width(260.0),
                        );
                        if let Some(op) = move_buttons(ui) {
                            ba = Some((j, op));
                        }
                    });
                }
                if let Some((j, op)) = ba {
                    apply(&mut e.bullets, j, op);
                }
                if ui.button("+ Bullet").clicked() {
                    e.bullets.push(String::new());
                }
                ui.weak("Bullets support **bold**, *italic*, [text](url)");
            });
        });
    }
    if let Some((i, op)) = action {
        apply(list, i, op);
    }
    if ui.button("+ Add entry").clicked() {
        list.push(Entry::default());
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let cmd = egui::Modifiers::COMMAND;
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::S)) {
            self.save(false);
        }
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::O)) {
            self.request(Pending::Open);
        }
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::E)) {
            self.export_pdf();
        }
        if ctx.input_mut(|i| {
            i.consume_key(cmd | egui::Modifiers::SHIFT, egui::Key::Z)
                || i.consume_key(cmd, egui::Key::Y)
        }) {
            self.redo();
        } else if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::Z)) {
            self.undo();
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty() && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Close);
        }
        if let Some(action) = self.pending {
            egui::Window::new("Unsaved changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("You have unsaved changes.");
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            self.save(false);
                            if !self.dirty() {
                                self.pending = None;
                                self.run(action);
                            }
                        }
                        if ui.button("Discard").clicked() {
                            self.pending = None;
                            self.run(action);
                        }
                        if ui.button("Cancel").clicked() {
                            self.pending = None;
                        }
                    });
                });
            if self.allow_close {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
                    self.request(Pending::New);
                }
                if ui.button("Open…").clicked() {
                    self.request(Pending::Open);
                }
                if ui
                    .add_enabled(!self.undo.is_empty(), egui::Button::new("↶"))
                    .on_hover_text("Undo (Ctrl+Z)")
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(!self.redo.is_empty(), egui::Button::new("↷"))
                    .on_hover_text("Redo (Ctrl+Y)")
                    .clicked()
                {
                    self.redo();
                }
                if ui.button("Save").clicked() {
                    self.save(false);
                }
                if ui.button("Save as…").clicked() {
                    self.save(true);
                }
                if ui.button("Import JSON Resume…").clicked() {
                    self.import_json_resume();
                }
                ui.separator();
                if ui.button("Export .tex").clicked() {
                    self.export_tex();
                }
                if ui.button("Export PDF").clicked() {
                    self.export_pdf();
                }
                ui.menu_button("More export", |ui| {
                    if ui.button("Markdown").clicked() {
                        self.export_text("Markdown", "md", export::markdown);
                        ui.close_menu();
                    }
                    if ui.button("HTML").clicked() {
                        self.export_text("HTML", "html", export::html);
                        ui.close_menu();
                    }
                    if ui.button("JSON Resume").clicked() {
                        self.export_text("JSON Resume", "json", export::json_resume);
                        ui.close_menu();
                    }
                    if ui.button("Plain text (ATS)").clicked() {
                        self.export_text("Text", "txt", export::text);
                        ui.close_menu();
                    }
                });
                ui.separator();
                ui.label("Template:");
                egui::ComboBox::from_id_salt("tpl")
                    .selected_text(
                        latex::TEMPLATES[self.cv.template.min(latex::TEMPLATES.len() - 1)],
                    )
                    .show_ui(ui, |ui| {
                        for (i, t) in latex::TEMPLATES.iter().enumerate() {
                            ui.selectable_value(&mut self.cv.template, i, *t);
                        }
                    });
                let mut c = egui::Color32::from_rgb(
                    self.cv.accent[0],
                    self.cv.accent[1],
                    self.cv.accent[2],
                );
                if ui.color_edit_button_srgba(&mut c).changed() {
                    self.cv.accent = [c.r(), c.g(), c.b()];
                }
            });
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            let name = self
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or("Untitled".into());
            ui.label(format!(
                "{}{} — {}",
                name,
                if self.dirty() { " *" } else { "" },
                self.status
            ));
        });
        if self.recover {
            egui::TopBottomPanel::top("recover").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("An autosaved project from a previous session was found.");
                    if ui.button("Recover").clicked() {
                        if let Ok(cv) = Cv::load(&autosave_path()) {
                            self.cv = cv;
                        }
                        self.recover = false;
                    }
                    if ui.button("Dismiss").clicked() {
                        let _ = fs::remove_file(autosave_path());
                        self.recover = false;
                    }
                });
            });
        }
        let warnings = self.cv.validate();
        if !warnings.is_empty() {
            egui::TopBottomPanel::bottom("warnings").show(ctx, |ui| {
                for w in &warnings {
                    ui.colored_label(egui::Color32::from_rgb(200, 120, 0), format!("⚠ {w}"));
                }
            });
        }
        egui::SidePanel::right("preview")
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.heading("LaTeX output");
                ui.checkbox(&mut self.show_source, "Show source");
                egui::ScrollArea::both().show(ui, |ui| {
                    let mut src = latex::generate(&self.cv);
                    ui.add(
                        egui::TextEdit::multiline(&mut src)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .interactive(self.show_source),
                    );
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let cv = &mut self.cv;
                ui.collapsing("Personal", |ui| {
                    egui::Grid::new("p").num_columns(2).show(ui, |ui| {
                        for (l, v) in [
                            ("Name", &mut cv.name),
                            ("Title", &mut cv.role),
                            ("Email", &mut cv.email),
                            ("Phone", &mut cv.phone),
                            ("Location", &mut cv.location),
                            ("Link", &mut cv.link),
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
                ui.collapsing("Experience", |ui| {
                    entries_ui(
                        ui,
                        "exp",
                        &mut cv.experience,
                        ["Company", "Location", "Role", "Dates"],
                    )
                });
                ui.collapsing("Education", |ui| {
                    entries_ui(
                        ui,
                        "edu",
                        &mut cv.education,
                        ["School", "Location", "Degree", "Dates"],
                    )
                });
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
                ui.collapsing("Projects", |ui| {
                    entries_ui(
                        ui,
                        "prj",
                        &mut cv.projects,
                        ["Project", "Location", "Role/Tech", "Date"],
                    )
                });
                let mut sec_action = None;
                for (i, sec) in cv.sections.iter_mut().enumerate() {
                    ui.push_id(("sec", i), |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut sec.visible, "")
                                .on_hover_text("Show in CV");
                            ui.add(egui::TextEdit::singleline(&mut sec.title).desired_width(180.0));
                            if let Some(op) = move_buttons(ui) {
                                sec_action = Some((i, op));
                            }
                        });
                        ui.collapsing(format!("Entries: {}", sec.title), |ui| {
                            entries_ui(
                                ui,
                                "cs",
                                &mut sec.entries,
                                ["Title", "Location", "Subtitle", "Date"],
                            )
                        });
                    });
                }
                if let Some((i, op)) = sec_action {
                    apply(&mut cv.sections, i, op);
                }
                ui.horizontal(|ui| {
                    if ui.button("+ Add section").clicked() {
                        cv.sections.push(Section::default());
                    }
                    ui.menu_button("+ Preset", |ui| {
                        for t in ["Certifications", "Awards", "Languages", "Publications"] {
                            if ui.button(t).clicked() {
                                cv.sections.push(Section {
                                    title: t.into(),
                                    ..Section::default()
                                });
                                ui.close_menu();
                            }
                        }
                    });
                });
                ui.collapsing("Additional", |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut cv.additional).desired_width(f32::INFINITY),
                    );
                });
            });
        });
        self.record_history();
        self.autosave();
    }
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("LaTeX CV Builder"),
        ..Default::default()
    };
    eframe::run_native(
        "LaTeX CV Builder",
        opts,
        Box::new(|_| Ok(Box::new(App::new()))),
    )
}
