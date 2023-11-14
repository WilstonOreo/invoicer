
use std::collections::HashSet;

use serde::{Deserialize, Deserializer};
use crate::helpers::DateTime;

#[derive(Debug, Deserialize, Clone)]
pub struct WorklogRecord {
    #[serde(rename = "Tags", deserialize_with = "deserialize_tags")]
    pub tags: Option<HashSet<String>>,
    #[serde(rename = "Start")]
    pub start: String,
    #[serde(rename = "Hours")]
    pub hours: f32,
    #[serde(rename = "Rate")]
    pub rate: Option<f32>,
    #[serde(rename = "Message")]
    pub message: String
}

fn deserialize_tags<'de, D>(deserializer: D) -> Result<Option<HashSet<String>>, D::Error>
where D: Deserializer<'de> {
    let buf = String::deserialize(deserializer);
    if buf.is_err() {
        return Ok(None);
    }
    let buf = buf.unwrap();

    let s = buf.split(",").map(|ss| ss.trim().to_string() ).collect::<HashSet<String>>();
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

    pub fn net(&self) -> f32 {
        self.hours * self.rate.unwrap_or_default()
    }

    pub fn tags(&self) -> HashSet<String> {
        match &self.tags {
            Some(tags) => tags.clone(),
            None => HashSet::new()
        }
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        match &self.tags {
            Some(tags) => tags.contains(tag),
            None => false
        }
    }
}


pub struct Worklog {
    begin_date: DateTime,
    end_date: DateTime,
    records: Vec<WorklogRecord>,
    tags: HashSet<String>,
    rate: f32
}

impl Worklog {
    pub fn new() -> Self {
        Self {
            begin_date: DateTime::MAX,
            end_date: DateTime::MIN,
            records: Vec::new(),
            rate: 100.0,
            tags: HashSet::new(),
        }
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

    pub fn from_csv_file(filename: &str)  -> Result<Self, Box<dyn std::error::Error>> {
        use std::io::BufReader;
        let file = std::fs::File::open(&filename)?;
        let buf_reader = BufReader::new(file);
        Self::from_csv(buf_reader)
    }

    pub fn from_records_with_tag(&self, tag: &str) -> Self {
        let mut worklog = Worklog::new();

        for record in self.records() {
            if record.has_tag(tag) {
                worklog.add_record(record.clone());
            }
        }

        worklog
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate;
    }

    pub fn tags(&self) -> &HashSet<String> {
        &self.tags
    }

    pub fn add_record(&mut self, record: WorklogRecord) {
        self.begin_date = record.begin_date().min(self.begin_date);
        self.end_date = record.end_date().max(self.end_date);
        self.tags.extend(record.tags());

        self.records.push(record);
    }

    pub fn append(&mut self, worklog: &Self) {
        for record in worklog.records() {
            self.add_record(record.clone());
        }
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

    pub fn len(&self) -> usize {
        self.records.len()
    }
}

impl Default for Worklog {
    fn default() -> Self {
        Worklog::new()
    }
}