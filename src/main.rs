use crate::image_matrix::{ImageSequence, SlideAnimation};
use eframe::egui::{
    menu, Button, CentralPanel, Color32, Context, DragValue, Key, KeyboardShortcut, Modifiers,
    Painter, PointerButton, Pos2, Rect, Rounding, ScrollArea, Sense, Stroke, TextEdit,
    TopBottomPanel, Ui, Vec2, Window,
};
use eframe::{App, Frame, NativeOptions};
use image::imageops;
use image::imageops::{BiLevel, FilterType};
use image::io::Reader;
use rfd::{FileDialog, MessageDialog};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::fs;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod image_matrix;

fn main() {
    eframe::run_native(
        "",
        NativeOptions::default(),
        Box::new(|_cc| {
            Box::new(MainWindow {
                project: Project {
                    image_sequence: ImageSequence::new(4, 4),
                    frame_rate: 10,
                },
                current_file: None,
                scale: 1,
                current_frame: 1,
                show_grid: false,
                stoke_thickness: 1.0,
                onion_skin: false,
                onion_opacity: 0.05,
                display_color: [0xFF, 0x00, 0x00],
                new_file_dialog: NewFileDialog {
                    show: false,
                    width: 4,
                    height: 4,
                    frame_rate: 10,
                },
                code_display: CodeDisplay::SingleFrame,
                play: false,
                last_frame_delta: Instant::now(),
            })
        }),
    )
    .unwrap();
}

#[derive(Serialize, Deserialize)]
struct Project {
    image_sequence: ImageSequence,
    frame_rate: u16,
}

struct NewFileDialog {
    show: bool,
    width: u8,
    height: u8,
    frame_rate: u16,
}

struct MainWindow {
    project: Project,
    current_file: Option<PathBuf>,
    scale: u16,
    current_frame: usize,
    show_grid: bool,
    stoke_thickness: f32,
    onion_skin: bool,
    onion_opacity: f32,
    display_color: [u8; 3],
    new_file_dialog: NewFileDialog,
    code_display: CodeDisplay,
    play: bool,
    last_frame_delta: Instant,
}

#[derive(PartialEq)]
enum CodeDisplay {
    SingleFrame,
    AllFrames,
}

impl App for MainWindow {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        let frame_time = Duration::from_nanos(1000000000 / u64::from(self.project.frame_rate));
        if self.play && self.last_frame_delta.elapsed() >= frame_time {
            self.last_frame_delta = Instant::now();
            self.current_frame =
                self.current_frame % self.project.image_sequence.get_frame_count() + 1;
        }
        ctx.input_mut(|input_state| {
            if input_state.consume_shortcut(&Self::OPEN_SHORTCUT) {
                self.open_file();
            }
        });
        ctx.input_mut(|input_state| {
            if input_state.consume_shortcut(&Self::SAVE_SHORTCUT) {
                self.save_file();
            }
        });
        self.show_menu(ctx);
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.show_painter(ui);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Display color:");
                        ui.color_edit_button_srgb(&mut self.display_color);
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            DragValue::new(&mut self.current_frame)
                                .clamp_range(1..=self.project.image_sequence.get_frame_count())
                                .prefix("Frame: ")
                                .suffix(format!(
                                    "/{}",
                                    self.project.image_sequence.get_frame_count()
                                )),
                        );
                        if ui.button(if self.play { "Stop" } else { "Play" }).clicked() {
                            self.last_frame_delta = Instant::now();
                            self.play = !self.play;
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Add frame").clicked() {
                            self.project.image_sequence.add_frame();
                            self.current_frame = self.project.image_sequence.get_frame_count();
                        }
                        if ui.button("Insert frame").clicked() {
                            self.project
                                .image_sequence
                                .insert_frame(self.current_frame - 1);
                        }
                        if ui.button("Duplicate frame").clicked() {
                            self.project
                                .image_sequence
                                .duplicate_frame(self.current_frame - 1);
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Move up").clicked()
                            && self.project.image_sequence.move_up(self.current_frame - 1)
                        {
                            self.current_frame -= 1;
                        }
                        if ui.button("Move down").clicked()
                            && self
                                .project
                                .image_sequence
                                .move_down(self.current_frame - 1)
                        {
                            self.current_frame += 1;
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Delete frame").clicked() {
                            self.project
                                .image_sequence
                                .delete_frame(self.current_frame - 1);
                            if self.project.image_sequence.get_frame_count() == 0 {
                                self.project.image_sequence.add_frame();
                            } else if self.project.image_sequence.get_frame_count()
                                == self.current_frame - 1
                            {
                                self.current_frame -= 1;
                            }
                        }
                        if ui.button("Clear frame").clicked() {
                            self.project
                                .image_sequence
                                .clear_frame(self.current_frame - 1);
                        }
                    });
                });
            });
            Window::new("New")
                .open(&mut self.new_file_dialog.show)
                .show(ctx, |ui| {
                    ui.label("Width:");
                    ui.horizontal(|ui| {
                        ui.add(DragValue::new(&mut self.new_file_dialog.width).clamp_range(1..=8));
                        ui.label(format!(" × 8 = {}", self.new_file_dialog.width * 8));
                    });
                    ui.label("Height:");
                    ui.horizontal(|ui| {
                        ui.add(DragValue::new(&mut self.new_file_dialog.height).clamp_range(1..=8));
                        ui.label(format!(" × 8 = {}", self.new_file_dialog.height * 8));
                    });
                    ui.label("Frame rate:");
                    ui.add(
                        DragValue::new(&mut self.new_file_dialog.frame_rate)
                            .clamp_range(Self::FPS_RANGE),
                    );
                    ui.vertical_centered_justified(|ui| {
                        if ui.button("Confirm").clicked() {
                            self.current_file = None;
                            self.current_frame = 1;
                            self.project = Project {
                                image_sequence: ImageSequence::new(
                                    self.new_file_dialog.width,
                                    self.new_file_dialog.height,
                                ),
                                frame_rate: self.new_file_dialog.frame_rate,
                            };
                        }
                    });
                });
            ui.collapsing("Code", |ui| {
                ui.radio_value(
                    &mut self.code_display,
                    CodeDisplay::SingleFrame,
                    "Current frame",
                );
                ui.radio_value(&mut self.code_display, CodeDisplay::AllFrames, "All frames");
                ScrollArea::vertical().show(ui, |ui| {
                    ui.add(
                        TextEdit::multiline(&mut match self.code_display {
                            CodeDisplay::SingleFrame => self
                                .project
                                .image_sequence
                                .get_frame_as_string(self.current_frame - 1),
                            CodeDisplay::AllFrames => {
                                self.project.image_sequence.get_sequence_as_string()
                            }
                        })
                        .code_editor()
                        .desired_width(f32::INFINITY),
                    );
                });
            });
        });
        if self.play {
            ctx.request_repaint();
        }
    }
}

#[derive(Clone, Copy)]
pub enum Direction {
    Top,
    Left,
    Bottom,
    Right,
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Direction::Top => "Top",
                Direction::Left => "Left",
                Direction::Bottom => "Bottom",
                Direction::Right => "Right",
            }
        )
    }
}

impl Direction {
    fn iter() -> impl ExactSizeIterator<Item = Self> {
        [Self::Top, Self::Left, Self::Bottom, Self::Right].into_iter()
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

    const FPS_RANGE: RangeInclusive<u16> = 1..=60;

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

    fn render_frame(
        &self,
        painter: &Painter,
        painter_top_left: Pos2,
        frame_idx: usize,
        color: Color32,
    ) {
        if let Some(pixels) = self.project.image_sequence.iter_pixels(frame_idx) {
            let scale = usize::from(self.scale);
            let scale_vec2 = Vec2::new(self.scale.into(), self.scale.into());
            pixels.filter(|&(_, _, pixel)| pixel).for_each(|(x, y, _)| {
                let position_scaled =
                    Pos2::new((x * scale) as f32, (y * scale) as f32) + painter_top_left.to_vec2();
                painter.rect_filled(
                    Rect::from_min_size(position_scaled, scale_vec2),
                    Rounding::none(),
                    color,
                );
            });
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
        let color = Color32::from_rgb(
            self.display_color[0],
            self.display_color[1],
            self.display_color[2],
        );
        if self.onion_skin {
            if let Some(frame_idx) = self.current_frame.checked_sub(2) {
                self.render_frame(
                    &painter,
                    painter_top_left,
                    frame_idx,
                    color.linear_multiply(self.onion_opacity),
                );
            }
        }
        self.render_frame(&painter, painter_top_left, self.current_frame - 1, color);
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
                    if ui.button("New file").clicked() {
                        self.new_file_dialog.show = true;
                        ui.close_menu();
                    }
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
                    ui.separator();
                    if ui.button("Import image").clicked() {
                        self.import_image();
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.add(
                        DragValue::new(&mut self.scale)
                            .clamp_range(1..=64)
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
                    ui.checkbox(&mut self.onion_skin, "Onion skin");
                    ui.add(
                        DragValue::new(&mut self.onion_opacity)
                            .clamp_range(0.0..=1.0)
                            .speed(0.05)
                            .prefix("Onion skin opacity: "),
                    );
                });
                ui.menu_button("Animation", |ui| {
                    ui.add(
                        DragValue::new(&mut self.project.frame_rate)
                            .clamp_range(Self::FPS_RANGE)
                            .prefix("Frame rate: ")
                            .suffix(" f/s"),
                    );
                    ui.separator();
                    SlideAnimation::iter().for_each(|slide_animation| {
                        ui.menu_button(slide_animation.to_string(), |ui| {
                            Direction::iter().for_each(|direction| {
                                if ui.button(direction.to_string()).clicked() {
                                    self.project.image_sequence.slide(
                                        self.current_frame - 1,
                                        direction,
                                        slide_animation,
                                    );
                                    ui.close_menu();
                                }
                            });
                        });
                    });
                });
            });
        });
    }

    fn import_image(&mut self) {
        let Some(path) = FileDialog::new()
            .pick_file() else {
            return;
        };

        let Ok(Ok(image)) = Reader::open(&path).and_then(|reader| reader.with_guessed_format()).map(|reader| reader.decode()) else {
            MessageDialog::new()
                .set_description(&format!(
                    "Could not read/decode {}",
                    path.display()
                ))
                .show();
            return;
        };

        let [width, height] = self.project.image_sequence.get_dimensions_pixels();
        let scaled_image = image.resize_exact(
            width.try_into().unwrap(),
            height.try_into().unwrap(),
            FilterType::Lanczos3,
        );
        drop(image);

        let mut gray_image = scaled_image.into_luma8();
        imageops::dither(&mut gray_image, &BiLevel);

        self.project
            .image_sequence
            .insert_frame(self.current_frame - 1);
        gray_image
            .iter()
            .zip(
                self.project
                    .image_sequence
                    .iter_pixels_mut(self.current_frame - 1)
                    .unwrap(),
            )
            .for_each(|(&color, pixel)| {
                *pixel = color != 0;
            });
    }
}
