# Invoicer

**Invoicer** creates invoices based on LateX templates from CSV worklogs.

## TL;DR

```shell
invoicer -r Recipient.toml -w worklog.csv -o output.tex
```

Using [jobber](https://github.com/fightling/jobber):

```shell
jobber --export > invoicer
```

## Usage

Invoicer needs a *recipient* and one or several *worklogs* as input.
The application can be configured by editing the default config `invoicer.toml` or by providing a custom config as TOML file.

### Recipient

The recipient information can be retrieved from a TOML file, which looks like this:

```toml
companyname = "Example Client"
locale = "de"

[contact]
fullname = "Jane Doe"
street = "123 Fake St."
email = "jane.doe@exampleclient.com"
zipcode = 1234
city = "Berlin"
country = "Germany"
phone = "+49123456789"
```

### Worklog without tags

Worklogs in CSV format can be used as source to generate invoice positions.
Worklogs CSVs can be generated with [jobber](https://github.com/fightling/jobber):

```shell
jobber --export 2.10...31.10. --tags SomeTag --csv "Start,Hours,Rate,Message"
```

An example for worklog may look like this:

```csv
"Start","Hours","Rate","Message"
"10/04/2023 14:00",3,100,"Source Code Review"
"10/05/2023 14:00",2,100,"Source Code Review"
"10/05/2023 16:00",1,100,"Account Setup"
"10/16/2023 09:00",0.5,100,"Source Code Review"
"10/16/2023 09:30",1.5,100,"Discussion"
```

You can also add severals worklogs at once:

```shell
invoicer -r Recipient.toml -w worklog_october.csv -w worklog_december.csv -o output.tex
```


### Worklog with tags

An example for worklog with tags may look like this:

```csv
"Tags","Start","Hours","Message"
"CustomerB,donuts","10/04/2023 14:00",3,"Yummy"
"ExampleRecipient,dev","10/05/2023 14:00",2,"Source Code Review"
"CustomerB,beer","10/05/2023 16:00",1,"Getting drunk"
"ExampleRecipient,dev","10/16/2023 09:00",0.5,"Source Code Review"
"ExampleRecipient,dev","10/16/2023 09:30",1.5,"Discussion"
```

*CustomerB" and "ExampleRecipient" have tags defined in TOML files.
The TOML contain a recipients' address and a list of tasks, like the file `ExampleRecipient.toml`:

```toml
[contact]
companyname = "Eine kleine Firma"
fullname = "Max Mustermann"
street = "Musterstraße 12"
email = "m@firma.com"
zipcode = 1234
city = "Berlin"
country = "Germany"

[tasks]
dev = "Software Development"
```

```shell
invoicer -w worklog.csv
```

You can also use `jobber` to pipe its export output into `invoicer`:

```shell
jobber --export > invoicer
```

## Output

The output is a tex file `output.tex`, which can compiled to PDF with MikTeX or TexLive.
The `-o` argument is optional, the output name can be generated via the format string given in the `invoicer.toml`.

### Windows

```powershell
pdflatex.exe .\output.tex
```

### Linux

```shell
pdflatex .\output.tex
```

## Locales

An invoice can have different *locales* (aka language), which are stored in the `locales` folder as TOML files.
Currently, only `de` (German) and `en` (English within EU) are supported.

## Default template

The default LaTex template is located in `templates/invoice.tex`.
You can either edit this template or copy it and enter the new template filename in `invoicer.toml`.
