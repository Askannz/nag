use chrono::DateTime;
use anyhow::Result;

mod cronline_builder;
mod parsers;

use super::Instant;
use super::cron::{CronColumn, CronValue, Cronline};
use cronline_builder::CronlineBuilder;

struct ParseUpdate { 
    cron_updates: Vec<(CronColumn, CronValue)>,
    words: Vec<String>
}

pub(super) fn parse_cronline(now: &DateTime<chrono::Local>, words: &[&str]) -> Result<(Cronline, Vec<String>)> {

    let words: Vec<String> = words
        .iter()
        .map(|s| (*s).to_owned())
        .collect();

    let mut state = ParsingState::new(words, now.clone());

    loop {

        let parse_update = parsers::parse(&state);

        if let Some(parse_update) = parse_update {

            let ParseUpdate { cron_updates, words } = parse_update;
            for (col, val) in cron_updates.into_iter() {
                state.update(col, val)?;
            }
            state.words = words;

            if state.words.is_empty() {
                break;
            }

        } else {
            break;
        }
    }

    state.finalize(&now)
}

struct ParsingState {
    words: Vec<String>,
    cronline_builder: CronlineBuilder,
    now: DateTime<chrono::Local>
}


impl ParsingState {

    fn new(words: Vec<String>, now: DateTime<chrono::Local>) -> Self {
        ParsingState {
            words,
            cronline_builder: CronlineBuilder::new(),
            now
        }
    }

    fn update(&mut self, col: CronColumn, val: CronValue) -> Result<()> {
        self.cronline_builder.set(col, val)
    }

    fn finalize(mut self, now: &Instant) -> Result<(Cronline, Vec<String>)> {

        self.cronline_builder.autofill(now);

        let cronline = self.cronline_builder.build()?;

        let remaining_words = self.words;

        Ok((cronline, remaining_words))
    }
}
