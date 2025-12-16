use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use futures::StreamExt;
use image::{ImageBuffer, Rgba};
use llm::{
    LLMProvider,
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
};
use log::{error, trace};
use tokio::sync::mpsc;

use crate::{
    message::{Message, OwnerType},
    persona::{Persona, loader},
    settings::Settings,
};

pub enum ChatUpdate {
    MessageCreated,
    StreamUpdate,
    StreamFinished,
    Error(String),
}

#[derive(Debug)]
pub struct Chat {
    root: Arc<Mutex<Node>>,
    personas: Vec<Persona>,
    settings: Settings,
    tx: Option<mpsc::Sender<ChatUpdate>>,

    messages_ids: usize,
}

impl Chat {
    pub fn load() -> Self {
        let user = loader::load_most_recent_user().unwrap_or(Persona::default_user());
        let char = loader::load_most_recent_char().unwrap_or(Persona::default_char());
        let settings = Settings::load();
        Self::with_personas(user, char, settings)
    }

    pub fn with_personas(user: Persona, char: Persona, settings: Settings) -> Self {
        let mut root = Node::new();
        let mut messages_ids = 0;
        for greeting in char.greetings(Some(user.name())) {
            root.messages
                .push(Message::from_char(0, greeting, messages_ids));
            root.childs.push(Node::new());
            messages_ids += 1;
        }

        Chat {
            root: Arc::new(Mutex::new(root)),
            personas: vec![user, char],
            settings,
            tx: None,
            messages_ids,
        }
    }

    pub fn set_tx(&mut self, tx: mpsc::Sender<ChatUpdate>) {
        self.tx = Some(tx);
    }

    pub fn get_rx(&mut self) -> mpsc::Receiver<ChatUpdate> {
        let (tx, rx) = mpsc::channel(10);
        self.tx = Some(tx);
        rx
    }

    pub fn user(&self) -> Persona {
        self.personas[0].clone()
    }

    pub fn title(&self) -> String {
        format!(
            "{}'s chat with {}",
            self.personas[0].name(),
            self.personas[1].name()
        )
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: Settings) {
        trace!("Settings changed");
        self.settings = settings;
        let _ = self.settings.save();
    }

    pub fn owner_name(&self, message: &Message) -> &str {
        self.personas[usize::from(message.owner)].name()
    }

    pub fn message_image(&self, message: &Message) -> Option<Rc<ImageBuffer<Rgba<u8>, Vec<u8>>>> {
        self.personas[usize::from(message.owner)].image()
    }

    pub fn raw_images(&self) -> Vec<Option<(u32, u32, Vec<u8>)>> {
        let mut raw_images = vec![];
        for p in &self.personas {
            raw_images.push(p.raw_image())
        }
        raw_images
    }

    pub fn add_user_message(&mut self, text: String) {
        let text = text.trim().to_string();
        if !text.is_empty() {
            trace!("Adding user Message");
            self.root
                .lock()
                .unwrap()
                .push(Message::from_user(text, self.messages_ids));
            self.messages_ids += 1;
        }

        // Response from the llm
        trace!("Adding char response");
        self.root
            .lock()
            .unwrap()
            .push(Message::empty_from_char(0, self.messages_ids));
        self.messages_ids += 1;
        self.generate();
    }

    pub fn next(&mut self, depth: usize) {
        trace!("Next depth {depth}");
        if self.root.lock().unwrap().next(depth, self.messages_ids) {
            trace!("Adding char response");
            self.messages_ids += 1;
            self.generate();
        }
    }

    pub fn previous(&mut self, depth: usize) {
        trace!("Next depth {depth}");
        self.root.lock().unwrap().previous(depth);
    }

    pub fn add_edit(&mut self, depth: usize, text: String) {
        let text = text.trim().to_string();
        trace!("Adding new edit depth {depth}");
        let added_response = self
            .root
            .lock()
            .unwrap()
            .add_edit(depth, self.messages_ids, text);
        self.messages_ids += 1;
        if added_response {
            self.messages_ids += 1;
            self.generate();
        }
    }

    pub fn delete(&mut self, depth: usize) {
        trace!("Deleting depth {depth}");
        self.root.lock().unwrap().delete(depth);
    }

    fn generate(&mut self) {
        // Initialize and configure the LLM client with streaming enabled
        let llm = self.llm();
        let mut history: Vec<ChatMessage> = self
            .get_history()
            .into_iter()
            .map(|m| m.to_chat_message())
            .collect();
        history.pop();
        let root = self.root.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            match llm.chat_stream(&history).await {
                Err(e) => {
                    error!("{}", e);
                    Self::send_update(&tx, ChatUpdate::Error(e.to_string())).await;
                }
                Ok(mut stream) => {
                    Self::send_update(&tx, ChatUpdate::MessageCreated).await;
                    while let Some(Ok(token)) = stream.next().await {
                        root.lock().unwrap().append_to_last_message(&token);
                        Self::send_update(&tx, ChatUpdate::StreamUpdate).await;
                    }
                    trace!("Streaming completed.");
                    Self::send_update(&tx, ChatUpdate::StreamFinished).await;
                }
            }
        });
    }

    async fn send_update(tx: &Option<mpsc::Sender<ChatUpdate>>, cu: ChatUpdate) {
        if let Some(tx) = tx {
            tx.send(cu).await.unwrap();
        }
    }

    pub fn get_history(&self) -> Vec<Message> {
        let mut history = vec![];
        self.root.lock().unwrap().get_history(&mut history);
        history
    }

    pub fn get_history_structure(&self) -> Vec<(usize, usize)> {
        let mut structure = vec![];
        self.root
            .lock()
            .unwrap()
            .get_history_structure(&mut structure);
        structure
    }

    fn llm(&self) -> Box<dyn LLMProvider> {
        LLMBuilder::new()
            .backend(LLMBackend::OpenRouter)
            .api_key(self.settings.api_key.clone())
            .model(self.settings.model.clone())
            .temperature(self.settings.temperature)
            .max_tokens(self.settings.max_tokens)
            .reasoning(self.settings.reasoning)
            .system(self.personas[1].system_prompt(Some(self.personas[0].name())))
            .build()
            .expect("Failed to build LLM (Openrouter)")
    }
}

#[derive(Debug)]
struct Node {
    messages: Vec<Message>,
    childs: Vec<Node>,
    selected: usize,
}

impl Node {
    fn new() -> Self {
        Node {
            messages: vec![],
            childs: vec![],
            selected: 0,
        }
    }

    fn push(&mut self, message: Message) {
        match self.childs.is_empty() {
            true => {
                self.messages.push(message);
                self.childs.push(Node::new());
            }
            false => self.childs[self.selected].push(message),
        }
    }

    fn append_to_last_message(&mut self, text: &str) {
        if self.messages.is_empty() {
            return;
        }

        match self.childs[self.selected].childs.is_empty() {
            true => self.messages[self.selected].text.push_str(text),
            false => self.childs[self.selected].append_to_last_message(text),
        }
    }

    pub fn get_history(&self, history: &mut Vec<Message>) {
        if !self.messages.is_empty() {
            history.push(self.messages[self.selected].clone());
            self.childs[self.selected].get_history(history);
        }
    }

    pub fn get_history_structure(&self, structure: &mut Vec<(usize, usize)>) {
        if !self.messages.is_empty() {
            structure.push((self.selected + 1, self.messages.len()));
            self.childs[self.selected].get_history_structure(structure);
        }
    }

    fn previous(&mut self, depth: usize) {
        match depth == 0 {
            true => {
                if self.selected > 0 {
                    self.selected -= 1
                }
            }
            false => self.childs[self.selected].previous(depth - 1),
        }
    }

    fn next(&mut self, depth: usize, ids: usize) -> bool {
        match depth == 0 {
            true => match self.selected + 1 >= self.messages.len() {
                true => {
                    self.messages
                        .push(self.messages[self.selected].create_brother(ids));
                    self.childs.push(Node::new());
                    self.selected += 1;
                    true
                }
                false => {
                    self.selected += 1;
                    false
                }
            },
            false => self.childs[self.selected].next(depth - 1, ids),
        }
    }

    pub fn add_edit(&mut self, depth: usize, ids: usize, text: String) -> bool {
        match depth == 0 {
            true => {
                let mut added_response = false;
                let mut edit = self.messages[self.selected].create_brother(ids);
                edit.text = text;
                self.messages.push(edit);
                let mut new_node = Node::new();
                if let OwnerType::User = self.messages[self.selected].owner {
                    new_node.messages.push(Message::empty_from_char(0, ids + 1));
                    new_node.childs.push(Node::new());
                    added_response = true;
                }
                self.childs.push(new_node);
                self.selected = self.messages.len() - 1;
                added_response
            }
            false => self.childs[self.selected].add_edit(depth - 1, ids, text),
        }
    }

    fn delete(&mut self, depth: usize) {
        match depth == 0 {
            true => {
                self.messages.remove(self.selected);
                self.childs.remove(self.selected);
                if self.selected > 0 {
                    self.selected -= 1
                }
            }
            false => self.childs[self.selected].delete(depth - 1),
        }
    }
}
