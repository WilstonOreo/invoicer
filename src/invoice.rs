use serde::{Deserialize, Deserializer};
use std::io::Write;
use crate::locale::{Currency, Locale};
use crate::generate_tex::*;
use crate::helpers::{ from_toml_file, DateTime, date_to_str, FromTomlFile };
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
    #[serde(deserialize_with = "locale_from_str")]
    locale: Option<Locale>,
    contact: Contact,
}

fn locale_from_str<'de, D>(deserializer: D) -> Result<Option<Locale>, D::Error>
where D: Deserializer<'de> {
    let buf = String::deserialize(deserializer)?;

    use std::str::FromStr;
    let s = Locale::from_str(&buf).unwrap();
    println!("{}", s.name());
    Ok(Some(s))
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
    counter: u32,
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
            counter: 0,
            invoicee: invoicee,
            positions: Vec::new(),
            begin_date: DateTime::MAX,
            end_date: DateTime::MIN,
        }
    }

    pub fn locale(&self) -> Locale {
        self.invoicee.locale.clone().unwrap_or_default()
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
            date: date_to_str(invoice.date),
            number: invoice.number(),
            periodbegin: date_to_str(invoice.begin_date()),
            periodend: date_to_str(invoice.end_date()),
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

    fn generate_tex<'a>(&self, w: &'a mut dyn Write, l: Locale) -> std::io::Result<()> {
        writeln!(w, "\\position{{{text}}}{{{amount}{unit}}}{{{rate}}}{{{net}}}", 
            text = self.text,
            amount = l.format_number(self.amount, 2),
            unit = self.unit,
            rate = format!("{rate}{currency}/{unit}", rate = self.rate, currency = l.currency().symbol(), unit = self.unit),
            net = l.format_amount(self.net()))
    }
}




impl Invoice {
    pub fn number(&self) -> String {
        format!("{}{:02}", self.date.format("%Y%m").to_string(), self.counter)
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
            self.locale().generate_tex(w)
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
                position.generate_tex(w, self.locale())?;
            }
            Ok(())
        }));

        handlers.insert("INVOICE_SUM", Box::new(|w: &mut dyn Write| -> std::io::Result<()> {
            let l = self.locale();
            
            if self.config.invoice.calculate_value_added_tax {
                writeln!(w, "\\invoicesum{{{sum}}}{{{tax_rate}}}{{{tax}}}{{{sum_with_tax}}}", 
                    sum = l.format_amount(self.sum()), 
                    tax_rate = self.tax_rate(), 
                    tax = l.format_amount(self.tax()), 
                    sum_with_tax = l.format_amount(self.sum_with_tax()) 
                )?;
            } else {
                writeln!(w, "\\invoicesumnotax{{{sum}}}",
                    sum = l.format_amount(self.sum()), 
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

