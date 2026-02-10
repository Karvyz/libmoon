use std::{fmt::Debug, ops::Deref, path::PathBuf, time::SystemTime};

use image::{ImageBuffer, Rgba};
use log::error;

use crate::persona::card::Card;

mod card;
pub mod loader;

#[derive(Clone)]
pub struct Persona {
    data: Card,
    image: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    modified_time: SystemTime,
    path: PathBuf,
}

impl Debug for Persona {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Persona")
            .field("char", &self.data.name())
            .finish()
    }
}

impl Deref for Persona {
    type Target = Card;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl Persona {
    pub fn new(
        data: Card,
        image: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
        modified_time: SystemTime,
        path: PathBuf,
    ) -> Self {
        Persona {
            data,
            image,
            modified_time,
            path,
        }
    }

    pub fn default_user() -> Self {
        Self {
            data: Card::basic("User", ""),
            image: None,
            modified_time: SystemTime::now(),
            path: PathBuf::new(),
        }
    }

    pub fn default_char() -> Self {
        Self {
            data: Card::basic("Luna", "You are Luna, an helpfull AI assistant."),
            image: None,

            modified_time: SystemTime::now(),

            path: PathBuf::new(),
        }
    }

    // pub fn save(&self, path: PathBuf) -> Result<(), Box<dyn Error>> {
    //     if !path.exists() {
    //         fs::create_dir_all(&path)?;
    //     }
    //
    //     let config_path = path.join(format!("{}.json", self.name()));
    //     let content = match &self.ptype {
    //         PType::Basic(basic) => serde_json::to_string_pretty(basic)?,
    //         PType::Card(card) => serde_json::to_string_pretty(card)?,
    //     };
    //     fs::write(config_path, content)?;
    //
    //     Ok(())
    // }

    pub fn image(&self) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.image.clone()
    }

    pub fn raw_image(&self) -> Option<(u32, u32, Vec<u8>)> {
        match self.image.clone() {
            Some(image) => {
                let (width, height) = image.dimensions();
                let content = image.to_vec();
                Some((width, height, content))
            }
            None => None,
        }
    }

    pub fn modified_time(&self) -> SystemTime {
        self.modified_time
    }

    pub fn set_modified_time(&mut self) {
        self.modified_time = SystemTime::now();
        if let Err(e) = loader::touch(&self.path) {
            error!("{e}");
        }
    }

    pub fn replace_names(s: &str, self_name: &str, partner_name: Option<&str>) -> String {
        let replaced_char_name = s.replace("{{char}}", self_name);
        match partner_name {
            Some(name) => replaced_char_name.replace("{{user}}", name),
            None => replaced_char_name,
        }
    }
}
