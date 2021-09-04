use std::convert::TryInto;
use std::collections::{HashSet, HashMap};
use chrono::DateTime;
use anyhow::bail;
use log::debug;
use super::super::cron::{CronValue, CronColumn, Cronline, CRON_COLUMNS};


#[derive(Debug)]
pub(super) struct CronlineBuilder {
    pub(super) map: HashMap<CronColumn, CronValue>
}


impl CronlineBuilder {

    pub fn new() -> Self {
        CronlineBuilder { map: HashMap::new() }
    }

    pub fn set(&mut self, col: CronColumn, val: CronValue) -> anyhow::Result<()> {
        debug!("Setting {:?} to {:?}", col, val);
        match self.map.insert(col, val) {
            None => Ok(()),
            Some(_) => bail!("{:?} already specified", col)
        }
    }

    pub fn autofill(&mut self, now: &DateTime<chrono::offset::Local>) {

        debug!("Autofilling cronline: {:?}", self.map);
    
        // Auto-filling wildcards (a.k.a "CronValue::Every") columns
        debug!("Filling wildcards");
        let mut wildcard_fill_state = false;
        for col in CRON_COLUMNS.iter() {

            debug!("Column {:?}, wildcard={}", col, wildcard_fill_state);

            match self.map.get(&col).copied() {
    
                None if wildcard_fill_state => { self.map.insert(*col, CronValue::Every); },
                None => (),
    
                Some(CronValue::Every) => {
                    wildcard_fill_state = true;
                    self.map.insert(*col, CronValue::Every);
                },
    
                Some(CronValue::On(_val)) if wildcard_fill_state => break,
                Some(CronValue::On(val)) => { self.map.insert(*col, CronValue::On(val)); },
            };
        }
    

        // Auto-filling fixed columns
        debug!("Filling fixed columns");
        let cronline_now = Cronline::from_time(now);
        let mut fixed_fill_state = false;
        for col in CRON_COLUMNS.iter() {

            debug!("Column {:?}, fixed={}", col, fixed_fill_state);
            
            match self.map.get(&col).copied() {
    
                None if fixed_fill_state => { self.map.insert(*col, cronline_now.get(*col)); },
                None => (),
    
                Some(CronValue::Every) if fixed_fill_state => break,
                Some(CronValue::Every) => { self.map.insert(*col, CronValue::Every); },
    
                Some(CronValue::On(val)) => {
                    fixed_fill_state = true;
                    self.map.insert(*col, CronValue::On(val));
                }
            };
        }

        debug!("Filled cronline: {:?}", self.map);
    }

    pub fn build(self) -> anyhow::Result<Cronline> {

        debug!("Building cronline");

        let required_columns: HashSet<_> = CRON_COLUMNS.iter().cloned().collect();
        let cronline_columns: HashSet<_> = self.map.keys().cloned().collect();
        let cols_diff: Vec<_> = required_columns.difference(&cronline_columns).collect();

        if !cols_diff.is_empty() {
            debug!("Missing columns: {:?}", cols_diff);
            let cols_str = cols_diff
                .iter()
                .map(|c| format!("{:?}", c))
                .collect::<Vec<String>>()
                .join(", ");
            bail!("Incomplete cronline: missing {}", cols_str);
        }

        let line: [CronValue; 5] = {
            let mut vec_map = self.map.into_iter()
                .collect::<Vec<(CronColumn, CronValue)>>();
            vec_map.sort_by_key(|(c, _v)| c.rank());
            vec_map.iter()
                .map(|(_c, v)| *v)
                .collect::<Vec<CronValue>>()
                .try_into()
                .unwrap() 
                // We've already checked above that all columns are present
                // and accounted for
        };
        
        Ok(Cronline::from_values(line))
    }
}
