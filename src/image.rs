use anyhow::{Context, Result, bail};
use image::RgbaImage;
use macroquad::prelude::*;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::map::Map;

#[derive(Debug, Clone)]
pub struct ImageData {
    pub image: RgbaImage,
    pub texture: Texture2D,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub color_temp: Map<[u8; 3], f32>,
}

#[derive(Debug, Clone)]
pub struct Image {
    pub path: PathBuf,
    pub data: Arc<Mutex<Option<ImageData>>>,
}

impl Image {
    pub fn new(path: PathBuf) -> Result<Image> {
        if !path.exists() {
            bail!("Image path does not exist: {}", path.display());
        }

        Ok(Image {
            path,
            data: Arc::new(Mutex::new(None)),
        })
    }

    pub fn load(&self) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        if data.is_some() {
            bail!("Image data is already loaded for: {}", self.path.display());
        }

        let image = image::open(&self.path)
            .with_context(|| format!("Failed to open image: {}", self.path.display()))?
            .into_rgba8();
        let (width, height) = image.dimensions();
        let texture = Texture2D::from_rgba8(width as u16, height as u16, &image);
        let min = 0.0;
        let max = 255.0;
        let step = (max - min) / 256.0;
        let color_temp = Map::new();

        *data = Some(ImageData {
            image,
            texture,
            min,
            max,
            step,
            color_temp,
        });
        Ok(())
    }
}
