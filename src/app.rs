use egui::{Color32, Grid, RichText};
use macroquad::prelude::*;
use std::path::PathBuf;

const BAR_X: f32 = 290.0;
const BAR_MAX: f32 = 60.0;
const BAR_MIN: f32 = 191.0;

#[derive(Debug)]
pub struct SelectFolderData {
    file_dialog: egui_file_dialog::FileDialog,
    picked_file: Option<PathBuf>,
}

#[derive(Debug)]
pub struct BrowseData {
    images: Vec<(PathBuf, Option<image::RgbaImage>, Option<Texture2D>)>,
    images_height: f32,
    max_width: f32,
    scroll: f32,
    selected_image: Option<usize>,
    min: f32,
    max: f32,
    step: f32,
    color_map: Option<Vec<([u8; 3], f32)>>,
    hover_temp: Option<([u8; 3], f32)>,
}

impl BrowseData {
    pub fn new(path: PathBuf) -> Self {
        let mut images = Vec::new();
        for entry in std::fs::read_dir(path.clone()).expect("Failed to read selected directory") {
            if let Ok(entry) = entry {
                if let Some(ext) = entry.path().extension() {
                    if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "bmp" {
                        images.push((entry.path(), None, None));
                    }
                }
            }
        }
        BrowseData {
            images,
            images_height: 0.0,
            max_width: 0.0,
            scroll: 0.0,
            selected_image: None,
            min: 0.0,
            max: 30.0,
            step: 1.0,
            color_map: None,
            hover_temp: None,
        }
    }
}

#[derive(Debug)]
pub enum AppState {
    SelectFolder(SelectFolderData),
    Browse(BrowseData),
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
                picked_file: None,
            }),
        }
    }

    pub async fn update(&mut self) {
        match &self.state {
            AppState::SelectFolder(_) => self.update_select_folder().await,
            AppState::Browse(_) => self.update_browse().await,
        }
    }

    pub async fn draw(&mut self) {
        clear_background(DARKGRAY);
        match &self.state {
            AppState::SelectFolder(_) => self.draw_select_folder().await,
            AppState::Browse(_) => self.draw_browse().await,
        }
    }

    pub async fn update_select_folder(&mut self) {}

    pub async fn draw_select_folder(&mut self) {
        let mut new_state: Option<AppState> = None;
        let data: &mut SelectFolderData = match &mut self.state {
            AppState::SelectFolder(data) => data,
            _ => return, // Ensure we are in the correct state
        };

        egui_macroquad::ui(|egui_ctx| {
            data.file_dialog.update(egui_ctx);

            if let Some(path) = data.file_dialog.take_picked() {
                data.picked_file = Some(path.to_path_buf());
                new_state = Some(AppState::Browse(BrowseData::new(path.to_path_buf())));
            }
        });
        egui_macroquad::draw();

        if let Some(new_state) = new_state {
            self.state = new_state;
        }
    }

    pub async fn update_browse(&mut self) {
        let data: &mut BrowseData = match &mut self.state {
            AppState::Browse(data) => data,
            _ => return, // Ensure we are in the correct state
        };

        let mouse_wheel = mouse_wheel();
        if mouse_wheel.1 != 0.0 {
            data.scroll += mouse_wheel.1 * 10.0;
            if data.scroll > 0.0 {
                data.scroll = 0.0; // Prevent scrolling above the top
            }
            if data.images_height > 0.0 {
                let screen_height = screen_height();
                if data.scroll < -(data.images_height - screen_height) {
                    data.scroll = -(data.images_height - screen_height);
                }
            }
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let mouse_pos = mouse_position();
            let mut y = data.scroll;
            for (i, image) in data.images.iter().enumerate() {
                if let Some(tex) = &image.2 {
                    if mouse_pos.0 >= 0.0
                        && mouse_pos.0 <= tex.width() / 2.0
                        && mouse_pos.1 >= y
                        && mouse_pos.1 <= y + tex.height() / 2.0
                    {
                        data.selected_image = Some(i);
                        break;
                    }
                    y += tex.height() / 2.0 + 10.0;
                    if i != data.images.len() - 1 {
                        y += 10.0;
                    }
                }
            }
        }

        if let Some(selected) = data.selected_image {
            if data.color_map.is_some() {
                let mouse_pos = mouse_position();
                if let Some(tex) = &data.images[selected].2 {
                    let image_ratio = tex.width() as f32 / tex.height() as f32;
                    let width = screen_width() - data.max_width - 10.0 - 300.0 - 10.0;
                    let height = width / image_ratio;
                    let x_offset = data.max_width + 10.0;
                    let y_offset = 0.0;
                    if mouse_pos.0 >= x_offset
                        && mouse_pos.0 <= x_offset + width
                        && mouse_pos.1 >= y_offset
                        && mouse_pos.1 <= y_offset + height
                    {
                        let pixel_x =
                            ((mouse_pos.0 - x_offset) * (tex.width() as f32 / width)) as u32;
                        let pixel_y =
                            ((mouse_pos.1 - y_offset) * (tex.height() as f32 / height)) as u32;

                        if let Some(img) = &data.images[selected].1 {
                            if pixel_x < img.width() && pixel_y < img.height() {
                                if let Some(color_map) = &data.color_map {
                                    let pixel = img.get_pixel(pixel_x, pixel_y);
                                    let temp = get_temp([pixel[0], pixel[1], pixel[2]], color_map);
                                    data.hover_temp = Some(([pixel[0], pixel[1], pixel[2]], temp));
                                }
                            }
                        }
                    } else {
                        data.hover_temp = None; // Reset hover temp if not hovering over the image
                    }
                }
            }
        }
    }

    pub async fn draw_browse(&mut self) {
        let data: &mut BrowseData = match &mut self.state {
            AppState::Browse(data) => data,
            _ => return, // Ensure we are in the correct state
        };

        data.images_height = 0.0;
        let images_len = data.images.len();
        let selected_image = data.selected_image;
        let mut y = data.scroll;
        for (i, image) in data.images.iter_mut().enumerate() {
            if let Some(tex) = &image.2 {
                draw_texture_ex(
                    tex,
                    0.0,
                    y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(tex.width() / 2.0, tex.height() / 2.0)),
                        ..Default::default()
                    },
                );
                if selected_image == Some(i) {
                    draw_rectangle_lines(
                        0.0,
                        y,
                        tex.width() / 2.0,
                        tex.height() / 2.0,
                        2.0,
                        YELLOW,
                    );
                }
                y += tex.height() / 2.0 + 10.0;
                data.images_height += tex.height() + 10.0;
                if i != images_len - 1 {
                    y += 10.0;
                }
                if data.max_width < tex.width() / 2.0 {
                    data.max_width = tex.width() / 2.0;
                }
            } else {
                let img = image::ImageReader::open(&image.0)
                    .expect("Failed to open image")
                    .decode()
                    .expect("Failed to decode image");
                let width = img.width();
                let height = img.height();
                let image_data = img.to_rgba8();
                let texture2d = Texture2D::from_rgba8(width as u16, height as u16, &image_data);
                image.1 = Some(image_data);
                image.2 = Some(texture2d);
            }
        }

        let max_width = data.max_width;
        if let Some(image) = data.selected_image {
            if let Some(tex) = &data.images[image].2 {
                let image_ratio = tex.width() as f32 / tex.height() as f32;
                let width = screen_width() - max_width - 10.0 - 300.0 - 10.0;
                let height = width / image_ratio;
                draw_texture_ex(
                    tex,
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
                    if let Some(image) = data.selected_image {
                        if let Some(_tex) = &data.images[image].2 {
                            ui.label(
                                RichText::new(format!(
                                    "{}",
                                    data.images[image]
                                        .0
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                ))
                                .size(20.0),
                            );

                            Grid::new("props").show(ui, |ui| {
                                ui.label("Width");
                                ui.label(format!(
                                    "{} px",
                                    data.images[image]
                                        .2
                                        .as_ref()
                                        .map_or(0, |tex| tex.width() as usize)
                                ));
                                ui.end_row();

                                ui.label("Height");
                                ui.label(format!(
                                    "{} px",
                                    data.images[image]
                                        .2
                                        .as_ref()
                                        .map_or(0, |tex| tex.height() as usize)
                                ));
                                ui.end_row();

                                ui.label("Size");
                                ui.label(format!(
                                    "{} bytes",
                                    data.images[image].0.metadata().map_or(0, |m| m.len())
                                ));
                                ui.end_row();
                            });

                            ui.separator();

                            Grid::new("controls").num_columns(2).show(ui, |ui| {
                                ui.label("Min");
                                ui.add(egui::DragValue::new(&mut data.min).speed(data.step));
                                ui.end_row();

                                ui.label("Max");
                                ui.add(egui::DragValue::new(&mut data.max).speed(data.step));
                                ui.end_row();

                                ui.label("Step");
                                ui.add(egui::DragValue::new(&mut data.step).speed(0.1));
                                ui.end_row();
                            });

                            if let Some(hover_temp) = data.hover_temp {
                                ui.label(RichText::new(format!(
                                    "Hover: RGB({},{},{}) -> {:.2}°C ",
                                    hover_temp.0[0], hover_temp.0[1], hover_temp.0[2], hover_temp.1,
                                )));
                            }

                            if ui.button("Extract color map").clicked() {
                                if let Some(img) = &data.images[image].1 {
                                    data.color_map = Some(extract_color_to_temp_map(
                                        img, data.min, data.max, data.step,
                                    ));
                                    if let Some(color_map) = &data.color_map {
                                        for (color, temp) in color_map.iter() {
                                            println!(
                                                "{temp}°C -> RGB({},{},{})",
                                                color[0], color[1], color[2]
                                            );
                                        }
                                    }
                                } else {
                                    ui.label(RichText::new("No image loaded"));
                                }
                            }
                        }
                    } else {
                        ui.label(RichText::new("No image selected"));
                    }
                });

            if let Some(hover_temp) = &data.hover_temp {
                let mouse_pos = mouse_position();
                egui::Window::new("Temp").collapsible(false).resizable(false).fixed_pos([mouse_pos.0 + 20.0, mouse_pos.1 + 20.0]).title_bar(false).show(egui_ctx, |ui| {
                    ui.label(RichText::new(format!("{:.2}°C", hover_temp.1)).color(Color32::from_rgb(255, 255, 255)).size(20.0));
                });
            }
        });
        egui_macroquad::draw();
    }
}

fn extract_color_to_temp_map(
    img: &image::RgbaImage,
    min_temp: f32,
    max_temp: f32,
    step: f32,
) -> Vec<([u8; 3], f32)> {
    let x = BAR_X;
    let y = BAR_MAX;
    let width = 1.0;
    let height = BAR_MIN - BAR_MAX;

    let steps = ((max_temp - min_temp) / step).round() as u32;
    let mut map = Vec::with_capacity((steps + 1) as usize);

    for i in 0..=steps {
        let offset = ((i as f32 / steps as f32) * height as f32).round() as u32;
        println!("Step {i}: offset {offset}");
        let py = y + height - offset as f32;
        let px = x + width as f32 / 2.0;

        let pixel = img.get_pixel(px as u32, py as u32);
        println!(
            "Step {i}: pixel at ({}, {}) -> RGB({}, {}, {})",
            px, py, pixel[0], pixel[1], pixel[2]
        );
        let rgb = [pixel[0], pixel[1], pixel[2]];
        let temp = min_temp + i as f32 * step;

        map.push((rgb, temp));
    }

    map
}

fn get_temp(color: [u8; 3], color_map: &Vec<([u8; 3], f32)>) -> f32 {
    color_map
        .iter()
        .min_by_key(|(c, _)| {
            let r_diff = (c[0] as i32 - color[0] as i32).abs();
            let g_diff = (c[1] as i32 - color[1] as i32).abs();
            let b_diff = (c[2] as i32 - color[2] as i32).abs();
            r_diff + g_diff + b_diff
        })
        .map_or(0.0, |(_, temp)| *temp)
}
