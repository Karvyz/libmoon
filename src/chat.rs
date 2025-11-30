use std::sync::{Arc, Mutex};

use futures::StreamExt;
use llm::{
    LLMProvider,
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
};
use log::{error, trace};
use tokio::runtime::Runtime;

use crate::{
    message::{Message, OwnerType},
    persona::{Persona, loader},
    settings::Settings,
};

#[derive(Debug)]
pub struct Chat {
    root: Arc<Mutex<Node>>,
    user: Persona,
    char: Persona,
    settings: Settings,
    runtime: Runtime,

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
            user,
            char,
            settings,
            runtime: Runtime::new().unwrap(),
            messages_ids,
        }
    }

    pub fn add_user_message(&mut self, text: String) {
        self.root
            .lock()
            .unwrap()
            .push(Message::from_user(text, self.messages_ids));
        self.messages_ids += 1;

        // Response from the llm
        self.root
            .lock()
            .unwrap()
            .push(Message::empty_from_char(0, self.messages_ids));
        self.messages_ids += 1;
        self.generate();
    }

    pub fn next(&mut self, depth: usize) {
        if self.root.lock().unwrap().next(depth, self.messages_ids) {
            self.messages_ids += 1;
            self.generate();
        }
    }

    pub fn previous(&mut self, depth: usize) {
        self.root.lock().unwrap().previous(depth);
    }

    pub fn add_edit(&mut self, depth: usize, text: String) {
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

    fn generate(&self) {
        // Initialize and configure the LLM client with streaming enabled
        let llm = self.llm();
        let history: Vec<ChatMessage> = self
            .get_history()
            .into_iter()
            .map(|m| m.to_chat_message())
            .collect();
        let root = self.root.clone();
        self.runtime.spawn(async move {
            match llm.chat_stream(&history).await {
                Err(e) => error!("{}", e),
                Ok(mut stream) => {
                    while let Some(Ok(token)) = stream.next().await {
                        root.lock().unwrap().append_to_last_message(&token);
                    }
                    trace!("Streaming completed.");
                }
            }
        });
    }

    pub fn get_history(&self) -> Vec<Message> {
        let mut history = vec![];
        self.root.lock().unwrap().get_history(&mut history);
        history
    }

    fn llm(&self) -> Box<dyn LLMProvider> {
        LLMBuilder::new()
            .backend(LLMBackend::OpenRouter)
            .api_key(self.settings.api_key.clone())
            .model(self.settings.model.clone())
            .temperature(self.settings.temperature)
            .max_tokens(self.settings.max_tokens)
            .reasoning(self.settings.reasoning)
            .system(self.char.system_prompt(Some(self.user.name())))
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
}
