
use std::sync::mpsc::RecvError;

use invoicer::invoice::*;
use invoicer::worklog::Worklog;
use invoicer::helpers::*;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author="Michael Winkelmann", version, about="Invoicer")]
struct Arguments{
    /// Worklog CSV file
    #[arg(short, long)]
    worklog: Option<Vec<String>>,

    /// Recipient TOML file (optional)
    #[arg(short, long)]
    recipient_toml: Option<Vec<String>>,

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
    let config = Config::from_toml_file(args.config.as_str())?;

    let date = match args.date {
        Some(date_str) => {
            DateTime::parse_from_str((date_str + " 00:00").as_str(), "%Y-%d-%m %H:%M").unwrap()
        },
        None => now()
    };

    // Create a merged worklog from all input worklogs
    // 1) Try to read worklog from stdin
    let mut worklog = if args.stdin {
        Worklog::from_csv(std::io::stdin()).unwrap_or_default() 
    } else { 
        Worklog::new() 
    };

    // 2) Try to read worklog from given commandline arguments
    let worklog_csvs = args.worklog.unwrap_or_default();
    for worklog_csv in worklog_csvs {
        match Worklog::from_csv_file(&worklog_csv) {
            Ok(wl) => {
                worklog.append(&wl);
            }
            Err(e) => eprintln!("Error loading worklog {worklog_csv}: {e}")
        }
    }

    // Create list of recipients from different inputs
    let mut recipients = Vec::new();
    if args.recipient_toml.is_some() {
        for recipient_toml in args.recipient_toml.unwrap() {
            match Recipient::from_toml_file(&recipient_toml) {
                Ok(recipient) => recipients.push(recipient),
                Err(e) => eprintln!("Could not load recipient '{}'!", recipient_toml),
            }
        }
    } 

    println!("Worklog tags: {:?}", worklog.tags());
    
    if recipients.is_empty() {
        // If no recipient is given as command-line argument, try to fetch recipients from work logs
        recipients = Recipient::from_tags(worklog.tags());
    }

    println!("Recipients: {:?}", recipients.iter().map(|r| r.name().clone()).collect::<Vec<String>>());


    // Return if no recipients are given
    if !recipients.is_empty() {
    } else {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No recipient given!")));
    }

    let mut counter = args.counter.unwrap_or(1);

    // Create an invoice for each recipient
    for recipient in &recipients {
        let mut worklog = worklog.from_records_with_tag(recipient.name());
        let mut invoice = Invoice::new(date, config.clone(), recipient.clone());
        worklog.set_rate(invoice.default_rate());

        invoice.set_counter(counter);
            
        use invoicer::generate_tex::GenerateTex;
        let tex_file = match args.invoice_output {
            Some(ref output) => if recipients.len() > 1 { format!("{output}{counter}.tex") } else { format!("{output}.tex") },
            None => invoice.filename()
        };
        
        invoice.add_worklog(&worklog);

        if invoice.positions().is_empty() {
            eprintln!("{tex_file}: Warning: The generated invoice contains no positions, no invoice generated!");
        }
        invoice.generate_tex_file(tex_file.clone())?;

        println!("{tex_file}: Generated invoice '{rec_name}' with {positions} positions, sum (incl. VAT) = {sum}", 
            rec_name = recipient.name(),
            positions = invoice.positions().len(),
            sum = invoice.locale().format_amount(invoice.sum_with_tax()) 
        );

        counter += 1;
    }

    Ok(())
}
