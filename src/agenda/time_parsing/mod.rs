use chrono::DateTime;
use log::debug;

mod cronline_builder;
mod parsers;
#[cfg(test)]
mod tests;

use crate::Opts;
use super::Instant;
use super::cron::{CronColumn, CronValue, Cronline};
use cronline_builder::CronlineBuilder;

#[derive(Debug, PartialEq)]
pub(super) struct CronlineResult<'a> { 
    pub cronline: Cronline,
    pub remaining_words: &'a[&'a str],
    pub comment: Option<String>
}

#[derive(Debug)]
struct ParseUpdate<'a> { 
    cron_updates: Vec<(CronColumn, CronValue)>,
    remaining_words: &'a[&'a str]
}

pub(super) fn parse_cronline<'a>(
    opts: &'a Opts, now: &DateTime<chrono::Local>, words: &'a [&'a str]
) -> anyhow::Result<CronlineResult<'a>> {

    let mut state = ParsingState::new(opts, words, now.clone());

    loop {

        let parse_update = parsers::parse(&state);

        debug!("Parse update? {}", parse_update.is_some());

        if let Some(parse_update) = parse_update {

            debug!(
                "Parse update: cron={:?} rem_words={:?}",
                parse_update.cron_updates, parse_update.remaining_words
            );

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

        debug!(
            "Parse state: cron={:?} rem_words={:?}",
            state.cronline_builder.map, state.remaining_words
        );
    }

    state.finalize(&now)
}

#[derive(Debug)]
struct ParsingState<'a> {
    remaining_words: &'a[&'a str],
    cronline_builder: CronlineBuilder,
    now: DateTime<chrono::Local>,
    opts: &'a Opts
}


impl<'a> ParsingState<'a> {

    fn new(opts: &'a Opts, words: &'a[&'a str], now: DateTime<chrono::Local>) -> Self {
        ParsingState {
            remaining_words: words,
            cronline_builder: CronlineBuilder::new(),
            now,
            opts
        }
    }

    fn update(&mut self, col: CronColumn, val: CronValue) -> anyhow::Result<()> {
        self.cronline_builder.set(col, val)
    }

    fn finalize(mut self, now: &Instant) -> anyhow::Result<CronlineResult<'a>> {

        let comment = self.cronline_builder.autofill(now);

        let cronline = self.cronline_builder.build()?;

        let result = CronlineResult {
            cronline,
            remaining_words: self.remaining_words,
            comment
        };

        Ok(result)
    }
}
