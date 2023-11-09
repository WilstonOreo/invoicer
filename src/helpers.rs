use std::{io::Read, fs::File};

pub fn from_toml_file<T: serde::de::DeserializeOwned>(filename: &str)  -> Result<T, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(&filename)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    
    match toml::from_str(&s) {
        Ok(result) => Ok(result),
        Err(err) => {
            eprintln!("Error reading {filename}: {err}");
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{err}"))))
        }
    }
}

pub fn name_from_file(filename: &str) -> String {
    std::path::Path::new(&filename).file_stem().unwrap().to_str().unwrap().to_string()
}

pub trait FromTomlFile: serde::de::DeserializeOwned {
    fn from_toml_file(filename: &str)  -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = std::fs::File::open(&filename)?;
        let mut s = String::new();
        file.read_to_string(&mut s)?;
        
        Ok(toml::from_str(&s)?)    
    } 
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

pub fn date_to_str(d: DateTime, format_str: &String) -> String {
    d.format(format_str.as_str()).to_string()
}

pub fn now() -> DateTime {
    chrono::offset::Local::now().naive_local()
}
