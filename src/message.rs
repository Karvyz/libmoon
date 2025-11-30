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
    id: usize,
}

impl Message {
    pub fn from_user(text: String, id: usize) -> Self {
        Message {
            owner: OwnerType::User,
            text,
            id,
        }
    }

    pub fn from_char(char_id: usize, text: String, id: usize) -> Self {
        Message {
            owner: OwnerType::Char(char_id),
            text: text.trim().to_string(),
            id,
        }
    }

    pub fn empty_from_char(char_id: usize, id: usize) -> Self {
        Self::from_char(char_id, String::new(), id)
    }

    pub fn to_chat_message(&self) -> ChatMessage {
        match self.owner {
            OwnerType::User => ChatMessage::user().content(&self.text).build(),
            OwnerType::Char(_) => ChatMessage::assistant().content(&self.text).build(),
        }
    }

    pub fn create_brother(&self, id: usize) -> Self {
        Message {
            owner: self.owner.clone(),
            text: String::new(),
            id,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }
}
