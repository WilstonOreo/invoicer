


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

struct Payment {
    account_holder: Option<String>,
    iban: String,
    bic: String,
    taxid: String,
}


type DateTime = chrono::DateTime<chrono::Utc>;

#[derive(Debug, serde::Deserialize)]
struct WorklogRecord {
    start: String,
    hours: f32,
    rate: f32,
    message: String
}

struct Worklog {
    start_date: DateTime,
    end_date: DateTime,
    records: Vec<WorklogRecord>
}

impl Worklog {

    pub fn new() -> Self {
        Self {
            start_date: DateTime::MAX_UTC,
            end_date: DateTime::MIN_UTC,
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
            println!("{:?}", record);
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
    println!("Hello, world!");

    let worklog = Worklog::from_csv_file("examples/ExampleWorklog.csv").unwrap();


}
