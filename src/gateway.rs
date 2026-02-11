use anyhow::{Result, anyhow};
use image::{ImageBuffer, Rgba};
use log::{error, trace};
use std::{
    fs::{self, File},
    path::PathBuf,
    sync::Arc,
    time::SystemTime,
};
use tokio::sync::{Mutex, mpsc};

use crate::persona::{Persona, card::Card};

pub enum GatewayUpdate {
    Char,
    User,
}

pub struct Gateway {
    pub chars: Arc<Mutex<Vec<Persona>>>,
    pub users: Arc<Mutex<Vec<Persona>>>,

    rx: mpsc::Receiver<GatewayUpdate>,
}

impl Gateway {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);
        let chars = Arc::new(Mutex::new(vec![]));
        let tchars = chars.clone();
        let users = Arc::new(Mutex::new(vec![]));
        let tusers = users.clone();
        tokio::spawn(async move {
            Self::load_users(tusers, &tx).await;
            Self::load_chars(tchars, &tx).await;
        });
        Self { chars, users, rx }
    }

    pub fn load_most_recent_char() -> Option<Persona> {
        let path = Self::cache_path("chars");
        Self::load_most_recent_from_cache(path)
    }

    pub fn load_most_recent_user() -> Option<Persona> {
        let path = Self::cache_path("users");
        Self::load_most_recent_from_cache(path)
    }

    pub async fn recv(&mut self) -> Option<GatewayUpdate> {
        self.rx.recv().await
    }

    pub(crate) fn touch(path: &PathBuf) -> std::io::Result<()> {
        let dest = File::open(path)?;
        dest.set_modified(SystemTime::now())
    }

    fn load_most_recent_from_cache(path: PathBuf) -> Option<Persona> {
        trace!("Trying to load from {:?}", path);
        match Self::most_recent_dir(path) {
            Ok(most_recent) => match Self::try_load_subdir(most_recent) {
                Ok(persona) => return Some(persona),
                Err(e) => error!("{e}"),
            },
            Err(e) => error!("{e}"),
        }
        None
    }

    fn most_recent_dir(dir: PathBuf) -> Result<PathBuf> {
        let mut most_recent_dir: Result<PathBuf> = Err(anyhow!("No file found in {:?}", dir));
        let mut most_recent_change = SystemTime::UNIX_EPOCH;
        for entry in (fs::read_dir(dir)?).flatten() {
            let path = entry.path();

            if path.is_dir() {
                let modified_time = Self::modified_time(&path);
                if modified_time >= most_recent_change {
                    most_recent_change = modified_time;
                    most_recent_dir = Ok(path)
                }
            }
        }
        most_recent_dir
    }

    async fn load_users(users: Arc<Mutex<Vec<Persona>>>, tx: &mpsc::Sender<GatewayUpdate>) {
        trace!("Trying to load users");
        if let Ok(dir) = fs::read_dir(Self::cache_path("users")) {
            for entry in dir.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && let Ok(persona) = Self::try_load_subdir(path)
                {
                    users.lock().await.push(persona);
                    let _ = tx.try_send(GatewayUpdate::User);
                }
            }
        }
    }

    async fn load_chars(chars: Arc<Mutex<Vec<Persona>>>, tx: &mpsc::Sender<GatewayUpdate>) {
        trace!("Trying to load chars");
        if let Ok(dir) = fs::read_dir(Self::cache_path("chars")) {
            for entry in dir.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && let Ok(persona) = Self::try_load_subdir(path)
                {
                    chars.lock().await.push(persona);
                    let _ = tx.try_send(GatewayUpdate::Char);
                }
            }
        }
    }

    fn try_load_subdir(dir: PathBuf) -> Result<Persona> {
        let modified_time = Self::modified_time(&dir);

        let mut image = Err(anyhow!("Persona not found"));
        let mut persona = Err(anyhow!("Persona not found"));
        for entry in (fs::read_dir(&dir)?).flatten() {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension()
                && let Some(ext) = ext.to_str()
            {
                match ext {
                    "json" => persona = Self::load_persona(path),
                    "png" => image = Self::load_image(path),
                    _ => (),
                }
            }
        }

        match persona {
            Ok(data) => Ok(Persona::new(data, image.ok(), modified_time, dir)),
            Err(_) => Err(anyhow!("Persona not found")),
        }
    }

    fn load_persona(path: PathBuf) -> Result<Card> {
        let data = fs::read_to_string(&path)?;
        Card::load_from_json(&data)
    }

    fn load_image(path: PathBuf) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        let mut image = Self::crop_to_square(image::open(path)?.to_rgba8());

        let (width, height) = image.dimensions();
        let center_x = width as f64 / 2.0;
        let center_y = height as f64 / 2.0;
        let radius = width.min(height) as f64 / 2.0;

        // Process each pixel
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let distance_from_center =
                ((x as f64 - center_x).powi(2) + (y as f64 - center_y).powi(2)).sqrt();

            if distance_from_center > radius {
                pixel[3] = 0
            }
        }
        Ok(image)
    }

    fn crop_to_square(image: ImageBuffer<Rgba<u8>, Vec<u8>>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let (width, height) = image.dimensions();
        let size = width.min(height);

        let x_offset = (width - size) / 2;
        let y_offset = (height - size) / 2;

        image::imageops::crop_imm(&image, x_offset, y_offset, size, size).to_image()
    }

    fn cache_path(subdir: &str) -> PathBuf {
        dirs::cache_dir()
            .map(|mut path| {
                path.push("moon");
                path.push(subdir);
                path
            })
            .unwrap_or_default()
    }

    fn modified_time(path: &PathBuf) -> SystemTime {
        if let Ok(metadata) = fs::metadata(path)
            && let Ok(modified_time) = metadata.modified()
        {
            return modified_time;
        }
        SystemTime::UNIX_EPOCH
    }
}

impl Default for Gateway {
    fn default() -> Self {
        Self::new()
    }
}
