
use common_macros::hash_map;
use lazy_static::lazy_static;

use std::collections::HashMap;
use serde::Deserialize;
use struct_iterable::Iterable;

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

#[derive(Debug, Deserialize, Iterable)]

pub struct Locale {
    currency: Currency,
    decimalseparator: String,
    thousandseparator: String,
    translations: HashMap<String, String>
}

use crate::{generate_tex::{GenerateTex, generate_tex_command}, helpers::FromTomlFile};

impl GenerateTex for Locale {
    fn generate_tex<'a>(&self, w: &'a mut dyn std::io::Write) -> std::io::Result<()> {
        for (name, translation) in &self.translations {
            generate_tex_command(w, format!("tr{}", name).as_str(), translation)?;
        }
        Ok(())
    }
}

impl FromTomlFile for Locale {}

#[cfg(test)]
mod tests {
    use crate::{helpers::FromTomlFile, generate_tex::GenerateTex};
    use super::Locale;

    #[test]
    fn load_toml_and_generate_tex() {
        let locale = Locale::from_toml_file("locales/en.toml");
        assert!(locale.is_ok());

        let locale = locale.unwrap();

        assert!(!locale.translations.is_empty());
        locale.generate_tex(&mut std::io::stdout()).unwrap();
    }

}