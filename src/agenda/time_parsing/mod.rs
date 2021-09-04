use chrono::DateTime;
use anyhow::Result;

mod cronline_builder;
mod parsers;

use super::Instant;
use super::cron::{CronColumn, CronValue, Cronline};
use cronline_builder::CronlineBuilder;

struct ParseUpdate<'a> { 
    cron_updates: Vec<(CronColumn, CronValue)>,
    remaining_words: &'a[&'a str]
}

pub(super) fn parse_cronline<'a>(now: &DateTime<chrono::Local>, words: &'a [&'a str]) -> Result<(Cronline, &'a [&'a str])> {

    let mut state = ParsingState::new(words, now.clone());

    loop {

        let parse_update = parsers::parse(&state);

        if let Some(parse_update) = parse_update {

            let ParseUpdate { cron_updates, remaining_words } = parse_update;
            for (col, val) in cron_updates.into_iter() {
                state.update(col, val)?;
            }
            state.remaining_words = remaining_words;

            if state.remaining_words.is_empty() {
                break;
            }

        } else {
            break;
        }
    }

    state.finalize(&now)
}

struct ParsingState<'a> {
    remaining_words: &'a[&'a str],
    cronline_builder: CronlineBuilder,
    now: DateTime<chrono::Local>
}


impl<'a> ParsingState<'a> {

    fn new(words: &'a[&'a str], now: DateTime<chrono::Local>) -> Self {
        ParsingState {
            remaining_words: words,
            cronline_builder: CronlineBuilder::new(),
            now
        }
    }

    fn update(&mut self, col: CronColumn, val: CronValue) -> Result<()> {
        self.cronline_builder.set(col, val)
    }

    fn finalize(mut self, now: &Instant) -> Result<(Cronline, &'a [&'a str])> {

        self.cronline_builder.autofill(now);

        let cronline = self.cronline_builder.build()?;

        Ok((cronline, self.remaining_words))
    }
}
