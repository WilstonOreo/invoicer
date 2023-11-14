use std::{path::{PathBuf, Path}, fmt::Display, collections::HashMap, fs::File, iter::FromFn, io::Read};

use chrono::Datelike;
use serde::{Deserialize, Serialize};
use toml::map::Map;

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

    fn mkdir(&self) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(&self.config_dir())?;
        std::fs::create_dir_all(&self.tag_dir())?;
        std::fs::create_dir_all(&self.template_dir())?;
        std::fs::create_dir_all(&self.invoice_dir())?;
        Ok(())
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
enum OverwriteBehaviour {
    Force,
    RenameOld,
    RenameNew,
    Skip,
}

impl Default for OverwriteBehaviour {
    fn default() -> Self {
        OverwriteBehaviour::RenameOld
    }
}


#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pdf_generator: Option<String>,
    #[serde(default)]
    overwrite: OverwriteBehaviour,
    #[serde(default)]
    directories: Directories,
    contact: Contact,
    payment: Payment,
    invoice: InvoiceConfig,
}

pub fn toml_file_to_map<P: FilePath>(p: P)  -> Result<Map<String, toml::Value>, Box<dyn std::error::Error>> {
    let path_str = p.to_string();
    let mut file = std::fs::File::open(p)?;
    let mut s = String::new();
    file.read_to_string(&mut s)?;
    
    match toml::from_str(&s) {
        Ok(result) => Ok(result),
        Err(err) => {
            eprintln!("Error reading {}: {err}", path_str);
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{err}"))))
        }
    }
}


impl Config {
    pub fn from_toml_file<P: FilePath>(filename: P) -> Result<Self, Box<dyn std::error::Error>> {
        crate::helpers::from_toml_file::<Self, P>(filename)
    }

    pub fn from_toml_files(filename: Option<impl FilePath>) -> Result<Self, Box<dyn std::error::Error>> {
        
        let mut toml = toml::Table::new();

        fn merge_map(p: PathBuf, toml: &mut Map<String, toml::Value>) {
            if p.exists() {
                let map = toml_file_to_map(p).unwrap();
                for (key, value) in map {
                    toml.insert(key, value);
                }
            }
        }

        merge_map(home::home_dir().unwrap().join("invoicer.toml"), &mut toml);
        merge_map(std::env::current_dir().unwrap().join("invoicer.toml"), &mut toml);
        if let Some(filename) = filename {
            merge_map(PathBuf::from(&filename), &mut toml);
        }

        Ok(Self::deserialize(toml).unwrap())
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

    pub fn set_invoice_dir(&mut self, p: impl FilePath) {
        self.directories.invoices = Some(p.to_string());
    }
}



pub struct InvoiceFingerprints(bimap::BiMap<String, String>);


impl InvoiceFingerprints {
    pub fn add(&mut self, invoice: &Invoice) {
        self.0.insert(invoice.fingerprint(), invoice.number());
    }

    pub fn contains_fingerprint(&self, f: String) -> bool {
        self.0.contains_left(&f)
    }

    pub fn contains_number(&self, n: String) -> bool {
        self.0.contains_right(&n)
    }

    pub fn number_for_fingerprint(&self, f: String) -> String {
        self.0.get_by_right(&f).unwrap().clone()
    }
}

impl Default for InvoiceFingerprints {
    fn default() -> Self {
        InvoiceFingerprints(bimap::BiMap::new())
    }
}

impl From<HashMap<String, String>> for InvoiceFingerprints {
    fn from(map: HashMap<String, String>) -> Self {
        let mut bimap = bimap::BiMap::new();
        for (k, v) in map {
            bimap.insert(k, v);
        }
        Self(bimap)
    }
}

impl FromTomlFile for InvoiceFingerprints {}

impl<'de> Deserialize<'de>  for InvoiceFingerprints {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s: HashMap<String, String> = Deserialize::deserialize(deserializer)?;
        Ok(Self::from(s))
    }
}

impl Serialize for InvoiceFingerprints {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(&k, &v)?;
        }
        map.end()
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
            config: config.clone(),
            date: date.unwrap_or(now()),
            counter: counter.unwrap_or(1),
            worklog: Worklog::new(),
            recipients: Vec::new(),
        }
    }

    fn fingerprint_file(&self) -> PathBuf {
        self.config.directories.config_dir().join("fingerprints.toml")
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
        
        self.mkdir()?;


        // Return if no recipients are given
        if self.recipients.is_empty() {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No recipient given!")));
        }

        let mut counter = self.counter;

        let mut fingerprints = InvoiceFingerprints::from_toml_file(self.fingerprint_file()).unwrap_or_default();

        // Create an invoice for each recipient
        for recipient in &self.recipients {
            let mut worklog = self.worklog.from_records_with_tag(recipient.name());
            let mut invoice = Invoice::new(&self,  recipient.clone());
            worklog.set_rate(invoice.default_rate());

            counter = invoice.generate_number(counter, Some(&fingerprints));
            
            let tex_file = Path::new(&self.invoice_dir()).join(invoice.filename());
            invoice.add_worklog(&worklog);

            if invoice.positions().is_empty() {
                eprintln!("{:?}: Warning: The generated invoice contains no positions, no invoice will be generated!", tex_file);
                continue;
            }

            if tex_file.exists() {
                eprintln!("{:?}: Warning: The tex file to be generated already exists.", tex_file);
                continue;
            }

            invoice.generate_tex_file(tex_file.clone())?;
            fingerprints.add(&invoice);

            let sum_text = if invoice.calculate_value_added_tax() {
                format!("total (incl. VAT) = {sum}", sum = invoice.locale().format_amount(invoice.sum_with_tax()))
            } else {
                format!("total = {sum}", sum = invoice.locale().format_amount(invoice.sum()))
            };

            println!("{:?}: {positions} positions, {sum}", 
                tex_file,
                positions = invoice.positions().len(),
                sum = sum_text
            );
        }

        // Save fingerprint file
        use std::io::Write;
        let s = toml::to_string(&fingerprints).unwrap();
        let mut f = std::fs::File::create(self.fingerprint_file())?;
        write!(f, "{}", s)?;

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

        println!("worklog_tags: {:?}", self.worklog.tags());
        println!("recipients: {:?}", self.recipients.iter().map(|r| r.name().clone()).collect::<Vec<String>>());

        Ok(())
    }
}