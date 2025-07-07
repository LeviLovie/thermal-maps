use anyhow::{Context, Result};
use egui::{Color32, Grid, RichText};
use macroquad::prelude::*;
use std::path::PathBuf;

use crate::{image::Image, map::Map};

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
    loaded: bool,
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
            loaded: false,
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
            self.scroll += mouse_wheel.1 * 3.0;
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
                if let Some(t) = &image.texture {
                    if mouse_pos.0 >= 0.0
                        && mouse_pos.0 <= t.width() / 2.0
                        && mouse_pos.1 >= y
                        && mouse_pos.1 <= y + t.height() / 2.0
                    {
                        self.selected_image = Some(i);
                        self.hover = Some(mouse_pos.1 - y);
                        break;
                    }
                    y += t.height() / 2.0 + 10.0;
                    if i != self.images.len() - 1 {
                        y += 10.0;
                    }
                }
            }
        }

        // Update images the have an image loaded but not the texture
        // if !self.loaded {
        let mut not_loaded = false;
        for image in self.images.iter_mut() {
            let is_loading = *image.is_loading.lock().unwrap();
            if is_loading {
                not_loaded = true;
            }
            if !is_loading
                && let Some(d) = image.data.lock().unwrap().as_ref()
                && image.texture.is_none()
            {
                image.texture = Some(Texture2D::from_rgba8(
                    d.image.width() as u16,
                    d.image.height() as u16,
                    d.image.as_raw(),
                ));
            }
        }
        if !not_loaded {
            self.loaded = true;
        }

        // Update temperature based on mouse pos from the image
        if let Some(selected) = self.selected_image {
            if let Some(t) = &self.images[selected].texture
                && let Some(d) = self.images[selected].data.lock().unwrap().as_ref()
            {
                let mouse_pos = mouse_position();
                let image_ratio = t.width() / t.height();
                let width = screen_width() - self.max_width - 10.0 - 175.0 - 10.0;
                let height = width / image_ratio;
                let x_offset = self.max_width + 10.0;
                let y_offset = 0.0;

                // Check if mouse is hovering over the image
                if mouse_pos.0 >= x_offset
                    && mouse_pos.0 <= x_offset + width
                    && mouse_pos.1 >= y_offset
                    && mouse_pos.1 <= y_offset + height
                {
                    let pixel_x = (mouse_pos.0 - x_offset) * (t.width() / width);
                    let pixel_y = (mouse_pos.1 - y_offset) * (t.height() / height);

                    if pixel_x < d.image.width() as f32 && pixel_y < d.image.height() as f32 {
                        let pixel = d.image.get_pixel(pixel_x as u32, pixel_y as u32);
                        let rgb = [pixel[0], pixel[1], pixel[2]];
                        self.hover = d.color_temp.get_closest_by(|color| {
                            let d = |a: u8, b: u8| (a as f32 - b as f32).powi(2);
                            d(color[0], rgb[0]) + d(color[1], rgb[1]) + d(color[2], rgb[2])
                        });
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
        let mut y = self.scroll;
        for (i, image) in self.images.iter_mut().enumerate() {
            if !(*image.is_loading.lock().unwrap()) && image.data.lock().unwrap().is_none() {
                image.load().context(format!(
                    "Failed to load image data for {}",
                    image.path.display()
                ))?;
                break;
            }

            if let Some(t) = &image.texture {
                draw_texture_ex(
                    &t,
                    0.0,
                    y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(t.width() / 2.0, t.height() / 2.0)),
                        ..Default::default()
                    },
                );
                if self.selected_image == Some(i) {
                    draw_rectangle_lines(0.0, y, t.width() / 2.0, t.height() / 2.0, 2.0, YELLOW);
                }

                y += t.height() / 2.0 + 10.0;
                self.images_height += t.height() + 10.0;
                if i != images_len - 1 {
                    y += 10.0;
                }
                if self.max_width < t.width() / 2.0 {
                    self.max_width = t.width() / 2.0;
                }
            }
        }

        let max_width = self.max_width;
        if let Some(image) = self.selected_image
            && self.loaded
        {
            if let Some(t) = &self.images[image].texture {
                let image_ratio = t.width() / t.height();
                let width = screen_width() - max_width - 10.0 - 175.0 - 10.0;
                let height = width / image_ratio;
                draw_texture_ex(
                    &t,
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
                .exact_width(175.0)
                .show(egui_ctx, |ui| {
                    let mut new_filtered_texture: Option<Texture2D> = None;
                    if let Some(image) = self.selected_image {
                        if let Some(d) = self.images[image].data.lock().unwrap().as_mut()
                            && let Some(t) = &self.images[image].texture
                        {
                            ui.heading(format!(
                                "{}",
                                self.images[image]
                                    .path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                            ));
                            if !self.loaded {
                                ui.label(
                                    RichText::new("Loading images...")
                                        .size(20.0)
                                        .color(Color32::from_rgb(200, 200, 200)),
                                );
                            }

                            Grid::new("props").show(ui, |ui| {
                                ui.label("Width");
                                ui.label(format!("{} px", t.width() as usize));
                                ui.end_row();

                                ui.label("Height");
                                ui.label(format!("{} px", t.height() as usize));
                                ui.end_row();

                                ui.label("Size");
                                ui.label(format!("{} bytes", d.image.as_raw().len()));
                                ui.end_row();
                            });

                            ui.separator();

                            ui.heading("Colors");
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

                            if ui.button("Extract color map").clicked() {
                                d.color_temp =
                                    extract_color_to_temp_map(&d.image, d.min, d.max, d.step);
                            }

                            ui.separator();

                            ui.add_enabled_ui(d.color_temp.len() > 0, |ui| {
                                ui.heading("Filter");

                                Grid::new("filter").num_columns(4).min_col_width(10.0).show(ui, |ui| {
                                    ui.label("Min");
                                    if ui.checkbox(&mut d.filter_min_enabled, "").clicked() {
                                        d.filter_min = d.min;
                                    }
                                    if !d.filter_min_enabled {
                                        d.filter_min = d.min;
                                    }
                                    ui.add_enabled_ui(d.filter_min_enabled, |ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut d.filter_min).speed(d.step),
                                        );
                                    });
                                    ui.end_row();

                                    ui.label("Max");
                                    if ui.checkbox(&mut d.filter_max_enabled, "").clicked() {
                                        d.filter_max = d.max;
                                    }
                                    if !d.filter_max_enabled {
                                        d.filter_max = d.max;
                                    }
                                    ui.add_enabled_ui(d.filter_max_enabled, |ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut d.filter_max).speed(d.step),
                                        );
                                    });
                                    ui.end_row();
                                });

                                if ui.button("Apply filter").clicked() {
                                    let mut f = d.image.clone();
                                    for row in f.rows_mut() {
                                        for pixel in row {
                                            let temp = d.color_temp.get_closest_by(|color| {
                                                let d =
                                                    |a: u8, b: u8| (a as f32 - b as f32).powi(2);
                                                d(color[0], pixel[0])
                                                    + d(color[1], pixel[1])
                                                    + d(color[2], pixel[2])
                                            });
                                            if let Some(temp) = temp {
                                                if temp < d.filter_min || temp > d.filter_max {
                                                    let r = pixel[0] as u32;
                                                    let g = pixel[1] as u32;
                                                    let b = pixel[2] as u32;

                                                    let gray = (0.299 * r as f32
                                                        + 0.587 * g as f32
                                                        + 0.114 * b as f32)
                                                        as u8;

                                                    pixel[0] = gray;
                                                    pixel[1] = gray;
                                                    pixel[2] = gray;
                                                }
                                            }
                                        }
                                    }
                                    new_filtered_texture = Some(Texture2D::from_rgba8(
                                        f.width() as u16,
                                        f.height() as u16,
                                        f.as_raw(),
                                    ));
                                }
                            });

                            ui.separator();

                            if let Some(hover) = self.hover {
                                ui.label(RichText::new(format!("Hover: {:.2}°C ", hover)));
                            }
                        }
                    } else {
                        ui.label(RichText::new("No image selected"));
                    }

                    if let Some(new_texture) = new_filtered_texture {
                        self.images[self.selected_image.unwrap()].texture = Some(new_texture);
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

const BAR_X: f32 = 290.0;
const BAR_MAX: f32 = 60.0;
const BAR_MIN: f32 = 191.0;

fn extract_color_to_temp_map(
    img: &image::RgbaImage,
    min_temp: f32,
    max_temp: f32,
    step: f32,
) -> Map<[u8; 3], f32> {
    let x = BAR_X;
    let y = BAR_MAX;
    let width = 1.0;
    let height = BAR_MIN - BAR_MAX;

    let steps = ((max_temp - min_temp) / step).round() as u32;
    let mut map = Map::new();

    for i in 0..=steps {
        let offset = ((i as f32 / steps as f32) * height as f32).round() as u32;
        let py = y + height - offset as f32;
        let px = x + width as f32 / 2.0;

        let pixel = img.get_pixel(px as u32, py as u32);
        let rgb = [pixel[0], pixel[1], pixel[2]];
        let temp = min_temp + i as f32 * step;

        map.push(rgb, temp);
    }

    map
}
