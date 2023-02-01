use crate::image_matrix::ImageSequence;
use eframe::egui::{
    menu, Button, CentralPanel, Color32, Context, DragValue, Key, KeyboardShortcut, Modifiers,
    PointerButton, Pos2, Rect, Rounding, Sense, Stroke, TopBottomPanel, Ui, Vec2,
};
use eframe::{App, Frame, NativeOptions};
use rfd::{FileDialog, MessageDialog};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

mod image_matrix;

fn main() {
    eframe::run_native(
        "",
        NativeOptions::default(),
        Box::new(|_cc| {
            Box::new(MainWindow {
                project: Project::default(),
                current_file: None,
                scale: 1,
                current_frame: 1,
                show_grid: false,
                stoke_thickness: 1.0,
            })
        }),
    );
}

#[derive(Serialize, Deserialize)]
struct Project {
    image_sequence: ImageSequence,
    frame_rate: u16,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            image_sequence: ImageSequence::new(4, 4, 1),
            frame_rate: 10,
        }
    }
}

struct MainWindow {
    project: Project,
    current_file: Option<PathBuf>,
    scale: u16,
    current_frame: usize,
    show_grid: bool,
    stoke_thickness: f32,
}

impl App for MainWindow {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        if ctx.input_mut().consume_shortcut(&Self::OPEN_SHORTCUT) {
            self.open_file();
        }
        if ctx.input_mut().consume_shortcut(&Self::SAVE_SHORTCUT) {
            self.save_file();
        }
        self.show_menu(ctx);
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.show_painter(ui);
                ui.vertical(|ui| {
                    ui.add(
                        DragValue::new(&mut self.current_frame)
                            .clamp_range(1..=self.project.image_sequence.get_frame_count())
                            .prefix("Frame: ")
                            .suffix(format!(
                                "/{}",
                                self.project.image_sequence.get_frame_count()
                            )),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Add frame").clicked() {
                            self.project.image_sequence.add_frame();
                            self.current_frame += self.project.image_sequence.get_frame_count();
                        }
                        if ui.button("Insert frame").clicked() {
                            self.project
                                .image_sequence
                                .insert_frame(self.current_frame - 1);
                        }
                    });
                    if ui.button("Delete frame").clicked() {
                        self.project
                            .image_sequence
                            .delete_frame(self.current_frame - 1);
                        if self.project.image_sequence.get_frame_count() == 0 {
                            self.project.image_sequence.add_frame();
                        }
                    }
                });
            });
        });
    }
}

impl MainWindow {
    const OPEN_SHORTCUT: KeyboardShortcut = KeyboardShortcut {
        modifiers: Modifiers::CTRL,
        key: Key::O,
    };

    const SAVE_SHORTCUT: KeyboardShortcut = KeyboardShortcut {
        modifiers: Modifiers::CTRL,
        key: Key::S,
    };

    fn open_file(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("BSON file", &["bson"])
            .pick_file() else {
            return;
        };

        let Ok(file_bytes) = fs::read(&path) else {
            MessageDialog::new()
                .set_description(&format!("Could not open file {} for reading", path.display()))
                .show();
            return;
        };

        let Ok(project) = bson::from_slice(&file_bytes) else {
            MessageDialog::new()
                .set_description(&format!("Could not parse file {}", path.display()))
                .show();
            return;
        };

        self.current_file = Some(path);
        self.current_frame = 1;
        self.project = project;
    }

    fn write_file(&self, path: &Path) -> bool {
        let serialized = match bson::to_vec(&self.project) {
            Ok(serialized) => serialized,
            Err(error) => {
                MessageDialog::new()
                    .set_description(&format!("Could not serialize project, error: {error}"))
                    .show();
                return false;
            }
        };

        if fs::write(path, serialized).is_err() {
            MessageDialog::new()
                .set_description(&format!(
                    "Could not open file {} for writing",
                    path.display()
                ))
                .show();
            return false;
        }

        true
    }

    fn save_file(&mut self) {
        if let Some(current_file) = &self.current_file {
            self.write_file(current_file);
        } else {
            self.save_file_as();
        }
    }

    fn save_file_as(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("BSON file", &["bson"])
            .save_file() else {
            return;
        };

        if self.write_file(&path) {
            self.current_file = Some(path);
        }
    }

    fn show_painter(&mut self, ui: &mut Ui) {
        let [width_pixels, height_pixels] = self.project.image_sequence.get_dimensions_pixels();
        let dimensions_scaled =
            self.project.image_sequence.get_dimensions_pixels_vec2() * f32::from(self.scale);
        let (response, painter) = ui.allocate_painter(dimensions_scaled, Sense::click_and_drag());
        let painter_top_left = response.rect.min;
        if let Some(pos) = response.interact_pointer_pos() {
            let Vec2 { x, y } = (pos - painter_top_left) / f32::from(self.scale);
            let (x, y) = (
                (x as usize).clamp(0, width_pixels - 1),
                (y as usize).clamp(0, height_pixels - 1),
            );
            if response.clicked_by(PointerButton::Primary)
                || response.dragged_by(PointerButton::Primary)
            {
                self.project.image_sequence[[x, y, self.current_frame - 1]] = true;
            } else if response.clicked_by(PointerButton::Secondary)
                || response.dragged_by(PointerButton::Secondary)
            {
                self.project.image_sequence[[x, y, self.current_frame - 1]] = false;
            }
        }
        painter.rect_filled(
            Rect::from_min_size(painter_top_left, dimensions_scaled),
            Rounding::none(),
            Color32::BLACK,
        );
        if let Some(pixels) = self
            .project
            .image_sequence
            .iter_pixels(self.current_frame - 1)
        {
            let scale = usize::from(self.scale);
            let scale_vec2 = Vec2::new(self.scale.into(), self.scale.into());
            pixels.filter(|&(_, _, pixel)| pixel).for_each(|(x, y, _)| {
                let position_scaled =
                    Pos2::new((x * scale) as f32, (y * scale) as f32) + painter_top_left.to_vec2();
                painter.rect_filled(
                    Rect::from_min_size(position_scaled, scale_vec2),
                    Rounding::none(),
                    Color32::RED,
                );
            });
        }
        if self.show_grid {
            let [width_matrices, height_matrices] =
                self.project.image_sequence.get_dimensions_pixels();
            let stroke = Stroke::new(self.stoke_thickness, Color32::WHITE);
            (0..width_matrices).for_each(|x| {
                painter.vline(
                    x as f32 * f32::from(self.scale) + painter_top_left.x,
                    painter_top_left.y..=dimensions_scaled.y + painter_top_left.y,
                    stroke,
                )
            });
            (0..height_matrices).for_each(|y| {
                painter.hline(
                    painter_top_left.x..=dimensions_scaled.x + painter_top_left.x,
                    y as f32 * f32::from(self.scale) + painter_top_left.y,
                    stroke,
                );
            });
        }
    }

    fn show_menu(&mut self, ctx: &Context) {
        TopBottomPanel::top("menu_panel").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(
                            Button::new("Open file")
                                .shortcut_text(ctx.format_shortcut(&Self::OPEN_SHORTCUT)),
                        )
                        .clicked()
                    {
                        self.open_file();
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            Button::new("Save file")
                                .shortcut_text(ctx.format_shortcut(&Self::SAVE_SHORTCUT)),
                        )
                        .clicked()
                    {
                        self.save_file();
                        ui.close_menu();
                    }
                    if ui.button("Save file as").clicked() {
                        self.save_file_as();
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.add(
                        DragValue::new(&mut self.scale)
                            .clamp_range(1..=16)
                            .prefix("Scale: ")
                            .suffix('x'),
                    );
                    ui.separator();
                    ui.checkbox(&mut self.show_grid, "Show grid");
                    ui.add(
                        DragValue::new(&mut self.stoke_thickness)
                            .clamp_range(0.1..=2.0)
                            .speed(0.1)
                            .prefix("Stroke: "),
                    );
                });
            });
        });
    }
}
