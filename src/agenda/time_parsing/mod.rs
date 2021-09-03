use chrono::DateTime;
use anyhow::Result;

mod cronline_builder;
mod parsers;

use super::Instant;
use super::cron::{CronColumn, CronValue, Cronline};
use cronline_builder::CronlineBuilder;
use parsers::{get_parsers, ParseUpdate};

pub fn parse_cronline_spec(now: &DateTime<chrono::Local>, words: &[&str]) -> Result<(Cronline, Vec<String>)> {

    let parser_funcs = get_parsers();

    let words: Vec<String> = words
        .iter()
        .map(|s| (*s).to_owned())
        .collect();

    let mut state = ParsingState::new(words, now.clone());

    loop {

        let parse_update = parser_funcs
            .iter()
            .find_map(|func| func(&state));

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

pub struct ParsingState {
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
