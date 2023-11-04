# Invoicer

**Invoicer** creates invoices based on LateX templates from CSV worklogs.

## TL;DR

```shell
invoicer -i Invoicee.toml -w worklog.csv -o output.tex
```

## Usage

Invoicer needs an *invoicee* and one or several *worklogs* as input.
The application can be configured by editing the default config `invoicer.toml` or by providing a custom config as TOML file.

### Invoicee

The invoicee information can be retrieved from a TOML file, which looks like this:

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

### Worklog

Worklogs in CSV format can be used as source to generate invoice positions.
Worklogs CSVs can be generated with [jobber](https://github.com/fightling/jobber).

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
invoicer -i Invoicee.toml -w worklog_october.csv -w worklog_december.csv -o output.tex
```

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

Invoice can have different *locales* (aka language)

## Default template

The default LaTex template is located in `templates/invoice.tex`.
You can either edit this template or copy it and enter the new template filename in `invoicer.toml`.
