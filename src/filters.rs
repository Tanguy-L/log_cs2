use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Rules {
    OneLine,
    Verbose,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Status {
    Infos,
    Warning,
    Error,
    Custom,
    Custom2,
    Custom3,
}

mod regex_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(regex: &Regex, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(regex.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Regex, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Regex::new(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListFilter {
    pub filters: Vec<Filter>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Filter {
    pub name: String,
    status: Status,
    pub key_code: String,
    #[serde(with = "regex_serde")]
    regex: Regex,
    rule: Rules,
    pub is_on: bool,
}

impl Filter {
    pub fn toggle(&mut self) {
        self.is_on = !self.is_on;
    }

    pub fn match_regex(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }

    pub fn get_status(&self) -> Status {
        return self.status.clone();
    }
}
