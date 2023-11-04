
use struct_iterable::Iterable;
use std::io::Write;

pub fn generate_tex_command<'a>(mut w: &'a mut dyn Write, commandname: &str, content: &dyn std::any::Any) -> std::io::Result<()> {   
    if let Some(string) = crate::helpers::any_to_str(content) {
        writeln!(&mut w, "\\newcommand{{\\{commandname}}}{{{string}}}")?;
    } else {
     //   writeln!(&mut w, "\\newcommand{{\\{commandname}}}{{ }}")?;
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
}

