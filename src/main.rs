
use invoicer::invoice::*;
use invoicer::worklog::Worklog;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author="Michael Winkelmann", version, about="Invoicer")]
struct Arguments{
    #[arg(long)]
    worklog: Option<Vec<String>>,
    #[arg(short, long)]
    invoicee_toml: Option<String>,
    #[arg(long)]
    invoice_output: Option<String>,
    #[arg(short, long)]
    config: Option<String>,
}



fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Arguments::parse();

    if args.invoicee_toml.is_none() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "No invoicee given!")));
    }

    use invoicer::helpers::FromTomlFile;
    let config = Config::from_toml_file(args.config.unwrap_or("invoicer.toml".to_string()).as_str())?;
    let invoicee = Invoicee::from_toml_file(&args.invoicee_toml.unwrap())?;

    let mut invoice = Invoice::new(chrono::offset::Local::now().naive_local(), config, invoicee);

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
