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
    country: Option<String>,
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
pub struct Recipient {
    #[serde(skip)]
    name: String,
    contact: Contact,
    invoice: InvoiceConfig,
    default_rate: Option<f32>
}



impl FromTomlFile for Recipient {
    fn from_toml_file(filename: &str)  -> Result<Self, Box<dyn std::error::Error>> {
        let mut recipient: Recipient = crate::helpers::from_toml_file(filename)?;
        recipient.name = crate::helpers::name_from_file(&filename);

        Ok(recipient)
    }
}


impl GenerateTexCommands for Recipient {
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
    template: Option<String>,
    number_format: Option<String>,
    filename_format: Option<String>,
    days_for_payment: Option<u32>,
    calculate_value_added_tax: Option<bool>,
    timesheet_template: Option<String>,
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
    default_getter!(filename_format, String, "${INVOICENUMBER}_${INVOICE}_${RECIPIENT}.tex");
    default_getter!(days_for_payment, u32, 14_u32);
    default_getter!(calculate_value_added_tax, bool, true);
    default_getter!(timesheet_template, String);
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


pub struct Timesheet {
    worklog: Worklog,
    template_file: String,
    locale: Locale,
}

impl Timesheet {
    pub fn new(template_file: String, locale: Locale) -> Self {
        Self {
            worklog: Worklog::new(),
            template_file: format!("templates/{template_file}"),
            locale: locale.clone(),
        }
    }
    
    pub fn add_record(&mut self, record: WorklogRecord) {
        self.worklog.add_record(record);
    }

    pub fn sort(&mut self) {
        self.worklog.sort()
    }
}

impl GenerateTex for Timesheet {
    fn generate_tex<'a>(&self, w: &'a mut dyn Write) -> std::io::Result<()> {
        let mut template = TexTemplate::new(self.template_file.clone());
        template
            .token("WORKLOG", |w| {
                for record in self.worklog.records() {
                    writeln!(w, "{} & {} & {}\\\\", record.start, self.locale.format_number(record.hours, 2), record.message)?;
                }
                Ok(())
            })
            .generate(w)
    }
}

pub struct Invoice {
    date: DateTime,
    config: InvoiceConfig,
    invoicer: Contact,
    payment: Payment,
    counter: u32,
    recipient: Recipient,
    positions: Vec<InvoicePosition>,
    timesheet: Option<Timesheet>,
    begin_date: DateTime,
    end_date: DateTime,
}

impl Invoice {
    pub fn new(date: DateTime, config: Config, recipient: Recipient) -> Self {
        Invoice {
            date: date,
            config: config.invoice,
            invoicer: config.contact,
            payment: config.payment,
            counter: 0,
            recipient: recipient,
            positions: Vec::new(),
            timesheet: None,
            begin_date: DateTime::MAX,
            end_date: DateTime::MIN,
        }
    }

    pub fn locale(&self) -> Locale {
        match &self.recipient.invoice.locale {
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

    pub fn set_counter(&mut self, counter: u32) {
        self.counter = counter;
    }

    pub fn positions(&self) -> &Vec<InvoicePosition> {
        &self.positions
    }

    pub fn default_rate(&self) -> f32 {
        self.recipient.default_rate
            .unwrap_or(self.payment.default_rate.unwrap_or(100.0))
    }

    pub fn generate_timesheet(&self) -> bool {
        !self.config.timesheet_template().is_empty() || self.timesheet.is_some()
    }

    pub fn add_worklog(&mut self, worklog: &Worklog) {
        let mut positions: BTreeMap<String, InvoicePosition> = BTreeMap::new();

        for record in worklog.records() {
            self.begin_date = record.begin_date().min(self.begin_date);
            self.end_date = record.end_date().max(self.end_date);

            let text = record.message.clone();
            let position = InvoicePosition::from_worklog_record(&record, worklog.rate());
            if positions.contains_key(&text) {
                positions.insert(text, positions.get(&record.message).unwrap().clone() + position);
            } else {
                positions.insert(text, position);
            }

            if self.generate_timesheet() {
                if self.timesheet.is_none() {
                    self.timesheet = Some(Timesheet::new(self.config.timesheet_template(), self.locale()));
                }
                self.timesheet.as_mut().unwrap().add_record(record.clone());
            }
        }

        for (_, position) in positions {
            self.positions.push(position)
        }

        // Sort timesheet each time a worklog was added
        if self.generate_timesheet() {
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
            .replace("${RECIPIENT}", &self.recipient.name)
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
            .token("LANGUAGE", |w| {
                self.locale().generate_tex(w)
            })
            .token("RECIPIENT_ADDRESS", |w| {            
                self.recipient.generate_tex_commands(w, "recipient")
            })
            .token("BILLER_ADDRESS", |w| {            
                self.invoicer.generate_tex_commands(w, "my")
            })
            .token("PAYMENT_DETAILS", |w| {
                self.payment.generate_tex_commands(w, "my")
            })
            .token("INVOICE_DETAILS", |w| {
                let details = InvoiceDetails::from_invoice(&self);
                details.generate_tex_commands(w, "invoice")
            })
            .token("INVOICE_POSITIONS", |w: &mut dyn Write| {
                for position in &self.positions {
                    position.generate_tex(w, &self.locale())?;
                }
                Ok(())
            })
            .token("INVOICE_SUM", |w: &mut dyn Write| {
                let l = self.locale();                
                if self.config.calculate_value_added_tax() {
                    writeln!(w, "\\invoicesum{{{sum}}}{{{tax_rate}}}{{{tax}}}{{{sum_with_tax}}}", 
                        sum = l.format_amount(self.sum()), 
                        tax_rate = self.tax_rate(), 
                        tax = l.format_amount(self.tax()), 
                        sum_with_tax = l.format_amount(self.sum_with_tax()) 
                    )
                } else {
                    writeln!(w, "\\invoicesumnotax{{{sum}}}",
                        sum = l.format_amount(self.sum()), 
                    )
                }
            })
            .token("INVOICE_VALUE_TAX_NOTE", |w| {
                if !self.config.calculate_value_added_tax() {
                    writeln!(w, "\\trinvoicevaluetaxnote")
                } else {
                    Ok(())
                }
            })
            .token("TIMESHEET", |w| {
                if self.generate_timesheet() {
                    writeln!(w, "\\newpage")?;
                    self.timesheet.as_ref().unwrap().generate_tex(w)?; 
                }
                Ok(())
            })
            .generate(w)
    }
}

