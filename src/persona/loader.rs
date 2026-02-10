use anyhow::{Result, anyhow};
use image::{ImageBuffer, Rgba};
use log::{error, trace};
use std::{
    fs::{self, File},
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{
    sync::{Mutex, mpsc},
    time::sleep,
};

use crate::persona::{Persona, card::Card};

pub enum LoaderUpdate {
    Char,
    User,
    Done,
}

pub struct Loader {
    chars: Arc<Mutex<Vec<Persona>>>,
    users: Arc<Mutex<Vec<Persona>>>,
}

impl Loader {
    pub fn new(tx: Option<mpsc::Sender<LoaderUpdate>>) -> Self {
        let chars = Arc::new(Mutex::new(vec![]));
        let tchars = chars.clone();
        let users = Arc::new(Mutex::new(vec![]));
        let tusers = users.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(1000)).await;
            tchars.lock().await.push(Persona::default_char());
            println!("test");
            if let Some(tx) = tx {
                tx.send(LoaderUpdate::Char).await;
            }
        });
        Self { chars, users }
    }
}

pub fn load_chars() -> Vec<Persona> {
    let path = cache_path("chars");
    load_from_cache(path)
}

pub fn load_users() -> Vec<Persona> {
    let path = cache_path("users");
    load_from_cache(path)
}

pub fn load_most_recent_char() -> Option<Persona> {
    let path = cache_path("chars");
    load_most_recent_from_cache(path)
}

pub fn load_most_recent_user() -> Option<Persona> {
    let path = cache_path("users");
    load_most_recent_from_cache(path)
}

pub(crate) fn touch(path: &PathBuf) -> std::io::Result<()> {
    let dest = File::open(path)?;
    dest.set_modified(SystemTime::now())
}

fn load_from_cache(path: PathBuf) -> Vec<Persona> {
    match try_load_dir(path) {
        Ok(personas) => personas,
        Err(e) => {
            error!("{e}");
            vec![]
        }
    }
}

fn load_most_recent_from_cache(path: PathBuf) -> Option<Persona> {
    trace!("Trying to load from {:?}", path);
    match most_recent_dir(path) {
        Ok(most_recent) => match try_load_subdir(most_recent) {
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
            let modified_time = modified_time(&path);
            if modified_time >= most_recent_change {
                most_recent_change = modified_time;
                most_recent_dir = Ok(path)
            }
        }
    }
    most_recent_dir
}

fn try_load_dir(dir: PathBuf) -> Result<Vec<Persona>> {
    trace!("Trying to load {:?}", dir);
    let mut personas = vec![];
    for entry in (fs::read_dir(dir)?).flatten() {
        let path = entry.path();
        if path.is_dir()
            && let Ok(persona) = try_load_subdir(path)
        {
            personas.push(persona);
        }
    }
    Ok(personas)
}

fn try_load_subdir(dir: PathBuf) -> Result<Persona> {
    let modified_time = modified_time(&dir);

    let mut image = Err(anyhow!("Persona not found"));
    let mut persona = Err(anyhow!("Persona not found"));
    for entry in (fs::read_dir(&dir)?).flatten() {
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension()
            && let Some(ext) = ext.to_str()
        {
            match ext {
                "json" => persona = load_persona(path),
                "png" => image = load_image(path),
                _ => (),
            }
        }
    }

    match persona {
        Ok(data) => Ok(Persona::new(
            data,
            match image {
                Ok(image) => Some(image),
                Err(_) => None,
            },
            modified_time,
            dir,
        )),
        Err(_) => Err(anyhow!("Persona not found")),
    }
}

fn load_persona(path: PathBuf) -> Result<Card> {
    let data = fs::read_to_string(&path)?;
    Card::load_from_json(&data)
}

fn load_image(path: PathBuf) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let mut image = crop_to_square(image::open(path)?.to_rgba8());

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
