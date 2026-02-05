use tokio::sync::mpsc;

use crate::{
    chat::{Chat, ChatUpdate},
    persona::{Persona, loader::load_most_recent_user},
    settings::Settings,
};

pub enum MoonUpdate {
    CU(ChatUpdate),
    Error(String),
}

pub struct Moon {
    tx: mpsc::Sender<ChatUpdate>,
    rx: mpsc::Receiver<ChatUpdate>,

    pub chat: Chat,
    pub settings: Settings,
}

impl Default for Moon {
    fn default() -> Self {
        Self::new()
    }
}

impl Moon {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);

        let settings = Settings::load();
        let user = load_most_recent_user().unwrap_or(Persona::default_user());
        let mut chat = Chat::with_personas(user, Persona::default_char(), settings.clone());
        chat.set_tx(tx.clone());
        Self {
            tx,
            rx,
            chat: Chat::with_personas(
                Persona::default_user(),
                Persona::default_char(),
                settings.clone(),
            ),
            settings,
        }
    }

    pub fn set_chars(&mut self, char: Persona) {
        let user = self.chat.user();
        self.chat = Chat::with_personas(user, char, self.settings.clone());
        self.chat.set_tx(self.tx.clone());
    }

    pub fn get_settings(&self) -> Settings {
        self.settings.clone()
    }

    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
        self.chat.set_settings(self.settings.clone());
    }

    pub async fn recv(&mut self) -> MoonUpdate {
        match self.rx.recv().await {
            Some(cu) => MoonUpdate::CU(cu),
            None => MoonUpdate::Error(String::new()),
        }
    }
}
