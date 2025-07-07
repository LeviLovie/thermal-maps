use anyhow::{bail, Result};
use image::RgbaImage;
use macroquad::prelude::*;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::map::Map;

#[derive(Debug, Clone)]
pub struct ImageData {
    pub raw_image: RgbaImage,
    pub image: RgbaImage,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub filter_min_enabled: bool,
    pub filter_min: f32,
    pub filter_max_enabled: bool,
    pub filter_max: f32,
    pub color_temp: Map<[u8; 3], f32>,
}

#[derive(Debug, Clone)]
pub struct Image {
    pub path: PathBuf,
    pub is_loading: Arc<Mutex<bool>>,
    pub texture: Option<Texture2D>,
    pub data: Arc<Mutex<Option<ImageData>>>,
}

impl Image {
    pub fn new(path: PathBuf) -> Result<Image> {
        if !path.exists() {
            bail!("Image path does not exist: {}", path.display());
        }

        Ok(Image {
            path,
            is_loading: Arc::new(Mutex::new(false)),
            texture: None,
            data: Arc::new(Mutex::new(None)),
        })
    }

    pub fn load(&mut self) -> Result<()> {
        let mut is_loading = self.is_loading.lock().unwrap();
        if *is_loading {
            return Ok(());
        }
        *is_loading = true;

        let path = self.path.clone();
        let data = Arc::clone(&self.data);
        let is_loading = Arc::clone(&self.is_loading);

        std::thread::spawn(move || {
            let mut data = data.lock().unwrap();
            if data.is_some() {
                eprintln!("Image data is already loaded for: {}", path.display());
                return;
            }

            let image = match image::open(&path) {
                Ok(img) => img.into_rgba8(),
                Err(e) => {
                    eprintln!("Failed to open image {}: {e}", path.display());
                    return;
                }
            };

            let min = 10.0;
            let max = 30.0;
            let step = 1.0;
            let color_temp = Map::new();

            *data = Some(ImageData {
                raw_image: image.clone(),
                image,
                min,
                max,
                step,
                color_temp,
                filter_min_enabled: false,
                filter_min: min,
                filter_max_enabled: false,
                filter_max: max,
            });
            *is_loading.lock().unwrap() = false;
        });

        Ok(())
    }
}
