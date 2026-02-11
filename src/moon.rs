use tokio::sync::mpsc;

use crate::{
    chat::{Chat, ChatUpdate},
    gateway::{Gateway, GatewayUpdate},
    persona::Persona,
    settings::Settings,
};

pub enum MoonUpdate {
    CU(ChatUpdate),
    GU(GatewayUpdate),
    Error(String),
}

pub struct Moon {
    ctx: mpsc::Sender<ChatUpdate>,
    crx: mpsc::Receiver<ChatUpdate>,

    pub chat: Chat,
    pub settings: Settings,
    pub gateway: Gateway,
}

impl Default for Moon {
    fn default() -> Self {
        Self::new()
    }
}

impl Moon {
    pub fn new() -> Self {
        let gateway = Gateway::new();
        let (ctx, crx) = mpsc::channel(10);

        let settings = Settings::load();
        let user = Gateway::load_most_recent_user().unwrap_or(Persona::default_user());
        let mut chat = Chat::with_personas(user, Persona::default_char(), settings.clone());
        chat.set_tx(ctx.clone());
        Self {
            ctx,
            crx,
            chat: Chat::with_personas(
                Persona::default_user(),
                Persona::default_char(),
                settings.clone(),
            ),
            settings,
            gateway,
        }
    }

    pub fn set_chars(&mut self, char: Persona) {
        let user = self.chat.user();
        self.chat = Chat::with_personas(user, char, self.settings.clone());
        self.chat.set_tx(self.ctx.clone());
    }

    pub fn get_settings(&self) -> Settings {
        self.settings.clone()
    }

    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = settings;
        self.chat.set_settings(self.settings.clone());
    }

    pub async fn recv(&mut self) -> MoonUpdate {
        tokio::select! {
            Some(update) = self.crx.recv() => MoonUpdate::CU(update),
            Some(update) = self.gateway.recv() => MoonUpdate::GU(update),
        }
    }
}
