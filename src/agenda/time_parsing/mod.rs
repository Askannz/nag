use chrono::DateTime;
use anyhow::Result;

mod cronline_builder;
mod parsers;

use super::Instant;
use super::cron::{CronColumn, CronValue, Cronline};
use cronline_builder::CronlineBuilder;
use parsers::get_parsers;

pub fn parse_cronline_spec(now: &DateTime<chrono::Local>, words: &[&str]) -> Result<(Cronline, Vec<String>)> {

    let parser_funcs = get_parsers();

    let words: Vec<String> = words
        .iter()
        .map(|s| (*s).to_owned())
        .collect();

    let mut state = ParsingState::new(words);

    loop {

        match parser_funcs
            .iter()
            .find_map(|func| func(now, &mut state).transpose())
            .transpose()?
        {
            None => break,
            Some(()) if state.words.is_empty() => break,
            _ => ()
        };
    }

    state.finalize(&now)
}

pub struct ParsingState {
    words: Vec<String>,
    cronline_builder: CronlineBuilder,
}


impl ParsingState {

    fn new(words: Vec<String>) -> Self {
        ParsingState {
            words,
            cronline_builder: CronlineBuilder::new()
        }
    }

    fn update(&mut self, col: CronColumn, val: CronValue) -> Result<()> {
        self.cronline_builder.set(col, val)
    }

    fn finalize(mut self, now: &Instant) -> Result<(Cronline, Vec<String>)> {

        self.cronline_builder.autofill(now);

        let cronline = self.cronline_builder.finalize()?;

        let remaining_words = self.words;

        Ok((cronline, remaining_words))
    }
}
