use chrono::Datelike;
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
    companyname: Option<String>,
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
    tax_rate: f32,
    default_rate: Option<f32>
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
    #[serde(skip)]
    name: String,
    contact: Contact,
    invoice: InvoiceConfig,
    default_rate: Option<f32>
}



impl FromTomlFile for Invoicee {
    fn from_toml_file(filename: &str)  -> Result<Self, Box<dyn std::error::Error>> {
        let mut invoicee: Invoicee = crate::helpers::from_toml_file(filename)?;
        invoicee.name = crate::helpers::name_from_file(&filename);

        Ok(invoicee)
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
    #[serde(deserialize_with = "locale_from_str", default)]
    locale: Option<Locale>,
//    #[serde(default = "default_number_format")]
    template: Option<String>,
  //  #[serde(default = "default_number_format")]
    number_format: Option<String>,
//    #[serde(default = "default_filename_format")]
    filename_format: Option<String>,
    days_for_payment: Option<u32>,
    calculate_value_added_tax: Option<bool>,
    timesheet: Option<bool>,
}

fn locale_from_str<'de, D>(deserializer: D) -> Result<Option<Locale>, D::Error>
where D: Deserializer<'de> {
    let buf = String::deserialize(deserializer);
    if buf.is_err() {
        return Ok(None);
    }
    let buf = buf.unwrap();

    use std::str::FromStr;
    let s = Locale::from_str(&buf).unwrap_or_default();
    Ok(Some(s))
}

macro_rules! default_getter {
    ($i:ident, $t:ty, $l:literal) => {
        pub fn $i(&self) -> $t { self.$i.clone().unwrap_or(Into::<$t>::into($l)) }        
    };
    ($i:ident, $t:ty) => {
        pub fn $i(&self) -> $t { self.$i.clone().unwrap_or_default() }        
    };
}

impl InvoiceConfig {
    default_getter!(locale, Locale);
    default_getter!(template, String, "invoice.tex");
    default_getter!(number_format, String, "%Y%m${COUNTER}");
    default_getter!(filename_format, String, "${INVOICENUMBER}_${INVOICE}_${INVOICEE}.tex");
    default_getter!(days_for_payment, u32, 14_u32);
    default_getter!(calculate_value_added_tax, bool, true);
    default_getter!(timesheet, bool, true);
}



#[derive(Debug, Deserialize)]
pub struct Config {
    contact: Contact,
    payment: Payment,
    invoice: InvoiceConfig,
}

impl Config {
    pub fn from_toml_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        from_toml_file::<Self>(filename)
    }
}

use std::ops::Add;


pub type Timesheet = Worklog;

pub struct Invoice {
    date: DateTime,
    config: InvoiceConfig,
    invoicer: Contact,
    payment: Payment,
    counter: u32,
    invoicee: Invoicee,
    positions: Vec<InvoicePosition>,
    timesheet: Option<Timesheet>,
    begin_date: DateTime,
    end_date: DateTime,
}

impl Invoice {
    pub fn new(date: DateTime, config: Config, invoicee: Invoicee) -> Self {
        Invoice {
            date: date,
            config: config.invoice,
            invoicer: config.contact,
            payment: config.payment,
            counter: 0,
            invoicee: invoicee,
            positions: Vec::new(),
            timesheet: None,
            begin_date: DateTime::MAX,
            end_date: DateTime::MIN,
        }
    }

    pub fn locale(&self) -> Locale {
        match &self.invoicee.invoice.locale {
            Some(locale) => locale.clone(),
            None => match &self.config.locale {
                Some(locale) => locale.clone(),
                None => Locale::default()
            }
        }
    }
    
    pub fn add_position(&mut self, position: InvoicePosition) {
        self.positions.push(position);
    }

    pub fn positions(&self) -> &Vec<InvoicePosition> {
        &self.positions
    }

    pub fn default_rate(&self) -> f32 {
        self.invoicee.default_rate
            .unwrap_or(self.payment.default_rate.unwrap_or(100.0))
    }

    pub fn add_worklog(&mut self, worklog: &Worklog) {
        let mut positions: BTreeMap<String, InvoicePosition> = BTreeMap::new();

        for record in worklog.records() {
            self.begin_date = record.begin_date().min(self.begin_date);
            self.end_date = record.end_date().max(self.end_date);

            if self.config.timesheet() {
                if self.timesheet.is_none() {
                    self.timesheet = Some(Timesheet::new());
                }
                self.timesheet.as_mut().unwrap().add_record(record.clone());
            }

            let text = record.message.clone();
            let position = InvoicePosition::from_worklog_record(&record, worklog.rate());
            if positions.contains_key(&text) {
                positions.insert(text, positions.get(&record.message).unwrap().clone() + position);
            } else {
                positions.insert(text, position);
            }
        }

        for (_, position) in positions {
            self.positions.push(position)
        }

        // Sort timesheet each time a worklog was added
        if self.config.timesheet() {
            self.timesheet.as_mut().unwrap().sort();
        }
    }

    pub fn number(&self) -> String {
        let date = self.date.date();
        self.config.number_format()
            .replace("%Y", format!("{:04}", date.year()).as_str())
            .replace("%m", format!("{:02}", date.month()).as_str())
            .replace("${COUNTER}", format!("{:02}", self.counter).as_str())
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
        self.payment.tax_rate
    }

    pub fn currency(&self) -> Currency {
        self.payment.currency()
    }

    pub fn currency_symbol(&self) -> String {
        self.payment.currency_symbol()
    }

    pub fn filename(&self) -> String {
        self.config.filename_format()
            .replace("${INVOICENUMBER}", self.number().as_str())
            .replace("${INVOICE}", &self.locale().tr("invoice".to_string()))
            .replace("${INVOICEE}", &self.invoicee.name)
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
    price_per_item: f32,
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
            price_per_item: (self.amount * self.price_per_item + other.amount * other.price_per_item) / sum,
            unit: self.unit
        }

    }
}


impl InvoicePosition {
    pub fn from_worklog_record(w: &WorklogRecord, default_rate: f32) -> Self {
        Self {
            text: w.message.clone(),
            amount: w.hours,
            price_per_item: w.rate.unwrap_or(default_rate),
            unit: String::from("h")
        }
    }

    fn net(&self) -> f32 {
        self.amount * self.price_per_item
    }

    fn generate_tex<'a>(&self, w: &'a mut dyn Write, l: &Locale) -> std::io::Result<()> {
        writeln!(w, "\\position{{{text}}}{{{amount}{unit}}}{{{rate}}}{{{net}}}", 
            text = self.text,
            amount = l.format_number(self.amount, 2),
            unit = self.unit,
            rate = format!("{p}{currency}/{unit}", p = self.price_per_item, currency = l.currency().symbol(), unit = self.unit),
            net = l.format_amount(self.net()))
    }
}



impl GenerateTex for Invoice {
    fn generate_tex<'a>(&self, w: &'a mut dyn Write) -> std::io::Result<()> {
        let mut template = TexTemplate::new(format!("templates/{}", &self.config.template())); 
        
        template
            .add_tag("LANGUAGE",  |w: &mut dyn Write| -> std::io::Result<()> {
                self.locale().generate_tex(w)
            })
            .add_tag( "INVOICEE_ADDRESS", |w: &mut dyn Write| -> std::io::Result<()> {            
                self.invoicee.generate_tex_commands(w, "invoicee")
            })
            .add_tag( "BILLER_ADDRESS", |w: &mut dyn Write| -> std::io::Result<()> {            
                self.invoicer.generate_tex_commands(w, "my")
            })
            .add_tag("PAYMENT_DETAILS", |w: &mut dyn Write| -> std::io::Result<()> {
                self.payment.generate_tex_commands(w, "my")
            })
            .add_tag("INVOICE_DETAILS", |w: &mut dyn Write| -> std::io::Result<()> {
                let details = InvoiceDetails::from_invoice(&self);
                details.generate_tex_commands(w, "invoice")
            })
            .add_tag("INVOICE_POSITIONS", |w: &mut dyn Write| -> std::io::Result<()> {
                for position in &self.positions {
                    position.generate_tex(w, &self.locale())?;
                }
                Ok(())
            })
            .add_tag("INVOICE_SUM", |w: &mut dyn Write| -> std::io::Result<()> {
                let l = self.locale();
                
                if self.config.calculate_value_added_tax() {
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
            })
            .add_tag("INVOICE_VALUE_TAX_NOTE", |w: &mut dyn Write| -> std::io::Result<()> {
                if !self.config.calculate_value_added_tax() {
                    writeln!(w, "\\trinvoicevaluetaxnote")?;
                }
                Ok(())
            })
            .generate(w)
    }
}

