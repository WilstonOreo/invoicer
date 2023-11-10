
use struct_iterable::Iterable;
use std::{io::Write, collections::HashMap};

pub fn generate_tex_command<'a>(mut w: &'a mut dyn Write, commandname: &str, content: &dyn std::any::Any) -> std::io::Result<()> {   
    if let Some(string) = crate::helpers::any_to_str(content) {
        let commandname = commandname.replace("_", "");
        writeln!(&mut w, "\\newcommand{{\\{commandname}}}{{{string}}}")?;
    }
    Ok(())
}

pub trait GenerateTexCommands : Iterable {
    fn generate_tex_commands<'a>(&self, w: &'a mut dyn Write, prefix: &str) -> std::io::Result<()> {
        for (field_name, field_value) in self.iter() {
            generate_tex_command(w, format!("{prefix}{field_name}").as_str(), field_value)?;
        }
        
        Ok(())
    }
}



pub trait GenerateTex {
    fn generate_tex<'a>(&self, w: &'a mut dyn Write) -> std::io::Result<()>;

    fn inline_input<'a>(&self, filename: &str, w: &'a mut dyn Write) -> std::io::Result<()> {
        let filename = format!("templates/{}.tex", filename);
        match crate::helpers::read_lines(&filename) {
            Ok(lines) => 
                for line in lines {
                    writeln!(w, "{}", line.unwrap())?;
                }
            Err(err) => {
                eprintln!("Could not include {}: {}", filename, err);
            }
        }
        
        Ok(())
    }

    fn generate_tex_file<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<()> {
        let mut f = std::fs::File::create(path)?;
        self.generate_tex(&mut f)
    }

    fn template_dir(&self) -> PathBuf { PathBuf::from(".") }
}

pub struct TexTemplate<'a> {
    filename: String,
    tokens: std::collections::HashMap<String, Box<dyn Fn(&mut dyn Write) -> Result<(), std::io::Error> + 'a>>
}

impl<'a> TexTemplate<'a> {
    pub fn new(filename: String) -> Self {
        Self {
            filename: filename,
            tokens: HashMap::new()
        }
    }

    pub fn token(&mut self, name: &str, tag: impl Fn(&mut dyn Write) -> Result<(), std::io::Error> + 'a) -> &mut Self {
        self.tokens.insert(name.to_string(), Box::new(tag));
        self
    }

    pub fn generate(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if let Ok(lines) = crate::helpers::read_lines(&self.filename) {
            // Consumes the iterator, returns an (Optional) String
            for line in lines {
                if let Ok(line) = line {
                    if line.starts_with("\\input{") {
                        let filename = line.replace("\\input{", "").replace("}", "");
                        self.inline_input(&filename, w)?;
                        continue;
                    }
                    writeln!(w, "{}", line)?;                    

                    if let Some(line_template) =  Self::token_name_from_line(&line) {
                        if let Some(handler) = self.tokens.get(line_template.as_str()) {
                            handler(w)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }


    fn inline_input(&self, filename: &str, w: &'a mut dyn Write) -> std::io::Result<()> {
        let filename = format!("templates/{}.tex", filename);
        match crate::helpers::read_lines(&filename) {
            Ok(lines) => 
                for line in lines {
                    writeln!(w, "{}", line.unwrap())?;
                }
            Err(err) => {
                eprintln!("Could not include {}: {}", filename, err);
            }
        }
        
        Ok(())
    }

    fn token_name_from_line(line: &String) -> Option<String> {
        let l = line.clone().trim().to_string();
        if l.starts_with("%$") {
            Some(l.replace("%$", "").trim().to_string())
        } else {
            None
        }
    }
}
