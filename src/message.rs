use llm::chat::ChatMessage;

#[derive(Debug, Clone)]
pub enum OwnerType {
    User,
    Char(usize),
}

#[derive(Debug, Clone)]
pub struct Message {
    pub owner: OwnerType,
    pub text: String,
    pub editing: Option<String>,
}

impl Message {
    pub fn from_user(text: String) -> Self {
        Message {
            owner: OwnerType::User,
            text,
            editing: None,
        }
    }

    pub fn from_char(char_id: usize, text: String) -> Self {
        Message {
            owner: OwnerType::Char(char_id),
            text: text.trim().to_string(),
            editing: None,
        }
    }

    pub fn empty_from_char(char_id: usize) -> Self {
        Self::from_char(char_id, String::new())
    }

    pub fn to_chat_message(&self) -> ChatMessage {
        match self.owner {
            OwnerType::User => ChatMessage::user().content(&self.text).build(),
            OwnerType::Char(_) => ChatMessage::assistant().content(&self.text).build(),
        }
    }

    pub fn create_brother(&self) -> Self {
        Message {
            owner: self.owner.clone(),
            text: String::new(),
            editing: None,
        }
    }
}
