use std::{io::Read, fs::File};

pub fn from_toml_file<T: serde::de::DeserializeOwned>(filename: &str)  -> Result<T, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(&filename)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    
    Ok(toml::from_str(&s)?)
}

pub fn any_to_str(any: &dyn std::any::Any) -> Option<String> {
    if let Some(opt_string) = any.downcast_ref::<Option<String>>() {
        if let Some(as_string) = opt_string {
            Some(as_string.clone())
        } else {
            None
        }
    } else if let Some(string) = any.downcast_ref::<String>() {
        Some(string.clone())
    } else if let Some(number) = any.downcast_ref::<u32>() {
        Some(number.to_string())
    } else {
        None
    }
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
pub fn read_lines<P>(filename: P) -> std::io::Result<std::io::Lines<std::io::BufReader<File>>>
where P: AsRef<std::path::Path>, {
    let file = File::open(filename)?;
    use std::io::BufRead;
    Ok(std::io::BufReader::new(file).lines())
}

pub type DateTime = chrono::NaiveDateTime;
