use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::persona::Persona;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Card {
    /// Identifier for the spec; must be "chara_card_v2".
    pub spec: String,

    /// Specification version; for Character Card V2, this is "2.0".
    pub spec_version: String,

    /// Container for all character-specific fields and configurations.
    pub data: CharacterData,
}

impl Card {
    pub fn basic(name: &str, description: &str) -> Self {
        Self {
            spec: "chara_card_v2".to_string(),
            spec_version: "2.0".to_string(),
            data: CharacterData {
                name: name.to_string(),
                description: description.to_string(),
                personality: String::new(),
                scenario: String::new(),
                first_mes: String::new(),
                mes_example: String::new(),
                creator_notes: String::new(),
                system_prompt: String::new(),
                post_history_instructions: String::new(),
                alternate_greetings: vec![],
                tags: vec![],
                creator: String::new(),
                character_version: String::new(),
                extensions: HashMap::new(),
                character_book: None,
            },
        }
    }

    pub fn load_from_json(data: &str) -> Result<Self> {
        Ok(serde_json::from_str(data)?)
    }

    pub fn name(&self) -> &str {
        &self.data.name
    }

    pub fn greetings(&self, partner_name: Option<&str>) -> Vec<String> {
        let mut greetings = vec![self.data.first_mes.clone()];
        greetings.append(&mut self.data.alternate_greetings.clone());
        greetings
            .iter()
            .map(|g| Persona::replace_names(g, &self.data.name, partner_name))
            .collect()
    }

    pub fn system_prompt(&self, partner_name: Option<&str>) -> String {
        let data = self.data.clone();
        Persona::replace_names(
            &[
                data.system_prompt,
                data.description,
                data.scenario,
                data.mes_example,
            ]
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join("/n"),
            &self.data.name,
            partner_name,
        )
    }
}

/// Contains core character properties along with new V2 fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterData {
    /// The character's display name.
    pub name: String,

    /// Detailed description of the character to be included in every prompt.
    pub description: String,

    /// A summary of the character's personality traits.
    pub personality: String,

    /// The scenario or current context in which the character exists.
    pub scenario: String,

    /// The character's first message (greeting) used at the start of a conversation.
    pub first_mes: String,

    /// Example dialogues demonstrating the character's behavior.
    pub mes_example: String,

    /// Out-of-character notes for the creator's reference.
    pub creator_notes: String,

    /// Custom system prompt that overrides the default system prompt.
    pub system_prompt: String,

    /// Post-history instructions inserted after the conversation history.
    pub post_history_instructions: String,

    /// Array of alternative greeting messages for additional variety.
    pub alternate_greetings: Vec<String>,

    /// Array of tags for categorization and filtering purposes.
    pub tags: Vec<String>,

    /// Identifier for the creator of the character card.
    pub creator: String,

    /// Version of the character card.
    pub character_version: String,

    /// Custom extension data for additional metadata at the character level.
    pub extensions: Extensions,

    /// Optional character-specific lorebook containing background lore and dynamic entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_book: Option<CharacterBook>,
}

pub type Extensions = HashMap<String, serde_json::Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// List of primary trigger keywords that activate this lore entry.
    pub keys: Vec<String>,

    /// The lore text to be injected into the prompt when this entry is triggered.
    pub content: String,

    /// Custom extension data specific to this lore entry.
    pub extensions: Extensions,

    /// Flag indicating whether this entry is active.
    pub enabled: bool,

    /// Numeric value controlling the order in which triggered entries are injected into the prompt.
    pub insertion_order: i32,

    /// Optional flag for enabling case-sensitive matching of keys.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,

    /// Optional internal name for the lore entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional priority value used for dropping the entry if the combined lore exceeds the token budget.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,

    /// Optional internal identifier for this entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Optional comment or note about the entry for human reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    /// If true, this entry will trigger only if both a primary keyword and a secondary keyword are present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selective: Option<bool>,

    /// Optional secondary trigger keywords used in conjunction with keys when selective is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_keys: Option<Vec<String>>,

    /// If true, this entry is always injected into the prompt regardless of trigger keywords.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constant: Option<bool>,

    /// Specifies the insertion position of this entry's content relative to the character's main definition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
}

/// Represents a character-specific lorebook attached to a character card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterBook {
    /// Optional title of the lorebook.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional description summarizing the lorebook's content and purpose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The number of recent chat messages to scan for triggering lore entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_depth: Option<i32>,

    /// The maximum token budget allocated for lore entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<i32>,

    /// Flag indicating whether recursive scanning is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive_scanning: Option<bool>,

    /// Custom extension data for the lorebook.
    pub extensions: Extensions,

    /// Array of lore entries that comprise the lorebook.
    pub entries: Vec<Entry>,
}
