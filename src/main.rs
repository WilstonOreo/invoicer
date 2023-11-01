
use std::{io::Read, fs::File, collections::{HashMap, BTreeMap}};

use lazy_static::lazy_static;
use serde::Deserialize;
use std::io::Write;
use common_macros::hash_map;

lazy_static! {
    static ref CURRENCIES: HashMap<&'static str, &'static str> = {
        hash_map! {
            "EUR" => "€",
            "USD" => "$",
        }
    };
}


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
     //   writeln!(&mut w, "\\newcommand{{\\{commandname}}}{{ }}")?;
    }
    Ok(())
}

trait GenerateTexCommands : Iterable {
    fn generate_tex_commands<'a>(&self, w: &'a mut dyn Write, prefix: &str) -> std::io::Result<()> {
        for (field_name, field_value) in self.iter() {
            generate_tex_command(w, format!("{prefix}{field_name}").as_str(), field_value)?;
        }
        
        Ok(())
    }
}

trait GenerateTex {
    fn generate_tex<'a>(&self, w: &'a mut dyn Write) -> std::io::Result<()>;

    fn inline_input<'a>(&self, filename: &str, w: &'a mut dyn Write) -> std::io::Result<()> {
        let filename = format!("templates/{}.tex", filename);
        match read_lines(&filename) {
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

impl GenerateTexCommands for Contact {}

#[derive(Debug, Deserialize, Iterable)]
struct Payment {
    accountholder: Option<String>,
    iban: String,
    bic: String,
    taxid: String,
    currency: String,
    taxrate: f32
}

impl GenerateTexCommands for Payment {}


#[derive(Debug, Deserialize, Iterable)]
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

impl GenerateTexCommands for Invoicee {
    fn generate_tex_commands<'a>(&self, w: &'a mut dyn Write, prefix: &str) -> std::io::Result<()> {
        generate_tex_command(w, format!("{prefix}name").as_str(), &self.name)?;
        self.contact.generate_tex_commands(w, prefix)?;
        Ok(())
    }
}


#[derive(Debug, Deserialize)]
struct InvoiceConfig {
    template: String,
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

use std::ops::Add;
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
        use std::io::BufReader;
        let file = File::open(&filename)?;
        let mut buf_reader = BufReader::new(file);
        Self::from_csv(buf_reader)
    }

    pub fn sum(&self) -> f32 {
        let mut sum = 0.0_f32;
        for record in &self.records {
            sum += record.net();
        }
        sum
    }

    pub fn sum_with_tax(&self, taxrate: f32) -> f32 {
        self.sum() * (1.0 + taxrate / 100.0)
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


#[derive(Clone, Copy)]
struct InvoicePosition {
    amount: f32,
    rate: f32
}

impl Add for InvoicePosition {
    type Output = Self; 

    fn add(self, other: Self) -> Self {
        let sum = self.amount + other.amount; 
        InvoicePosition { 
            amount: sum,
            rate: (self.amount * self.rate + other.amount * other.rate) / sum  
        }
    }
}


impl InvoicePosition {
    fn from_worklog_record(w: &WorklogRecord) -> Self {
        Self {
            amount: w.hours,
            rate: w.rate,
        }
    }

    fn net(&self) -> f32 {
        self.amount * self.rate
    }
}



struct InvoicePositions {
    positions: BTreeMap<String, InvoicePosition>,
}
impl GenerateTex for InvoicePositions {

    fn generate_tex<'a>(&self, w: &'a mut dyn Write) -> std::io::Result<()> {
        for (text, position) in &self.positions {
            writeln!(w, "\\position{{{}}}{{{}}}{{{}}}{{{}}}", text, position.amount, position.rate, position.net())?;
        }
        Ok(())
    }

}
impl InvoicePositions {
    fn from_worklog(worklog: &Worklog) -> Self {
        let mut positions = InvoicePositions {
            positions: BTreeMap::new()
        };

        for record in &worklog.records {
            let text = record.message.clone();
            if positions.positions.contains_key(&text) {
                positions.positions.insert(text, *positions.positions.get(&record.message).unwrap() + InvoicePosition::from_worklog_record(&record));
            } else {
                positions.positions.insert(text, InvoicePosition::from_worklog_record(&record));
            }
        }

        positions
    }
}


impl Invoice {

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


impl GenerateTex for Invoice {
    fn generate_tex<'a>(&self, w: &'a mut dyn Write) -> std::io::Result<()> {
        let mut handlers: HashMap<&str, Box<dyn Fn(&mut dyn Write) -> Result<(), std::io::Error>>> = HashMap::new();

        handlers.insert("LANGUAGE", Box::new(|w: &mut dyn Write| -> std::io::Result<()> {            
            let language = match &self.invoicee.language {
                Some(language) => language,
                None => "english"
            };

            self.inline_input(language, w)
        }));

        handlers.insert("INVOICEE_ADDRESS", Box::new(|w: &mut dyn Write| -> std::io::Result<()> {
            self.invoicee.generate_tex_commands(w, "invoicee")
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
            let positions = InvoicePositions::from_worklog(&self.worklog);
            positions.generate_tex(w)
        }));


        if let Ok(lines) = read_lines(format!("templates/{}", self.config.invoice.template)) {
            // Consumes the iterator, returns an (Optional) String
            for line in lines {
                if let Ok(line) = line {
                    if line.starts_with("\\input{") {
                        let filename = line.replace("\\input{", "").replace("}", "");
                        self.inline_input(&filename, w)?;
                        continue;
                    }
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

    invoice.generate_tex(&mut f);

//    config.contact.generate_tex(&mut f, "my").unwrap();
}
