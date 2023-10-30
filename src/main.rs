


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







#[derive(Parser, Debug)]
#[command(author="Michael Winkelmann", version, about="Invoicer")]
struct Arguments{
    #[arg(long, default_value_t = String::from(""))]
    worklog_csv: String,
    #[arg(long, default_value_t = String::from(""))]
    invoicee_toml: String,
    #[arg(short, long, default_value_t = String::from("/home/pi/data"))]
    my_address: String,
}



fn main() {
    println!("Hello, world!");
}
