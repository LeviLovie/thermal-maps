use anyhow::{Context, Result};
use egui::{Color32, Grid, RichText};
use macroquad::prelude::*;
use std::path::PathBuf;

use crate::image::Image;

// const BAR_X: f32 = 290.0;
// const BAR_MAX: f32 = 60.0;
// const BAR_MIN: f32 = 191.0;

#[derive(Debug)]
pub enum AppState {
    SelectFolder(SelectFolderData),
    Browse(BrowseData),
}

#[derive(Debug)]
pub struct SelectFolderData {
    file_dialog: egui_file_dialog::FileDialog,
    picked_dir: Option<PathBuf>,
}

impl SelectFolderData {
    pub async fn update(&mut self) -> Result<Option<AppState>> {
        Ok(None)
    }

    pub async fn draw(&mut self) -> Option<AppState> {
        let mut new_state: Option<AppState> = None;

        egui_macroquad::ui(|egui_ctx| {
            self.file_dialog.update(egui_ctx);

            if let Some(path) = self.file_dialog.take_picked() {
                self.picked_dir = Some(path.to_path_buf());
                match BrowseData::new(path.to_path_buf())
                    .context("Failed to create BrowseData from selected folder")
                {
                    Ok(browse_data) => {
                        new_state = Some(AppState::Browse(browse_data));
                    }
                    Err(e) => {
                        eprintln!("Error creating BrowseData: {}", e);
                    }
                }
            }
        });
        egui_macroquad::draw();

        new_state
    }
}

#[derive(Debug)]
pub struct BrowseData {
    images: Vec<Image>,
    images_height: f32,
    max_width: f32,
    scroll: f32,
    selected_image: Option<usize>,
    hover: Option<f32>,
}

impl BrowseData {
    pub fn new(path: PathBuf) -> Result<Self> {
        let mut images = Vec::new();
        for entry in std::fs::read_dir(path.clone()).expect("Failed to read selected directory") {
            if let Ok(entry) = entry {
                if let Some(ext) = entry.path().extension() {
                    if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "bmp" {
                        images.push(
                            Image::new(entry.path()).context(format!(
                                "Failed to create image from {}",
                                path.display()
                            ))?,
                        );
                    }
                }
            }
        }

        Ok(BrowseData {
            images,
            images_height: 0.0,
            max_width: 0.0,
            scroll: 0.0,
            selected_image: None,
            hover: None,
        })
    }

    pub async fn update(&mut self) -> Result<Option<AppState>> {
        // Update scrolling
        let mouse_wheel = mouse_wheel();
        if mouse_wheel.1 != 0.0 {
            self.scroll += mouse_wheel.1 * 10.0;
            if self.scroll > 0.0 {
                self.scroll = 0.0;
            }
            if self.images_height > 0.0 {
                let screen_height = screen_height();
                if self.scroll < -(self.images_height - screen_height) {
                    self.scroll = -(self.images_height - screen_height);
                }
            }
        }

        // Handle mouse selection
        if is_mouse_button_pressed(MouseButton::Left) {
            let mouse_pos = mouse_position();
            let mut y = self.scroll;
            for (i, image) in self.images.iter().enumerate() {
                let data = image.data.lock().unwrap();
                if let Some(d) = data.as_ref() {
                    if mouse_pos.0 >= 0.0
                        && mouse_pos.0 <= d.texture.width() / 2.0
                        && mouse_pos.1 >= y
                        && mouse_pos.1 <= y + d.texture.height() / 2.0
                    {
                        self.selected_image = Some(i);
                        self.hover = Some(mouse_pos.1 - y);
                        break;
                    }
                    y += d.texture.height() / 2.0 + 10.0;
                    if i != self.images.len() - 1 {
                        y += 10.0;
                    }
                }
            }
        }

        // Update temperature based on mouse pos from the image
        if let Some(selected) = self.selected_image {
            let data = self.images[selected].data.lock().unwrap();
            if let Some(d) = &*data {
                let mouse_pos = mouse_position();
                let image_ratio = d.texture.width() / d.texture.height();
                let width = screen_width() - self.max_width - 10.0 - 300.0 - 10.0;
                let height = width / image_ratio;
                let x_offset = self.max_width + 10.0;
                let y_offset = 0.0;

                // Check if mouse is hovering over the image
                if mouse_pos.0 >= x_offset
                    && mouse_pos.0 <= x_offset + width
                    && mouse_pos.1 >= y_offset
                    && mouse_pos.1 <= y_offset + height
                {
                    let pixel_x = (mouse_pos.0 - x_offset) * (d.texture.width() / width);
                    let pixel_y = (mouse_pos.1 - y_offset) * (d.texture.height() / height);

                    if pixel_x < d.image.width() as f32 && pixel_y < d.image.height() as f32 {
                        let pixel = d.image.get_pixel(pixel_x as u32, pixel_y as u32);
                        self.hover = d.color_temp.get_closest(&[pixel[0], pixel[1], pixel[2]]);
                    }
                } else {
                    self.hover = None;
                }
            }
        }

        Ok(None)
    }

    pub async fn draw(&mut self) -> Result<Option<AppState>> {
        self.images_height = 0.0;
        let images_len = self.images.len();
        let y = &mut self.scroll;
        for (i, image) in self.images.iter_mut().enumerate() {
            let mut data = image.data.lock().unwrap();
            if let Some(d) = data.as_mut() {
                draw_texture_ex(
                    &d.texture,
                    0.0,
                    *y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(
                            d.texture.width() / 2.0,
                            d.texture.height() / 2.0,
                        )),
                        ..Default::default()
                    },
                );
                if self.selected_image == Some(i) {
                    draw_rectangle_lines(
                        0.0,
                        *y,
                        d.texture.width() / 2.0,
                        d.texture.height() / 2.0,
                        2.0,
                        YELLOW,
                    );
                }
                *y += d.texture.height() / 2.0 + 10.0;
                self.images_height += d.texture.height() + 10.0;
                if i != images_len - 1 {
                    *y += 10.0;
                }
                if self.max_width < d.texture.width() / 2.0 {
                    self.max_width = d.texture.width() / 2.0;
                }
            } else {
                image.load().context(format!(
                    "Failed to load image data for {}",
                    image.path.display()
                ))?;
            }
        }

        let max_width = self.max_width;
        if let Some(image) = self.selected_image {
            let data = self.images[image].data.lock().unwrap();
            if let Some(d) = data.as_ref() {
                let image_ratio = d.texture.width() / d.texture.height();
                let width = screen_width() - max_width - 10.0 - 300.0 - 10.0;
                let height = width / image_ratio;
                draw_texture_ex(
                    &d.texture,
                    max_width + 10.0,
                    0.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(width, height)),
                        ..Default::default()
                    },
                );
            }
        }

        egui_macroquad::ui(|egui_ctx| {
            egui::SidePanel::right("properties")
                .exact_width(300.0)
                .show(egui_ctx, |ui| {
                    if let Some(image) = self.selected_image {
                        let mut data = self.images[image].data.lock().unwrap();
                        if let Some(d) = data.as_mut() {
                            ui.label(
                                RichText::new(format!(
                                    "{}",
                                    self.images[image]
                                        .path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                ))
                                .size(20.0),
                            );

                            Grid::new("props").show(ui, |ui| {
                                ui.label("Width");
                                ui.label(format!("{} px", d.texture.width() as usize));
                                ui.end_row();

                                ui.label("Height");
                                ui.label(format!("{} px", d.texture.height() as usize));
                                ui.end_row();

                                ui.label("Size");
                                ui.label(format!("{} bytes", d.image.as_raw().len()));
                                ui.end_row();
                            });

                            ui.separator();

                            Grid::new("controls").num_columns(2).show(ui, |ui| {
                                ui.label("Min");
                                ui.add(egui::DragValue::new(&mut d.min).speed(d.step));
                                ui.end_row();

                                ui.label("Max");
                                ui.add(egui::DragValue::new(&mut d.max).speed(d.step));
                                ui.end_row();

                                ui.label("Step");
                                ui.add(egui::DragValue::new(&mut d.step).speed(0.1));
                                ui.end_row();
                            });

                            if let Some(hover) = self.hover {
                                ui.label(RichText::new(format!("Hover: {:.2}°C ", hover)));
                            }

                            if ui.button("Extract color map").clicked() {
                                println!("Extracting color map...");
                            }
                        }
                    } else {
                        ui.label(RichText::new("No image selected"));
                    }
                });

            if let Some(hover) = &self.hover {
                let mouse_pos = mouse_position();
                egui::Window::new("Temp")
                    .collapsible(false)
                    .resizable(false)
                    .fixed_pos([mouse_pos.0 + 20.0, mouse_pos.1 + 20.0])
                    .title_bar(false)
                    .movable(false)
                    .interactable(false)
                    .show(egui_ctx, |ui| {
                        ui.label(
                            RichText::new(format!("{:.2}°C", hover))
                                .color(Color32::from_rgb(255, 255, 255))
                                .size(20.0),
                        );
                    });
            }
        });
        egui_macroquad::draw();

        Ok(None)
    }
}

pub struct App {
    state: AppState,
}

impl App {
    pub fn new() -> Self {
        let mut file_dialog = egui_file_dialog::FileDialog::new();
        file_dialog.pick_directory();
        App {
            state: AppState::SelectFolder(SelectFolderData {
                file_dialog,
                picked_dir: None,
            }),
        }
    }

    pub async fn update(&mut self) -> Result<()> {
        let new_state = match &mut self.state {
            AppState::SelectFolder(d) => d
                .update()
                .await
                .context("Failed to update SelectFolderData")?,
            AppState::Browse(d) => d.update().await.context("Failed to update BrowseData")?,
        };
        if let Some(state) = new_state {
            self.state = state;
        }
        Ok(())
    }

    pub async fn draw(&mut self) -> Result<()> {
        clear_background(DARKGRAY);
        let new_state = match &mut self.state {
            AppState::SelectFolder(d) => d.draw().await,
            AppState::Browse(d) => d.draw().await.context("Failed to draw BrowseData")?,
        };
        if let Some(state) = new_state {
            self.state = state;
        }
        Ok(())
    }
}

// fn extract_color_to_temp_map(
//     img: &image::RgbaImage,
//     min_temp: f32,
//     max_temp: f32,
//     step: f32,
// ) -> Vec<([u8; 3], f32)> {
//     let x = BAR_X;
//     let y = BAR_MAX;
//     let width = 1.0;
//     let height = BAR_MIN - BAR_MAX;
//
//     let steps = ((max_temp - min_temp) / step).round() as u32;
//     let mut map = Vec::with_capacity((steps + 1) as usize);
//
//     for i in 0..=steps {
//         let offset = ((i as f32 / steps as f32) * height as f32).round() as u32;
//         let py = y + height - offset as f32;
//         let px = x + width as f32 / 2.0;
//
//         let pixel = img.get_pixel(px as u32, py as u32);
//         let rgb = [pixel[0], pixel[1], pixel[2]];
//         let temp = min_temp + i as f32 * step;
//
//         map.push((rgb, temp));
//     }
//
//     map
// }
