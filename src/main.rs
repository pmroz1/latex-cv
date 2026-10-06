#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
mod build;
mod export;
mod latex;
mod model;

use eframe::egui;
use model::{Cv, Entry, Section, Skill};
use std::{
    fs,
    hash::{Hash, Hasher},
    path::PathBuf,
    time::{Duration, Instant},
};

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
    worker: Option<build::Worker>,
    live_preview: bool,
    job_id: u64,
    compiling: bool,
    edited_at: Option<Instant>,
    last_hash: u64,
    errors: Vec<String>,
    pdf: Option<Vec<u8>>,
    preview_tex: Option<egui::TextureHandle>,
    multi_page: bool,
    built_cv: Cv,
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
            worker: None,
            live_preview: true,
            job_id: 0,
            compiling: false,
            edited_at: Some(Instant::now()),
            last_hash: 0,
            errors: vec![],
            pdf: None,
            preview_tex: None,
            multi_page: false,
            built_cv: Cv::default(),
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
            let mut photo_name = None;
            if let (Some(src), Some(dir)) = (self.photo_path(), p.parent()) {
                let name = latex::photo_file_name(&self.cv.photo.path);
                if fs::copy(&src, dir.join(&name)).is_ok() {
                    photo_name = Some(name);
                }
            }
            self.status = match fs::write(&p, latex::generate_with(&self.cv, photo_name.as_deref()))
            {
                Ok(_) => format!("Exported {}", p.display()),
                Err(e) => format!("Export failed: {e}"),
            };
        }
    }
    fn photo_path(&self) -> Option<PathBuf> {
        let p = &self.cv.photo.path;
        (!p.is_empty() && PathBuf::from(p).is_file()).then(|| PathBuf::from(p))
    }
    fn source_hash(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        latex::generate_with(&self.cv, Some(&latex::photo_file_name(&self.cv.photo.path)))
            .hash(&mut h);
        if let Some(p) = self.photo_path() {
            if let Ok(m) = fs::metadata(&p) {
                m.len().hash(&mut h);
                m.modified().ok().hash(&mut h);
            }
        }
        h.finish()
    }
    /// Debounced background rebuild: called every frame.
    fn live_update(&mut self, ctx: &egui::Context) {
        if self.worker.is_none() {
            self.worker = Some(build::spawn(ctx.clone()));
        }
        if let Some(w) = &self.worker {
            while let Ok(out) = w.rx.try_recv() {
                if out.id != self.job_id {
                    continue;
                }
                self.compiling = false;
                match out.result {
                    Ok(b) => {
                        self.errors.clear();
                        self.multi_page = b.multi_page;
                        self.preview_tex = b
                            .preview
                            .map(|img| ctx.load_texture("preview", img, Default::default()));
                        self.pdf = Some(b.pdf);
                    }
                    Err(e) => self.errors = e,
                }
            }
        }
        if !self.live_preview {
            return;
        }
        if self.cv != self.built_cv {
            self.edited_at = Some(Instant::now());
            self.built_cv = self.cv.clone();
        }
        if let Some(t) = self.edited_at {
            let wait = Duration::from_millis(700);
            if t.elapsed() < wait {
                ctx.request_repaint_after(wait - t.elapsed());
                return;
            }
            self.edited_at = None;
            let h = self.source_hash();
            if h == self.last_hash {
                return; // cached: identical source already built
            }
            self.last_hash = h;
            self.job_id += 1;
            self.compiling = true;
            let job = build::Job {
                id: self.job_id,
                tex: latex::generate_with(
                    &self.cv,
                    Some(&latex::photo_file_name(&self.cv.photo.path)),
                ),
                photo: self.photo_path(),
            };
            if let Some(w) = &self.worker {
                let _ = w.tx.send(job);
            }
        }
    }
    fn cancel_build(&mut self) {
        self.job_id += 1;
        self.compiling = false;
        self.last_hash = 0;
    }
    fn export_pdf(&mut self) {
        let Some(p) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .set_file_name("cv.pdf")
            .save_file()
        else {
            return;
        };
        let fresh = self.pdf.is_some()
            && self.errors.is_empty()
            && !self.compiling
            && self.edited_at.is_none()
            && self.last_hash == self.source_hash();
        let pdf = match (&self.pdf, fresh) {
            (Some(pdf), true) => Ok(pdf.clone()),
            _ => {
                let photo = self.photo_path();
                let tex = latex::generate_with(
                    &self.cv,
                    Some(&latex::photo_file_name(&self.cv.photo.path)),
                );
                build::compile(&tex, photo.as_deref())
            }
        };
        self.status = match pdf {
            Ok(bytes) => match fs::write(&p, bytes) {
                Ok(_) => format!("PDF written to {}", p.display()),
                Err(e) => format!("Write failed: {e}"),
            },
            Err(errs) => {
                self.errors = errs.clone();
                format!("PDF export failed: {}", errs.join("; "))
            }
        };
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
            .default_width(460.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Preview");
                    ui.checkbox(&mut self.live_preview, "Live");
                    if self.compiling {
                        ui.spinner();
                        if ui.small_button("Cancel").clicked() {
                            self.cancel_build();
                        }
                    } else if self.edited_at.is_some() {
                        ui.weak("…waiting");
                    }
                    if !self.live_preview && ui.button("Build now").clicked() {
                        self.last_hash = 0;
                        self.edited_at = Some(Instant::now() - Duration::from_secs(1));
                        self.live_preview = true;
                    }
                    ui.checkbox(&mut self.show_source, "LaTeX source");
                });
                if !self.errors.is_empty() {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.colored_label(egui::Color32::from_rgb(200, 40, 40), "Build errors");
                        for e in &self.errors {
                            ui.colored_label(egui::Color32::from_rgb(200, 40, 40), format!("• {e}"));
                        }
                    });
                }
                if self.multi_page {
                    ui.colored_label(egui::Color32::from_rgb(200, 120, 0), "⚠ The CV runs past one page");
                }
                egui::ScrollArea::both().show(ui, |ui| {
                    if self.show_source {
                        let mut src = latex::generate(&self.cv);
                        ui.add(
                            egui::TextEdit::multiline(&mut src)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .interactive(false),
                        );
                    } else if let Some(t) = &self.preview_tex {
                        let w = ui.available_width();
                        let size = t.size_vec2();
                        ui.image((t.id(), size * (w / size.x)));
                    } else if self.errors.is_empty() {
                        ui.label("Preview needs a LaTeX engine (tectonic, pdflatex or xelatex) and pdftoppm (poppler-utils).");
                    }
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
                ui.collapsing("Design", |ui| {
                    let d = &mut cv.design;
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Accent:");
                        for (name, rgb) in latex::ACCENT_PRESETS {
                            let c = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                            if ui
                                .add(egui::Button::new(" ").fill(c))
                                .on_hover_text(name)
                                .clicked()
                            {
                                cv.accent = rgb;
                            }
                        }
                    });
                    egui::ComboBox::from_label("Font")
                        .selected_text(latex::FONTS[d.font.min(latex::FONTS.len() - 1)])
                        .show_ui(ui, |ui| {
                            for (i, f) in latex::FONTS.iter().enumerate() {
                                ui.selectable_value(&mut d.font, i, *f);
                            }
                        });
                    egui::ComboBox::from_label("Font size")
                        .selected_text(if d.font_size == 0 {
                            "Template default".to_string()
                        } else {
                            format!("{} pt", d.font_size)
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut d.font_size, 0, "Template default");
                            for sz in [10u8, 11, 12] {
                                ui.selectable_value(&mut d.font_size, sz, format!("{sz} pt"));
                            }
                        });
                    egui::ComboBox::from_label("Section headings")
                        .selected_text(latex::HEADINGS[d.heading.min(latex::HEADINGS.len() - 1)])
                        .show_ui(ui, |ui| {
                            for (i, f) in latex::HEADINGS.iter().enumerate() {
                                ui.selectable_value(&mut d.heading, i, *f);
                            }
                        });
                    ui.add(
                        egui::Slider::new(&mut d.margin_pct, 50..=150)
                            .suffix("%")
                            .text("Margins"),
                    );
                    ui.add(
                        egui::Slider::new(&mut d.spacing_pct, 80..=150)
                            .suffix("%")
                            .text("Line spacing"),
                    );
                    if ui.button("Reset design").clicked() {
                        *d = Default::default();
                    }
                });
                ui.collapsing("Photo", |ui| {
                    let ph = &mut cv.photo;
                    ui.horizontal(|ui| {
                        if ui.button("Choose photo…").clicked() {
                            if let Some(p) = rfd::FileDialog::new()
                                .add_filter("Image", &["png", "jpg", "jpeg"])
                                .pick_file()
                            {
                                ph.path = p.display().to_string();
                            }
                        }
                        if !ph.path.is_empty() && ui.button("Remove").clicked() {
                            ph.path.clear();
                        }
                    });
                    if ph.path.is_empty() {
                        ui.weak("No photo (PNG or JPEG)");
                    } else {
                        ui.weak(&ph.path);
                        if !PathBuf::from(&ph.path).is_file() {
                            ui.colored_label(
                                egui::Color32::from_rgb(200, 40, 40),
                                "File not found",
                            );
                        }
                        egui::ComboBox::from_label("Shape")
                            .selected_text(latex::SHAPES[ph.shape.min(latex::SHAPES.len() - 1)])
                            .show_ui(ui, |ui| {
                                for (i, f) in latex::SHAPES.iter().enumerate() {
                                    ui.selectable_value(&mut ph.shape, i, *f);
                                }
                            });
                        ui.add(
                            egui::Slider::new(&mut ph.size_mm, 15..=60)
                                .suffix(" mm")
                                .text("Size"),
                        );
                        ui.add(
                            egui::Slider::new(&mut ph.zoom_pct, 100..=300)
                                .suffix("%")
                                .text("Zoom"),
                        );
                        ui.add(egui::Slider::new(&mut ph.offset_x, -100..=100).text("Crop X"));
                        ui.add(egui::Slider::new(&mut ph.offset_y, -100..=100).text("Crop Y"));
                    }
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
        self.live_update(ctx);
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
