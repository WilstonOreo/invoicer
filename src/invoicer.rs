use std::{path::{PathBuf, Path}, fmt::Display};

use chrono::Datelike;
use serde::Deserialize;

use crate::{worklog::Worklog, invoice::*, helpers::*, generate_tex::GenerateTex};

pub trait HasDirectories {
    fn config_dir(&self) -> PathBuf;
    fn tag_dir(&self) -> PathBuf;
    fn template_dir(&self) -> PathBuf;
    fn invoice_dir(&self) -> PathBuf;
    fn locale_dir(&self) -> PathBuf;

    fn working_dir(&self) -> PathBuf {
        std::env::current_dir().unwrap()
    }

    fn format_path(&self, s: &String) -> String { s.clone() }

    fn mkdir(&self) {
        std::fs::create_dir_all(&self.config_dir());
        std::fs::create_dir_all(&self.tag_dir());
        std::fs::create_dir_all(&self.template_dir());
        std::fs::create_dir_all(&self.invoice_dir());
    }
}


#[derive(Debug, Deserialize, Clone, Default)]
struct Directories {
    config: Option<String>,
    tags: Option<String>,
    templates: Option<String>,
    invoices: Option<String>,
    locales: Option<String>,
}


impl HasDirectories for Directories {
    fn config_dir(&self) -> PathBuf {
        self.config.as_ref().unwrap_or(&String::from("${HOME}/.invoicer"))
        .replace("${HOME}", &home_dir())
        .replace("${WORKING_DIR}", &self.working_dir().to_string()).into()
    }

    fn tag_dir(&self) -> PathBuf {
        self.format_path(&self.tags.as_ref().unwrap_or(&String::from("${CONFIG_DIR}/tags"))).into()
    }

    fn template_dir(&self) -> PathBuf {
        self.format_path(&self.templates.as_ref().unwrap_or(&String::from("${CONFIG_DIR}/templates"))).into()
    }

    fn invoice_dir(&self) -> PathBuf {
        self.format_path(&self.invoices.as_ref().unwrap_or(&String::from("${HOME}/Documents/invoices/${YEAR}"))).into()
    }

    fn locale_dir(&self) -> PathBuf {
        self.format_path(&self.locales.as_ref().unwrap_or(&String::from("${CONFIG_DIR}/locales"))).into()
    }

    fn format_path(&self, s: &String) -> String {
        s.replace("${HOME}", &home_dir())
            .replace("${WORKING_DIR}", &std::env::current_dir().unwrap().into_os_string().into_string().unwrap())
            .replace("${CONFIG_DIR}", &self.config_dir().into_os_string().into_string().unwrap())
    }
}



#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    directories: Directories,
    contact: Contact,
    payment: Payment,
    invoice: InvoiceConfig,
}

impl Config {
    pub fn from_toml_file<P: FilePath>(filename: P) -> Result<Self, Box<dyn std::error::Error>> {
        crate::helpers::from_toml_file::<Self, P>(filename)
    }

    pub fn contact(&self) -> &Contact {
        &self.contact
    }

    pub fn payment(&self) -> &Payment {
        &self.payment
    }

    pub fn invoice(&self) -> &InvoiceConfig {
        &self.invoice
    }
}

pub struct Invoicer {
    config: Config,
    date: DateTime,
    counter: u32,
    worklog: Worklog,
    recipients: Vec<Recipient>,
}

impl Invoicer {
    pub fn new(config: Config, date: Option<DateTime>, counter: Option<u32>) -> Self {
        Self {
            config: config,
            date: date.unwrap_or(now()),
            counter: counter.unwrap_or(1),
            worklog: Worklog::new(),
            recipients: Vec::new()
        }
    }

    pub fn append_worklog(&mut self, worklog: &Worklog) {
        self.worklog.append(worklog);
    }

    pub fn append_worklog_from_csv_file(&mut self, csv: &str) -> Result<(), Box<dyn std::error::Error>> {
        match Worklog::from_csv_file(&csv) {
            Ok(worklog) => {
                self.append_worklog(&worklog);
                Ok(())
            }
            Err(e) => {
                eprintln!("Error loading worklog {csv}: {e}");
                Err(e)
            }
        }
    }

    pub fn has_recipients(&self) -> bool {
        !self.recipients.is_empty()
    }

    pub fn add_recipient(&mut self, recipient: Recipient) {
        self.recipients.push(recipient);
    }

    pub fn add_recipients_from_worklog(&mut self) {
        let mut recipients = Recipient::from_tags(self.worklog.tags(), &self.tag_dir());
        self.recipients.append(&mut recipients);
    }

    pub fn add_recipient_from_toml_file<P: FilePath>(&mut self, toml: P) -> Result<(), Box<dyn std::error::Error>> {
        let s = toml.to_string();
        match Recipient::from_toml_file(toml) {
            Ok(recipient) => {
                self.recipients.push(recipient);
                Ok(())
            },
            Err(e) => {
                eprintln!("Could not load recipient '{}': {e}!", s);
                Err(e)
            },
        }
    }


    pub fn generate(&self) -> Result<(), Box<dyn std::error::Error>> {

        println!("{}", self);
        
        self.mkdir();


        // Return if no recipients are given
        if self.recipients.is_empty() {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No recipient given!")));
        }

        let mut counter = self.counter;

        // Create an invoice for each recipient
        for recipient in &self.recipients {
            let mut worklog = self.worklog.from_records_with_tag(recipient.name());
            let mut invoice = Invoice::new(&self,  recipient.clone());
            worklog.set_rate(invoice.default_rate());

            invoice.set_counter(counter);
            
            let tex_file: String = Path::new(&self.invoice_dir()).join(invoice.filename()).into_os_string().into_string().unwrap();
            
            /*match invoice.output_file() {
                Some(ref output) => if self.recipients.len() > 1 { format!("{output}{counter}.tex") } else { format!("{output}.tex") },
                None => invoice.filename()
            };*/ // TODO: Output from command line
        
            invoice.add_worklog(&worklog);

            if invoice.positions().is_empty() {
                eprintln!("{tex_file}: Warning: The generated invoice contains no positions, no invoice will be generated!");
                continue;
            }

            invoice.generate_tex_file(tex_file.clone())?;

            let sum_text = if invoice.calculate_value_added_tax() {
                format!("total (incl. VAT) = {sum}", sum = invoice.locale().format_amount(invoice.sum_with_tax()))
            } else {
                format!("total = {sum}", sum = invoice.locale().format_amount(invoice.sum()))
            };

            println!("{tex_file}: {positions} positions, {sum}", 
                positions = invoice.positions().len(),
                sum = sum_text
            );

            counter += 1;
        }

        Ok(())
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn date(&self) -> DateTime {
        self.date
    }
}


impl HasDirectories for Invoicer {
    fn config_dir(&self) -> PathBuf {
        self.config().directories.config_dir()
    }

    fn tag_dir(&self) -> PathBuf {
        self.config().directories.tag_dir()
    }

    fn template_dir(&self) -> PathBuf {
        self.config().directories.template_dir()
    }

    fn locale_dir(&self) -> PathBuf {
        self.config().directories.locale_dir()
    }

    fn invoice_dir(&self) -> PathBuf {
        self.config().directories.invoice_dir()
            .to_string()
            .replace("${YEAR}", &self.date().year().to_string()).into()
    }
}

impl Display for Invoicer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Invoicer:")?;
        writeln!(f, "\tDirectories:")?;
        writeln!(f, "\t\tConfig:\t{:?}", self.config_dir())?;
        writeln!(f, "\t\tTemplates:\t{:?}", self.template_dir())?;
        writeln!(f, "\t\tTags:\t{:?}", self.tag_dir())?;
        writeln!(f, "\t\tLocales:\t{:?}", self.locale_dir())?;

        println!("Worklog tags: {:?}", self.worklog.tags());
        println!("Recipients: {:?}", self.recipients.iter().map(|r| r.name().clone()).collect::<Vec<String>>());

        Ok(())
    }
}