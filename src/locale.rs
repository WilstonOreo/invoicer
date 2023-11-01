
use common_macros::hash_map;
use lazy_static::lazy_static;

use std::collections::HashMap;
use serde::Deserialize;

lazy_static! {
    static ref CURRENCIES: HashMap<&'static str, &'static str> = {
        hash_map! {
            "EUR" => "€",
            "USD" => "$",
        }
    };
}


#[derive(Clone, Deserialize)]
pub struct Currency(String);


impl Currency {
    pub fn from_str(s: String) -> Currency {
        Self(s)
    }

    pub fn str(&self) -> &String {
        &self.0
    }
    
    pub fn symbol(&self) -> String {
        CURRENCIES.get(self.0.as_str()).unwrap_or(&"€").to_string()
    }
}

impl From<String> for Currency {
    fn from(value: String) -> Self {
        Self::from_str(value)
    }
}

impl Into<String> for Currency {
    fn into(self) -> String {
        self.str().clone()
    }
}

impl std::fmt::Debug for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

