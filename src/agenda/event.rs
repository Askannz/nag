use chrono::{DateTime, Datelike, Timelike};
use chrono::offset::{TimeZone, LocalResult};
use serde::{Deserialize, Serialize};
use super::cron::{Cronline, CronValue, CronColumn};

#[derive(Clone, Serialize, Deserialize)]
pub struct AgendaEvent {
    pub cronline: Cronline,
    pub text: String,
    pub tag: Option<String>
}

type Instant = DateTime<chrono::Local>;


// TODO: move all that stuff to Cronline
impl AgendaEvent {

    pub fn check_fires(&self, now: &Instant) -> bool {

        [
            now.year() as u64,
            now.month() as u64,
            now.day() as u64,
            now.hour() as u64,
            now.minute() as u64
        ]
        .iter()
        .zip(CRON_COLUMNS.iter())
        .all(|(val, col)| match self.cronline.get(*col) {
            CronValue::Every => true,
            CronValue::On(cron_val) => *val == cron_val
        })
    }

    pub fn get_next_occurence(&self, now: &Instant) -> Option<Instant> {

        fn recursion_func(now: &Instant, event: &AgendaEvent, acc: &[u64]) -> Option<Instant> {

            let level = acc.len();
    
            if level < 5 {
    
                let col = CRON_COLUMNS[level];

                let vals_to_try = match event.cronline.get(col) {
                    CronValue::On(val) => vec![val],
                    CronValue::Every => {
                        let (vmin, vmax) = get_search_range(now, &col);
                        (vmin..=vmax).collect()
                    }
                };

                vals_to_try.into_iter().find_map(|val| {
                    let new_acc = &[acc, &[val]].concat();
                    recursion_func(now, event, new_acc)
                })
    
            } else {
                try_make_instant(acc).filter(|t| t > now)
            }
        }
    
        recursion_func(now, self, &[])
    }
}

fn try_make_instant(acc: &[u64]) -> Option<Instant> {

    let (year, month, day, hour, minute, second) = (
        acc[0] as i32,
        acc[1] as u32,
        acc[2] as u32,
        acc[3] as u32,
        acc[4] as u32,
        0
    );

    let res = chrono::Local
        .ymd_opt(year, month, day)
        .and_hms_opt(hour, minute, second);

    match res {
        LocalResult::Single(t) => Some(t),
        _ => None
    }
}

fn get_search_range(now: &Instant, col: &CronColumn) -> (u64, u64) {
    let curr_year = now.year() as u64;
    match col {
        CronColumn::Year => (curr_year, curr_year+4),
        CronColumn::Month => (0, 12),
        CronColumn::Day => (0, 31),
        CronColumn::Hour => (0, 24),
        CronColumn::Minute => (0, 60)
    }
}

const CRON_COLUMNS: [CronColumn; 5] = [
    CronColumn::Year,
    CronColumn::Month,
    CronColumn::Day,
    CronColumn::Hour,
    CronColumn::Minute
];
