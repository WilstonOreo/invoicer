use std::path::PathBuf;

use invoicer::invoicer::{Invoicer, Config};
use invoicer::worklog::Worklog;
use invoicer::helpers::*;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author="Michael Winkelmann", version, about="Invoicer")]
struct Arguments{
    /// Worklog CSV file
    #[arg(short, long)]
    worklog: Vec<String>,

    /// Recipient TOML file (optional)
    #[arg(short, long)]
    recipient_toml: Vec<String>,

    /// Optional latex output file
    #[arg(short = 'o', long)]
    invoice_output: Option<String>,

    /// Optional config file. 
    #[arg(short, long, default_value = "invoicer.toml")]
    config: String,

    /// Optional counter for the invoice to generate an invoice number
    #[arg(short = 'n', long)]
    counter: Option<u32>,

    /// Optional invoice date in format %Y-m%-%d. If no date is given, current date is used.
    #[arg(short = 'd', long)]
    date: Option<String>,

    /// Read from stdin
    #[clap(long, action)]
    stdin: bool,
}



fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Arguments::parse();
    let config = Config::from_toml_file::<PathBuf>(args.config.into())?;

    let date = match args.date {
        Some(date_str) => {
            DateTime::parse_from_str((date_str + " 00:00").as_str(), "%Y-%d-%m %H:%M").unwrap()
        },
        None => now()
    };

    let mut invoicer = Invoicer::new(config, Some(date), args.counter);

    // Create a merged worklog from all input worklogs
    // 1) Try to read worklog from stdin    
    if args.stdin {
        match Worklog::from_csv(std::io::stdin()) {
            Ok(worklog) => invoicer.append_worklog(&worklog),
            Err(e) => eprintln!("Could not read worklog CSV from stdin: {e}"),
        }
    }

    // 2) Try to read worklog from given commandline arguments
    let worklog_csvs = args.worklog;
    for worklog_csv in worklog_csvs {
        invoicer.append_worklog_from_csv_file(&worklog_csv)?;
    } 

    // 3) Create list of recipients from toml files
    for recipient_toml in args.recipient_toml {
        invoicer.add_recipient_from_toml_file::<PathBuf>(recipient_toml.into())?;
    }

    // 4) Try to fetch recipients from worklogs
    if !invoicer.has_recipients() {
        // If no recipient is given as command-line argument, try to fetch recipients from worklog
        invoicer.add_recipients_from_worklog();
    }

    invoicer.generate()
}
