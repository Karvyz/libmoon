use std::rc::Rc;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::persona::{CharData, Persona};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Basic {
    name: String,
    description: String,
}

impl CharData for Basic {
    fn name(&self) -> &str {
        &self.name
    }

    fn system_prompt(&self, partner_name: Option<&str>) -> String {
        Persona::replace_names(&self.description, &self.name, partner_name)
    }

    fn greetings(&self, _: Option<&str>) -> Vec<String> {
        vec![]
    }
}

impl Basic {
    pub fn new(name: &str, description: &str) -> Rc<Self> {
        Rc::new(Basic {
            name: name.to_string(),
            description: description.to_string(),
        })
    }

    pub fn load_from_json(data: &str) -> Result<Rc<Self>> {
        Ok(Rc::new(serde_json::from_str(data)?))
    }
}
