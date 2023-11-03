
use invoicer::invoice::*;
use invoicer::worklog::Worklog;

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
    let worklog = Worklog::from_csv_file("examples/ExampleWorklog.csv").unwrap();
    let config = Config::from_toml_file("invoicer.toml").unwrap();
    use invoicer::helpers::FromTomlFile;
    let invoicee = Invoicee::from_toml_file("examples/ExampleInvoicee.toml").unwrap();
    println!("Performance period: {:?} - {:?}", worklog.begin_date(), worklog.end_date());

    let mut f = std::fs::File::create("test.tex").unwrap();

    let mut invoice = Invoice::new(chrono::offset::Local::now().naive_local(), config, invoicee);
    // Set counter
    // invoice.set_counter()
    invoice.add_worklog(&worklog);

    use invoicer::generate_tex::GenerateTex;
    invoice.generate_tex(&mut f);

//    config.contact.generate_tex(&mut f, "my").unwrap();
}
