use serde::Deserialize;
use std::io::Write;
use crate::locale::Currency;
use crate::generate_tex::*;
use crate::helpers::{ from_toml_file, DateTime, FromTomlFile };
use crate::worklog::{ Worklog, WorklogRecord };

use std::collections::{HashMap, BTreeMap};

use struct_iterable::Iterable;

#[derive(Debug, Deserialize, Iterable)]
pub struct Contact {
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
pub struct Payment {
    accountholder: Option<String>,
    iban: String,
    bic: String,
    taxid: String,
    currency: Option<Currency>,
    taxrate: f32
}

impl Payment {
    pub fn currency(&self) -> Currency {
        match &self.currency {
            Some(currency) => currency.clone(),
            None => "EUR".to_string().into()
        }
    }

    pub fn currency_symbol(&self) -> String {
        self.currency().symbol()
    }
}

impl GenerateTexCommands for Payment {}


#[derive(Debug, Deserialize, Iterable)]
pub struct Invoicee {
    name: String,
    language: Option<String>,
    contact: Contact,
}

impl FromTomlFile for Invoicee {}


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
pub struct Config {
    contact: Contact,
    payment: Payment,
    invoice: InvoiceConfig
}

impl Config {
    pub fn from_toml_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        from_toml_file::<Self>(filename)
    }
}

use std::ops::Add;



pub struct Invoice {
    date: DateTime,
    config: Config,
    invoicee: Invoicee,
    positions: Vec<InvoicePosition>,
    begin_date: DateTime,
    end_date: DateTime,
}

impl Invoice {
    pub fn new(date: DateTime, config: Config, invoicee: Invoicee) -> Self {
        Invoice {
            date: date,
            config: config,
            invoicee: invoicee,
            positions: Vec::new(),
            begin_date: DateTime::MAX,
            end_date: DateTime::MIN,
        }
    }
    
    pub fn add_position(&mut self, position: InvoicePosition) {
        self.positions.push(position);
    }

    pub fn add_worklog(&mut self, worklog: &Worklog) {
        let mut positions: BTreeMap<String, InvoicePosition> = BTreeMap::new();

        for record in worklog.records() {
            self.begin_date = record.begin_date().min(self.begin_date);
            self.end_date = record.end_date().max(self.end_date);

            let text = record.message.clone();
            if positions.contains_key(&text) {
                positions.insert(text, positions.get(&record.message).unwrap().clone() + InvoicePosition::from_worklog_record(&record));
            } else {
                positions.insert(text, InvoicePosition::from_worklog_record(&record));
            }
        }

        for (text, position) in positions {
            self.positions.push(position)
        }
    }

}

#[derive(Debug, Iterable)]
pub struct InvoiceDetails {
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


#[derive(Clone)]
pub struct InvoicePosition {
    text: String,
    amount: f32,
    rate: f32,
    unit: String 
}

impl Add for InvoicePosition {
    type Output = Self; 

    fn add(self, other: Self) -> Self {
        assert!(self.unit == other.unit && self.text == other.text);

        let sum = self.amount + other.amount; 
        InvoicePosition {
            text: self.text, 
            amount: sum,
            rate: (self.amount * self.rate + other.amount * other.rate) / sum,
            unit: self.unit
        }

    }
}


impl InvoicePosition {
    pub fn from_worklog_record(w: &WorklogRecord) -> Self {
        Self {
            text: w.message.clone(),
            amount: w.hours,
            rate: w.rate,
            unit: String::from("h")
        }
    }

    fn net(&self) -> f32 {
        self.amount * self.rate
    }
}

impl GenerateTex for InvoicePosition {
    fn generate_tex<'a>(&self, w: &'a mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "\\position{{{text}}}{{{amount}{unit}}}{{{rate}}}{{{net}}}", 
            text = self.text,
            amount = self.amount,
            unit = self.unit,
            rate = self.rate,
            net = self.net())
    }
}






pub struct InvoicePositions {
    currency: Currency,
    positions: BTreeMap<String, InvoicePosition>,
}

impl InvoicePositions {
    fn from_worklog(worklog: &Worklog, currency: Currency) -> Self {
        let mut positions = InvoicePositions {
            currency: currency, 
            positions: BTreeMap::new()
        };

        for record in worklog.records() {
            let text = record.message.clone();
            if positions.positions.contains_key(&text) {
                positions.positions.insert(text, positions.positions.get(&record.message).unwrap().clone() + InvoicePosition::from_worklog_record(&record));
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
        self.begin_date
    }

    fn end_date(&self) -> DateTime {
        self.end_date
    }

    pub fn sum(&self) -> f32 {
        let mut sum = 0.0_f32;
        for position in &self.positions {
            sum += position.net();
        }
        sum
    }

    pub fn sum_with_tax(&self) -> f32 {        
        self.sum() * (1.0 + self.tax_rate() / 100.0)
    }

    pub fn tax(&self) -> f32 {
        self.sum_with_tax() - self.sum() 
    }

    pub fn tax_rate(&self) -> f32 {
        self.config.payment.taxrate
    }

    pub fn currency(&self) -> Currency {
        self.config.payment.currency()
    }

    pub fn currency_symbol(&self) -> String {
        self.config.payment.currency_symbol()
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
            for position in &self.positions {
                position.generate_tex(w)?;
            }
            Ok(())
        }));

        handlers.insert("INVOICE_SUM", Box::new(|w: &mut dyn Write| -> std::io::Result<()> {
            if self.config.invoice.calculate_value_added_tax {
                writeln!(w, "\\invoicesum{{{sum}{currency}}}{{{tax_rate}}}{{{tax}}}{{{sum_with_tax}{currency}}}", 
                    currency = self.currency_symbol(),
                    sum = self.sum(), 
                    tax_rate = self.tax_rate(), 
                    tax = self.tax(), 
                    sum_with_tax = self.sum_with_tax()
                )?;
            } else {
                writeln!(w, "\\invoicesumnotax{{{sum}{currency}}}",
                    currency = self.currency_symbol(),
                    sum = self.sum(), 
                )?;
            }

            Ok(())
        }));
        handlers.insert("INVOICE_VALUE_TAX_NOTE", Box::new(|w: &mut dyn Write| -> std::io::Result<()> {
            if !self.config.invoice.calculate_value_added_tax {
                writeln!(w, "\\trinvoicevaluetaxnote")?;
            }
            Ok(())
        }));


        if let Ok(lines) = crate::helpers::read_lines(format!("templates/{}", self.config.invoice.template)) {
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

