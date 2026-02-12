use std::{
    hash::{DefaultHasher, Hash, Hasher},
    time::SystemTime,
    vec,
};

use llm::chat::ChatMessage;
use regex::Regex;

#[derive(Debug, Copy, Clone)]
pub enum OwnerType {
    User,
    Char(usize),
}

impl From<OwnerType> for usize {
    fn from(value: OwnerType) -> Self {
        match value {
            OwnerType::User => 0,
            OwnerType::Char(i) => i + 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub owner: OwnerType,
    pub owner_name: String,
    pub text: String,
    timestamp: SystemTime,
}

impl Message {
    pub fn from_user(owner_name: String, mut text: String) -> Self {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        Message {
            owner: OwnerType::User,
            owner_name,
            text,
            timestamp: SystemTime::now(),
        }
    }

    pub fn from_char(char_id: usize, owner_name: String, mut text: String) -> Self {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        Message {
            owner: OwnerType::Char(char_id),
            owner_name,
            text,
            timestamp: SystemTime::now(),
        }
    }

    pub fn empty_from_char(char_id: usize, owner_name: String) -> Self {
        Self::from_char(char_id, owner_name, String::new())
    }

    pub fn to_chat_message(&self) -> ChatMessage {
        match self.owner {
            OwnerType::User => ChatMessage::user().content(&self.text).build(),
            OwnerType::Char(_) => ChatMessage::assistant().content(&self.text).build(),
        }
    }

    pub fn create_brother(&self) -> Self {
        Message {
            owner: self.owner,
            owner_name: self.owner_name.clone(),
            text: String::new(),
            timestamp: SystemTime::now(),
        }
    }

    pub fn id(&self) -> usize {
        let mut hasher = DefaultHasher::new();
        self.timestamp.hash(&mut hasher);
        hasher.finish() as usize
    }

    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }

    pub fn clean(&self) -> String {
        // Remove markdown images
        let image_re = Regex::new(r"!\[[^\]]*\]\([^)]*\)[ \t\r\n]*").unwrap();
        let no_images = image_re.replace_all(&self.text, "");
        // Replace bullshit linebreaks
        let re_newlines = Regex::new(r"[ \t\r]*\n[ \t\r]*").unwrap();
        let one_linebreaks = re_newlines.replace_all(&no_images, "\n").to_string();

        // Trim whitespace from start and end and put a single one at the end
        let mut cleaned = one_linebreaks.trim().to_string();
        cleaned.push('\n');
        cleaned
    }

    pub fn spans(&self) -> Vec<Vec<(String, Style)>> {
        let mut spans = vec![];
        for s in self.clean().split('\n') {
            let line = Self::line(s);
            if !line.is_empty() {
                spans.push(line);
            }
        }
        spans
    }

    fn line(text: &str) -> Vec<(String, Style)> {
        let mut line = vec![];
        let mut cs = Style::Normal;
        let mut ct = String::new();
        for ch in text.chars() {
            let (ns, push_next) = cs.next(ch);
            match ns != cs {
                true => {
                    push_next.then(|| ct.push(ch));
                    (!ct.is_empty()).then(|| line.push((ct, cs)));
                    ct = String::new();
                    (!push_next).then(|| ct.push(ch));
                }
                false => ct.push(ch),
            }
            cs = ns;
        }
        (!ct.is_empty()).then(|| line.push((ct, cs)));
        line
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Style {
    Normal,
    Strong,
    Quote,
    StrongQuote,
}

impl Style {
    fn next(self, ch: char) -> (Style, bool) {
        let mut push_next = self != Style::Normal;
        let ns = match ch {
            '*' => match self {
                Style::Normal => Style::Strong,
                Style::Strong => Style::Normal,
                Style::Quote => Style::StrongQuote,
                Style::StrongQuote => Style::Quote,
            },
            '"' | '“' | '”' => match self {
                Style::Normal => Style::Quote,
                Style::Quote => Style::Normal,
                Style::Strong => Style::StrongQuote,
                Style::StrongQuote => Style::Strong,
            },
            _ => {
                push_next = false;
                self
            }
        };
        (ns, push_next)
    }
}
