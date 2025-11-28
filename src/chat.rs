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
    message::Message,
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
        for greeting in char.greetings(Some(user.name())) {
            root.messages.push(Message::from_char(0, greeting));
            root.childs.push(Node::new());
        }
        Chat {
            root: Arc::new(Mutex::new(root)),
            user,
            char,
            settings,
            runtime: Runtime::new().unwrap(),
        }
    }

    pub fn add_user_message(&mut self, text: String) {
        self.root.lock().unwrap().push(Message::from_user(text));

        // Response from the llm
        self.root.lock().unwrap().push(Message::empty_from_char(0));
        self.generate();
    }

    pub fn next(&mut self, depth: usize) {
        if self.root.lock().unwrap().next(depth) {
            self.generate();
        }
    }

    pub fn previous(&mut self, depth: usize) {
        self.root.lock().unwrap().previous(depth);
    }

    fn generate(&self) {
        // Initialize and configure the LLM client with streaming enabled
        let llm = self.llm();
        let history = self.get_history();
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

    pub fn get_history(&self) -> Vec<ChatMessage> {
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

    pub fn get_history(&self, history: &mut Vec<ChatMessage>) {
        if !self.messages.is_empty() {
            history.push(self.messages[self.selected].to_chat_message());
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

    fn next(&mut self, depth: usize) -> bool {
        match depth == 0 {
            true => match self.selected + 1 >= self.messages.len() {
                true => {
                    self.messages
                        .push(self.messages[self.selected].create_brother());
                    self.childs.push(Node::new());
                    self.selected += 1;
                    true
                }
                false => {
                    self.selected += 1;
                    false
                }
            },
            false => self.childs[self.selected].next(depth - 1),
        }
    }
}
