use serde::{Deserialize, Serialize};
use chrono::{Datelike, Timelike};
use crate::{Opts, DateFormat};
use super::Instant;


pub const CRON_COLUMNS: [CronColumn; 5] = [
    CronColumn::Minute,
    CronColumn::Hour,
    CronColumn::Day,
    CronColumn::Month,
    CronColumn::Year
];


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[repr(usize)]
pub enum CronColumn {
    Minute = 0,
    Hour = 1,
    Day = 2,
    Month = 3,
    Year = 4
}

impl CronColumn {

    pub fn rank(&self) -> usize {
        *self as usize
    }

    pub fn unit(&self) -> &str {
        match self {
            CronColumn::Year   => "year",
            CronColumn::Month  => "month",
            CronColumn::Day    => "day",
            CronColumn::Hour   => "hour",
            CronColumn::Minute => "minute"
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

    pub fn msg_format(&self, opts: &Opts) -> String {

        let format_val = |cronval, width| match cronval {
            CronValue::Every => "_".repeat(width),
            CronValue::On(val) => format!("{:0>1$}", val, width)
        };

        let (c1, c2) = match opts.date_format {
            DateFormat::DMY => (CronColumn::Day, CronColumn::Month),
            DateFormat::MDY => (CronColumn::Month, CronColumn::Day),
        };

        format!(
            "{}/{}/{} {}:{}",
            format_val(self.get(c1), 2),
            format_val(self.get(c2), 2),
            format_val(self.get(CronColumn::Year), 4),
            format_val(self.get(CronColumn::Hour), 2),
            format_val(self.get(CronColumn::Minute), 2)
        )
    }
}
