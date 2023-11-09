use serde::Deserialize;
use toml::value::Date;

use crate::{worklog::Worklog, invoice::{Recipient, Contact, Payment, InvoiceConfig, Invoice}, helpers::{FromTomlFile, DateTime, now}, generate_tex::GenerateTex};


#[derive(Debug, Deserialize, Clone)]
pub struct Folders {
    config: Option<String>,
    tags: Option<String>,
    templates: Option<String>,
    invoices: Option<String>
}


#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    folders: Folders,
    contact: Contact,
    payment: Payment,
    invoice: InvoiceConfig,
}

impl Config {
    pub fn from_toml_file(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        crate::helpers::from_toml_file::<Self>(filename)
    }

    pub fn contact(&self) -> &Contact {
        &self.contact
    }

    pub fn folders(&self) -> &Folders {
        &self.folders
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
        let mut recipients = Recipient::from_tags(self.worklog.tags());
        self.recipients.append(&mut recipients);
    }

    pub fn add_recipient_from_toml_file(&mut self, toml: &str) -> Result<(), Box<dyn std::error::Error>> {
        match Recipient::from_toml_file(&toml) {
            Ok(recipient) => {
                self.recipients.push(recipient);
                Ok(())
            },
            Err(e) => {
                eprintln!("Could not load recipient '{}': {e}!", toml);
                Err(e)
            },
        }
    }


    pub fn generate(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Worklog tags: {:?}", self.worklog.tags());
        println!("Recipients: {:?}", self.recipients.iter().map(|r| r.name().clone()).collect::<Vec<String>>());

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
            
            let tex_file = invoice.filename();
            
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