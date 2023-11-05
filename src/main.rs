
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
    recipient_toml: Option<String>,

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
    date: Option<String>
}



fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Arguments::parse();

    if args.recipient_toml.is_none() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No recipient given!")));
    }

    let date = match args.date {
        Some(date_str) => {
            DateTime::parse_from_str((date_str + " 00:00").as_str(), "%Y-%d-%m %H:%M").unwrap()
        },
        None => now()
    };

    let config = Config::from_toml_file(args.config.as_str())?;
    let recipient = Recipient::from_toml_file(&args.recipient_toml.unwrap())?;

    let mut invoice = Invoice::new(date, config, recipient);

    if args.counter.is_some() {
        invoice.set_counter(args.counter.unwrap());
    }

    let worklogs = args.worklog.unwrap_or_default();

    for worklog in worklogs {
        match Worklog::from_csv_file(&worklog) {
            Ok(mut worklog) => {
                worklog.set_rate(invoice.default_rate());
                invoice.add_worklog(&worklog);
            }
            Err(e) => eprintln!("Error loading worklog {worklog}: {e}")
        }
    }

    if invoice.positions().is_empty() {
        eprintln!("Warning: The generated invoice contains no positions!");
    }

    use invoicer::generate_tex::GenerateTex;
    invoice.generate_tex_file(args.invoice_output.unwrap_or(invoice.filename()))?;

    Ok(())
}
