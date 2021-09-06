use serde::{Deserialize, Serialize};
use chrono::{Datelike, Timelike};
use super::Instant;


pub const CRON_COLUMNS: [CronColumn; 5] = [
    CronColumn::Minute,
    CronColumn::Hour,
    CronColumn::Day,
    CronColumn::Month,
    CronColumn::Year
];


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum CronColumn {
    Year,
    Month,
    Day,
    Hour,
    Minute
}

impl CronColumn {

    pub fn rank(&self) -> usize {
        match self {
            CronColumn::Year   => 4,
            CronColumn::Month  => 3,
            CronColumn::Day    => 2,
            CronColumn::Hour   => 1,
            CronColumn::Minute => 0
        }
    }

    pub fn unit(&self) -> &str {
        match self {
            CronColumn::Year   => "year",
            CronColumn::Month  => "month",
            CronColumn::Day    => "day",
            CronColumn::Hour   => "minute",
            CronColumn::Minute => "hour"
        }    
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum CronValue {
    Every,
    On(u64)
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cronline {
    line: [CronValue; 5]
}

impl Cronline {

    pub fn from_values(line: [CronValue; 5]) -> Self {
        Cronline {
            line
        }
    }

    pub fn from_time(t: &Instant) -> Self {
        Cronline {
            line: [
                CronValue::On(t.minute() as u64),
                CronValue::On(t.hour() as u64),
                CronValue::On(t.day() as u64),
                CronValue::On(t.month() as u64),
                CronValue::On(t.year() as u64)
            ]
        }
    }

    pub fn get(&self, col: CronColumn) -> CronValue {
        self.line[col.rank()]
    }

    pub fn msg_format(&self) -> String {

        let format_val = |cronval, width| match cronval {
            CronValue::Every => "_".repeat(width),
            CronValue::On(val) => format!("{:0>1$}", val, width)
        };

        format!(
            "{}/{}/{} {}:{}",
            format_val(self.get(CronColumn::Day), 2),
            format_val(self.get(CronColumn::Month), 2),
            format_val(self.get(CronColumn::Year), 4),
            format_val(self.get(CronColumn::Hour), 2),
            format_val(self.get(CronColumn::Minute), 2)
        )
    }
}
