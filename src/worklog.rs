
use serde::{Deserialize, Deserializer};
use crate::helpers::DateTime;

#[derive(Debug, Deserialize, Clone)]
pub struct WorklogRecord {
    #[serde(rename = "Tags", deserialize_with = "deserialize_tags")]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "Start")]
    pub start: String,
    #[serde(rename = "Hours")]
    pub hours: f32,
    #[serde(rename = "Rate")]
    pub rate: Option<f32>,
    #[serde(rename = "Message")]
    pub message: String
}

fn deserialize_tags<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where D: Deserializer<'de> {
    let buf = String::deserialize(deserializer);
    if buf.is_err() {
        return Ok(None);
    }
    let buf = buf.unwrap();

    let s = buf.split(",").map(|ss| ss.trim().to_string() ).collect::<Vec<String>>();
    Ok(Some(s))
}

impl WorklogRecord {
    pub fn begin_date(&self) -> DateTime {
        DateTime::parse_from_str(&self.start, "%m/%d/%Y %H:%M").unwrap()
    }

    pub fn end_date(&self) -> DateTime {
        let mut date = self.begin_date();
        date += chrono::Duration::seconds((60.0 * 60.0 * self.hours) as i64);
        date
    }

    fn net(&self) -> f32 {
        self.hours * self.rate.unwrap_or_default()
    }
}


pub struct Worklog {
    begin_date: DateTime,
    end_date: DateTime,
    records: Vec<WorklogRecord>,
    rate: f32
}

impl Worklog {

    pub fn new() -> Self {
        Self {
            begin_date: DateTime::MAX,
            end_date: DateTime::MIN,
            records: Vec::new(),
            rate: 100.0,
        }
    }
    
    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate;
    }

    pub fn from_csv(reader: impl std::io::Read) -> Result<Self, Box<dyn std::error::Error>> {
        let mut rdr = csv::ReaderBuilder::new()
            .from_reader(reader);
        let mut worklog = Self::new();
        
        for result in rdr.deserialize() {
            // Notice that we need to provide a type hint for automatic
            // deserialization.
            let record: WorklogRecord = result?;
            worklog.add_record(record);
        }

        Ok(worklog)
    }
    pub fn add_record(&mut self, record: WorklogRecord) {
        self.begin_date = record.begin_date().min(self.begin_date);
        self.end_date = record.end_date().max(self.end_date);
        self.records.push(record);
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

    pub fn sort(&mut self) {
        self.records.sort_by_key(|r| r.begin_date());
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
