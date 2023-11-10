
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

impl Default for Currency {
    fn default() -> Self {
        Self("EUR".to_string())
    }
}


#[derive(Debug, Clone, Deserialize, Iterable)]

pub struct Locale {
    #[serde(skip)] 
    name: String,
    decimal: String,
    separator: String,
    pattern: String,
    currency: Currency,
    translations: HashMap<String, String>
}

impl Default for Locale {
    fn default() -> Self {
        Self {
            name: "en".to_string(),
            decimal: ".".to_string(),
            separator: ",".to_string(),
            pattern: "#!".to_string(),
            currency: Currency::default(),
            translations: HashMap::new()
        }
    }
}

impl Locale {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    pub fn tr(&self, s: String) -> &String {
        self.translations.get(&s).unwrap()
    } 

    pub fn format_number<T: std::fmt::Display>(&self, number: T, precision: usize) -> String {
        let s = format!("{number:.precision$}")
            .replace(".", &self.decimal);
        
        let mut fs = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i % 3 == 0 && (i > 2 + self.decimal.len()) {
                fs = self.separator.clone() + &fs;
            }
            fs = c.to_string() + &fs;
        }
        fs
    }

    pub fn format_amount<T: std::fmt::Display>(&self, number: T) -> String {
        self.pattern
            .replace('#', self.format_number(number, 2).as_str())
            .replace('!', self.currency.symbol().as_str())
    }
}


use crate::{generate_tex::{GenerateTex, generate_tex_command}, helpers::{FromTomlFile, self, FilePath}};

impl GenerateTex for Locale {
    fn generate_tex<'a>(&self, w: &'a mut dyn std::io::Write) -> std::io::Result<()> {
        for (name, translation) in &self.translations {
            generate_tex_command(w, format!("tr{}", name).as_str(), translation)?;
        }
        Ok(())
    }
}

impl FromTomlFile for Locale {
    fn from_toml_file<P: FilePath>(filename: P)  -> Result<Self, Box<dyn std::error::Error>> {
        let name = filename.to_string();
        let mut locale: Locale = helpers::from_toml_file(filename)?;
        locale.name = helpers::name_from_file::<std::path::PathBuf>(name.into());
        
        Ok(locale)
    }
}


#[cfg(test)]
mod tests {
    use crate::{helpers::FromTomlFile, generate_tex::GenerateTex};
    use super::Locale;

    #[test]
    fn load_toml_and_generate_tex() {
        let locale = Locale::from_toml_file(std::path::Path::new("locales/en.toml"));
        assert!(locale.is_ok());
        
        let locale = locale.unwrap();
        assert_eq!(locale.name, "en");

        assert!(!locale.translations.is_empty());
        assert!(locale.generate_tex(&mut std::io::sink()).is_ok());
    }

    #[test]
    fn format() {
        let locale = Locale::from_toml_file(std::path::Path::new("locales/en.toml")).unwrap();

        assert_eq!(locale.format_amount(1234.943_f32), "1,234.94€");
        assert_eq!(locale.format_amount(1234.00_f32), "1,234.00€");
        assert_eq!(locale.format_amount(1234_i32), "1234€"); // TODO: Handle int types differently?
    }
}