use std::{io::Read, fs::File, path::{Path, PathBuf, self}};

pub trait FilePath: AsRef<std::path::Path> + AsRef<std::ffi::OsStr> {

    fn to_string(&self) -> String {
        Path::new(&self).as_os_str().to_str().unwrap().into()
    }

    fn file_name(&self) -> String {
        String::from(Path::new(&self).file_name().unwrap().to_str().unwrap())
    }

    fn parent(&self) -> String {
        String::from(Path::new(&self).parent().unwrap().to_str().unwrap())
    }

}

impl FilePath for Path {
}

impl FilePath for PathBuf {
}

impl FilePath for &Path {

}


pub fn from_toml_file<T: serde::de::DeserializeOwned, P: FilePath>(p: P)  -> Result<T, Box<dyn std::error::Error>> {
    let path_str = p.to_string();
    let mut file = std::fs::File::open(p)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    
    match toml::from_str(&s) {
        Ok(result) => Ok(result),
        Err(err) => {
            eprintln!("Error reading {}: {err}", path_str);
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{err}"))))
        }
    }
}

pub fn name_from_file<P: FilePath>(p: P) -> String {
    std::path::Path::new(&p).file_stem().unwrap().to_str().unwrap().to_string()
}

pub fn home_dir() -> String {
    home::home_dir().unwrap_or(".".into()).into_os_string().into_string().unwrap()
}

pub trait FromTomlFile: serde::de::DeserializeOwned {
    fn from_toml_file<P: FilePath>(p: P)  -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = std::fs::File::open(p)?;
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
pub fn read_lines<P: AsRef<std::path::Path>>(filename: P) -> std::io::Result<std::io::Lines<std::io::BufReader<File>>> {
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
