use chrono::Datelike;
use serde::{Deserialize, Deserializer};
use std::io::Write;
use std::path::{PathBuf, Path};
use crate::invoicer::{Config, Invoicer, HasDirectories};
use crate::locale::{Currency, Locale};
use crate::generate_tex::*;
use crate::helpers::{ from_toml_file, DateTime, date_to_str, FromTomlFile, FilePath };
use crate::worklog::{ Worklog, WorklogRecord };

use std::collections::{HashMap, BTreeMap, HashSet};

use struct_iterable::Iterable;

#[derive(Debug, Deserialize, Iterable, Clone)]
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

#[derive(Debug, Deserialize, Iterable, Clone)]
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

#[derive(Debug, Iterable, Clone)]
pub struct RecipientTagInfo {
    is_default: bool,
    position_text: String,
}

impl<'de> Deserialize<'de> for RecipientTagInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Ok(Self::from(s))
    }
}

impl From<String> for RecipientTagInfo {
    fn from(value: String) -> Self {
        let value = value.trim();
        Self {
            is_default: value.starts_with("[default]"),
            position_text: value.replacen("[default]", "", 1)
        }
    }
}

impl From<&str> for RecipientTagInfo {
    fn from(value: &str) -> Self {
        let value = value.trim();
        Self {
            is_default: value.starts_with("[default]"),
            position_text: value.replacen("[default]", "", 1)
        }
    }
}



#[derive(Debug, Deserialize, Iterable, Clone)]
pub struct Recipient {
    #[serde(skip)]
    name: String,
    contact: Contact,
    invoice: InvoiceConfig,
    default_rate: Option<f32>,
    tags: HashMap<String, RecipientTagInfo>
}

impl Recipient {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn from_tag(tag: &String, tag_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_toml_file(Path::new(tag_dir).join(format!("{tag}.toml")))
    }

    pub fn from_tags(tags: &HashSet<String>, tag_dir: &Path) -> Vec<Self> {
        let mut v = Vec::new();
        for tag in tags {
            if let Ok(recipient) = Self::from_tag(tag, tag_dir) {
                v.push(recipient);
            }
        }
        v
    }

    pub fn tags(&self) -> &HashMap<String, RecipientTagInfo> {
        &self.tags
    }

    pub fn default_tag_name(&self) -> Option<&String> {
        for (name, tag) in &self.tags {
            if tag.is_default {
                return Some(name);
            }
        }

        None
    }
}


impl FromTomlFile for Recipient {
    fn from_toml_file<P: FilePath>(p: P)  -> Result<Self, Box<dyn std::error::Error>> {
        let name_str = p.to_string();
        let mut recipient: Recipient = crate::helpers::from_toml_file(p)?;
        recipient.name = crate::helpers::name_from_file::<PathBuf>(name_str.into());

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


#[derive(Debug, Deserialize, Clone)]
pub struct InvoiceConfig {
    #[serde(rename = "locale")]
    locale_str: Option<String>,
    template: Option<String>,
    date_format: Option<String>,
    number_format: Option<String>,
    filename_format: Option<String>,
    days_for_payment: Option<u32>,
    calculate_value_added_tax: Option<bool>,
    timesheet_template: Option<String>,
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
    default_getter!(locale_str, String, "en");
    default_getter!(template, String, "invoice.tex");
    default_getter!(date_format, String, "%Y/%m/%d");
    default_getter!(number_format, String, "%Y%m${COUNTER}");
    default_getter!(filename_format, String, "${INVOICENUMBER}_${INVOICE}_${RECIPIENT}.tex");
    default_getter!(days_for_payment, u32, 14_u32);
    default_getter!(calculate_value_added_tax, bool, true);
    default_getter!(timesheet_template, String);
}




use std::ops::AddAssign;


pub struct Timesheet {
    worklog: Worklog,
    template_file: String,
    template_dir: String,
    locale: Locale,
}

impl Timesheet {
    pub fn new<P: FilePath>(template_file: P, locale: Locale) -> Self {
        Self {
            worklog: Worklog::new(),
            template_file: template_file.file_name(),
            template_dir: template_file.parent(),
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

    fn template_dir(&self) -> PathBuf {
        self.template_dir.clone().into()
    }
}

pub struct Invoice<'a> {
    invoicer: &'a Invoicer,
    config: InvoiceConfig,
    counter: u32,
    recipient: Recipient,
    positions: Vec<InvoicePosition>,
    timesheet: Option<Timesheet>,
    begin_date: DateTime,
    end_date: DateTime,
}

impl<'a> Invoice<'a> {
    pub fn new(invoicer: &'a Invoicer, recipient: Recipient) -> Self {
        Invoice {
            invoicer: invoicer,
            counter: 0,
            config: invoicer.config().invoice().clone(),
            recipient: recipient,
            positions: Vec::new(),
            timesheet: None,
            begin_date: DateTime::MAX,
            end_date: DateTime::MIN,
        }
    }

    pub fn locale(&self) -> Locale {
        let locale_str = match &self.recipient.invoice.locale_str {
            Some(locale) => locale.clone(),
            None => match &self.config.locale_str {
                Some(locale) => locale.clone(),
                None => String::from("en")
            }
        };

        Locale::from_toml_file(self.invoicer.locale_dir().join(format!("{}.toml", locale_str))).unwrap()
    }

    pub fn date(&self) -> DateTime {
        self.invoicer.date()
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
            .unwrap_or(self.payment().default_rate.unwrap_or(100.0))
    }

    pub fn generate_timesheet(&self) -> bool {
        !self.config.timesheet_template().is_empty() || self.timesheet.is_some()
    }

    pub fn add_worklog(&mut self, worklog: &Worklog) {
        let mut positions: BTreeMap<String, InvoicePosition> = BTreeMap::new();

        for record in worklog.records() {
            self.begin_date = record.begin_date().min(self.begin_date);
            self.end_date = record.end_date().max(self.end_date);

            let tags = self.recipient.tags();

            let mut position = InvoicePosition::from_worklog_record(&record, worklog.rate());

            let mut key = String::new();
            for tag in &record.tags() {
                if tags.contains_key(tag) {
                    key = tag.clone(); 
                    position.text = self.recipient.tags().get(&key).unwrap().position_text.clone();
                }
            }

            if key.is_empty() {
                if let Some(default_tag_name) = self.recipient.default_tag_name() {
                    key = default_tag_name.clone();
                    position.text = self.recipient.tags().get(&key).unwrap().position_text.clone();
                } else {
                    key = record.message.clone();
                }   
            }

            positions.entry(key).and_modify(|k| *k += position.clone()).or_insert(position);
            
            if self.generate_timesheet() {
                if self.timesheet.is_none() {
                    self.timesheet = Some(Timesheet::new(Path::new(&self.template_dir()).join(self.config.timesheet_template()), self.locale()));
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
        let date = self.invoicer.date();
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

    pub fn payment(&self) -> &Payment {
        &self.invoicer.config().payment()
    }

    pub fn tax_rate(&self) -> f32 {
        self.payment().tax_rate
    }

    pub fn currency(&self) -> Currency {
        self.payment().currency()
    }

    pub fn currency_symbol(&self) -> String {
        self.payment().currency_symbol()
    }

    pub fn calculate_value_added_tax(&self) -> bool {
        self.config.calculate_value_added_tax()
    }

    pub fn filename(&self) -> String {
        self.config.filename_format()
            .replace("${INVOICENUMBER}", self.number().as_str())
            .replace("${INVOICE}", &self.locale().tr("invoice".to_string()))
            .replace("${RECIPIENT}", &self.recipient.name)
    }
}

#[derive(Debug, Iterable)]
struct InvoiceDetails {
    date: String,
    number: String,
    periodbegin: String,
    periodend: String,
    daysforpayment: u32,
}

impl InvoiceDetails {
    pub fn from_invoice<'a>(invoice: &'a Invoice) -> Self {
        let date_format = invoice.config.date_format();

        Self {
            date: date_to_str(invoice.date(), &date_format),
            number: invoice.number(),
            periodbegin: date_to_str(invoice.begin_date(), &date_format),
            periodend: date_to_str(invoice.end_date(), &date_format),
            daysforpayment: invoice.config.days_for_payment()
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

impl AddAssign for InvoicePosition {

    fn add_assign(&mut self, other: Self) {
        assert!(self.unit == other.unit && self.text == other.text);

        let sum = self.amount + other.amount; 
        *self = InvoicePosition {
            text: self.text.clone(), 
            amount: sum,
            price_per_item: (self.amount * self.price_per_item + other.amount * other.price_per_item) / sum,
            unit: self.unit.clone()
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



impl<'a> GenerateTex for Invoice<'a> {
    fn generate_tex(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let mut template = TexTemplate::new(format!("templates/{}", &self.config.template())); 
        
        template
            .token("LANGUAGE", |w| {
                self.locale().generate_tex(w)
            })
            .token("RECIPIENT_ADDRESS", |w| {            
                self.recipient.generate_tex_commands(w, "recipient")
            })
            .token("BILLER_ADDRESS", |w| {            
                self.invoicer.config().contact().generate_tex_commands(w, "my")
            })
            .token("PAYMENT_DETAILS", |w| {
                self.payment().generate_tex_commands(w, "my")
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

    fn template_dir(&self) -> PathBuf {
        self.invoicer.template_dir()
    }
}

