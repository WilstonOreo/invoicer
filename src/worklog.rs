
use serde::Deserialize;
use crate::helpers::DateTime;

#[derive(Debug, Deserialize)]
pub struct WorklogRecord {
    pub start: String,
    pub hours: f32,
    pub rate: f32,
    pub message: String
}

impl WorklogRecord {
    fn begin_date(&self) -> DateTime {
        DateTime::parse_from_str(&self.start, "%m/%d/%Y %H:%M").unwrap()
    }

    fn end_date(&self) -> DateTime {
        let mut date = self.begin_date();
        date += chrono::Duration::seconds((60.0 * 60.0 * self.hours) as i64);
        date
    }

    fn net(&self) -> f32 {
        self.hours * self.rate
    }
}


pub struct Worklog {
    begin_date: DateTime,
    end_date: DateTime,
    records: Vec<WorklogRecord>
}

impl Worklog {

    pub fn new() -> Self {
        Self {
            begin_date: DateTime::MAX,
            end_date: DateTime::MIN,
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
            worklog.begin_date = record.begin_date().min(worklog.begin_date);
            worklog.end_date = record.end_date().max(worklog.end_date);
            worklog.records.push(record);
        }

        Ok(worklog)
    }

    pub fn from_csv_file(filename: &str)  -> Result<Self, Box<dyn std::error::Error>> {
        use std::io::BufReader;
        let file = std::fs::File::open(&filename)?;
        let buf_reader = BufReader::new(file);
        Self::from_csv(buf_reader)
    }

    pub fn sum(&self) -> f32 {
        let mut sum = 0.0_f32;
        for record in &self.records {
            sum += record.net();
        }
        sum
    }

    pub fn sum_with_tax(&self, taxrate: f32) -> f32 {
        self.sum() * (1.0 + taxrate / 100.0)
    }

    pub fn records(&self) -> &Vec<WorklogRecord> {
        &self.records
    }

    pub fn begin_date(&self) -> DateTime {
        self.begin_date
    }

    pub fn end_date(&self) -> DateTime {
        self.end_date
    }
}
