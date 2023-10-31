
use std::{io::Read, fs::File, collections::HashMap};

use serde::Deserialize;
use std::io::Write;


fn from_toml_file<T: serde::de::DeserializeOwned>(filename: &str)  -> Result<T, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(&filename)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    
    Ok(toml::from_str(&s)?)
}

fn any_to_str(any: &dyn std::any::Any) -> Option<String> {
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

fn generate_tex_command<'a>(mut w: &'a mut dyn Write, commandname: &str, content: &dyn std::any::Any) -> std::io::Result<()> {   
    if let Some(string) = any_to_str(content) {
        writeln!(&mut w, "\\newcommand{{\\{commandname}}}{{{string}}}")?;
    } else {
        writeln!(&mut w, "\\newcommand{{\\{commandname}}}{{}}")?;
    }
    Ok(())
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> std::io::Result<std::io::Lines<std::io::BufReader<File>>>
where P: AsRef<std::path::Path>, {
    let file = File::open(filename)?;
    use std::io::BufRead;
    Ok(std::io::BufReader::new(file).lines())
}



use struct_iterable::Iterable;

#[derive(Debug, Deserialize, Iterable)]
struct Contact {
    fullname: String,
    street: String,
    zipcode: u32,
    city: String,
    country: String,
    phone: Option<String>,
    fax: Option<String>,
    email: String,
    website: Option<String>,
}

trait GenerateTexCommands : Iterable {
    fn generate_tex_commands<'a>(&self, w: &'a mut dyn Write, prefix: &str) -> std::io::Result<()> {
        for (field_name, field_value) in self.iter() {
            generate_tex_command(w, format!("{prefix}{field_name}").as_str(), field_value)?;
        }
        
        Ok(())
    }
}

impl GenerateTexCommands for Contact {}

#[derive(Debug, Deserialize, Iterable)]
struct Payment {
    account_holder: Option<String>,
    iban: String,
    bic: String,
    taxid: String,
}

impl GenerateTexCommands for Payment {}


#[derive(Debug, Deserialize)]
struct Invoicee {
    name: String,
    language: Option<String>,
    contact: Contact,
}


impl Invoicee {
    fn from_toml_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        from_toml_file::<Self>(filename)
    }
}



#[derive(Debug, Deserialize)]
struct InvoiceConfig {
    invoice_template: String,
    worklog_template: String,
    filename_format: String,
    days_for_payment: Option<u32>,
    calculate_value_added_tax: bool    
}

#[derive(Debug, Deserialize)]
struct Config {
    contact: Contact,
    payment: Payment,
    invoice: InvoiceConfig
}

impl Config {
    fn from_toml_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        from_toml_file::<Self>(filename)
    }
}


type DateTime = chrono::NaiveDateTime;

#[derive(Debug, Deserialize)]
struct WorklogRecord {
    start: String,
    hours: f32,
    rate: f32,
    message: String
}

impl WorklogRecord {
    fn begin_date(&self) -> DateTime {
        DateTime::parse_from_str(&self.start, "%m/%d/%Y %H:%M").unwrap()
    }

    fn end_date(&self) -> DateTime {
        let mut date = self.begin_date();
        date += chrono::Duration::seconds((60.0 * 60.0 * self.hours) as i64);
        date
    }

    fn net(&self) -> f32 {
        self.hours * self.rate
    }
}

struct Worklog {
    begin_date: DateTime,
    end_date: DateTime,
    records: Vec<WorklogRecord>
}

impl Worklog {

    pub fn new() -> Self {
        Self {
            begin_date: DateTime::MAX,
            end_date: DateTime::MIN,
            records: Vec::new()
        }
    }

    pub fn from_csv(reader: impl std::io::Read) -> Result<Self, Box<dyn std::error::Error>> {
        let mut rdr = csv::Reader::from_reader(reader);
        let mut worklog = Self::new();
        
        for result in rdr.deserialize() {
            // Notice that we need to provide a type hint for automatic
            // deserialization.
            let record: WorklogRecord = result?;
            worklog.begin_date = record.begin_date().min(worklog.begin_date);
            worklog.end_date = record.end_date().max(worklog.end_date);
            worklog.records.push(record);
        }

        Ok(worklog)
    }

    pub fn from_csv_file(filename: &str)  -> Result<Self, Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io::BufReader;
        let file = File::open(&filename)?;
        let mut buf_reader = BufReader::new(file);
        Self::from_csv(buf_reader)
    }
}


struct Invoice {
    date: DateTime,
    worklog: Worklog,
    config: Config,
    invoicee: Invoicee,
}

#[derive(Debug, Iterable)]
struct InvoiceDetails {
    date: String,
    number: String,
    periodbegin: String,
    periodend: String,
}

impl InvoiceDetails {
    pub fn from_invoice(invoice: &Invoice) -> Self {
        Self {
            date: invoice.date.to_string(),
            number: invoice.date.to_string(),
            periodbegin: invoice.begin_date().to_string(),
            periodend: invoice.end_date().to_string()
        }
    }
}

impl GenerateTexCommands for InvoiceDetails {}



impl Invoice {

    fn generate_invoice_tex<'a>(&self, w: &'a mut dyn Write) -> std::io::Result<()> {
        let mut handlers: HashMap<&str, Box<dyn Fn(&mut dyn Write) -> Result<(), std::io::Error>>> = HashMap::new();

        handlers.insert("LANGUAGE", Box::new(|w: &mut dyn Write| -> std::io::Result<()> {
            writeln!(w, "\\input{{{}}}", match &self.invoicee.language {
                Some(language) => language.clone(),
                None => "english".to_string()
            })?;
            Ok(())
        }));

        handlers.insert("INVOICEE_ADDRESS", Box::new(|w: &mut dyn Write| -> std::io::Result<()> {
            self.invoicee.contact.generate_tex_commands(w, "invoicee")
        }));

        handlers.insert("BILLER_ADDRESS", Box::new(|w: &mut dyn Write| -> std::io::Result<()> {
            self.config.contact.generate_tex_commands(w, "my")
        }));

        handlers.insert("PAYMENT_DETAILS", Box::new(|w: &mut dyn Write| -> std::io::Result<()> {
            self.config.payment.generate_tex_commands(w, "my")
        }));

        handlers.insert("INVOICE_DETAILS", Box::new(|w: &mut dyn Write| -> std::io::Result<()> {
            let details = InvoiceDetails::from_invoice(&self);
            details.generate_tex_commands(w, "invoice")
        }));

        handlers.insert("INVOICE_POSITIONS", Box::new(|w: &mut dyn Write| -> std::io::Result<()> {
            for record in &self.worklog.records {
                writeln!(w, "\\position{{{}}}{{{}}}{{{}}}{{{}}}", record.message, record.hours, record.rate, record.net())?;
            }
            Ok(())
        }));


        if let Ok(lines) = read_lines(format!("templates/{}", self.config.invoice.invoice_template)) {
            // Consumes the iterator, returns an (Optional) String
            for line in lines {
                if let Ok(line) = line {
                    writeln!(w, "{}", line)?;                    

                    if let Some(line_template) =  Self::line_template_name(&line) {
                        if let Some(handler) = handlers.get(line_template.as_str()) {
                            handler(w)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn generate_worklog_tex(&self, filename: &str) {

    }

    fn line_template_name(line: &String) -> Option<String> {
        let l = line.clone().trim().to_string();
        if l.starts_with("%$") {
            Some(l.replace("%$", "").trim().to_string())
        } else {
            None
        }
    }
    
    fn begin_date(&self) -> DateTime {
        self.worklog.begin_date
    }

    fn end_date(&self) -> DateTime {
        self.worklog.end_date
    }
}



use clap::Parser;


#[derive(Parser, Debug)]
#[command(author="Michael Winkelmann", version, about="Invoicer")]
struct Arguments{
    #[arg(long, default_value_t = String::new())]
    worklog_csv: String,
    #[arg(long, default_value_t = String::new())]
    invoicee_toml: String,
    #[arg(short, long, default_value_t = String::from("invoicer.toml"))]
    config: String,
}



fn main() {
    let worklog = Worklog::from_csv_file("examples/ExampleWorklog.csv").unwrap();
    let config = Config::from_toml_file("invoicer.toml").unwrap();
    let invoicee = Invoicee::from_toml_file("examples/ExampleInvoicee.toml").unwrap();
    println!("Performance period: {:?} - {:?}", worklog.begin_date, worklog.end_date);

    let mut f = File::create("test.tex").unwrap();

    let invoice = Invoice {
        date: chrono::offset::Local::now().naive_local(),
        worklog: worklog,
        config: config,
        invoicee: invoicee
    };

    invoice.generate_invoice_tex(&mut f);

//    config.contact.generate_tex(&mut f, "my").unwrap();
}
