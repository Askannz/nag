use std::convert::TryInto;

use anyhow::Result;
use regex::Regex;
use chrono::{Duration, Datelike};
use super::super::Instant;
use super::super::cron::{CronColumn, CronValue, CRON_COLUMNS};
use super::ParsingState;


pub type ParserFunc = dyn Fn(&Instant, &mut ParsingState) -> Result<Option<()>>;


pub fn get_parsers<'a>() -> Vec<&'a ParserFunc> {
    vec![
        &try_parse_preposition,
        &try_parse_day,
        &try_parse_month,
        &try_parse_clocktime,
        &try_parse_year,
        &try_parse_every,
        &try_parse_date_digits,
        &try_parse_relative
    ]
}



fn try_parse_preposition(_now: &Instant, state: &mut ParsingState) -> Result<Option<()>> {

    match state.words.split_first()
        .filter(|(w0, _other)| ["at", "in", "on"].contains(&w0.as_str()))
    {
        None => Ok(None),
        Some((_w0, other_w)) => {
            state.words = other_w.to_vec();
            Ok(Some(()))
        }
    }
}

fn try_parse_day(_now: &Instant, state: &mut ParsingState) -> Result<Option<()>> {

    let reg = Regex::new(r"^([0-9]{1,2})((st)|(nd)|(rd)|(th))?$").unwrap();

    let res_opt = (|| -> Option<(u64, Vec<String>)> {

        let (word, remaining_words) = match state.words.split_first()? {
            (word, remaining_words) if word == "the" => remaining_words.split_first()?,
            v => v,
        };

        let day: u64 = reg
            .captures(word)?
            .get(1)?
            .as_str()
            .parse()
            .ok()?;

        Some((day, remaining_words.to_vec()))
    })();


    let res = if let Some((day, remaining_words)) = res_opt {
        state.update(CronColumn::Day, CronValue::On(day))?;
        state.words = remaining_words.to_vec();
        Some(())
    } else {
        None
    };

    Ok(res)
}

fn try_parse_month(_now: &Instant, state: &mut ParsingState) -> Result<Option<()>> {

    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December"
    ];

    let res_opt = state.words.split_first()
        .map(|(first_w, other_w)| {
            MONTHS
                .iter()
                .position(|&m| m == first_w)
                .map(|index| (1 + index as u64, other_w.to_vec()))
        })
        .flatten();

    match res_opt {
        None => Ok(None),
        Some((month, other_w)) => {
            state.update(CronColumn::Month, CronValue::On(month))?;
            state.words = other_w;
            Ok(Some(()))
        }
    }
}

fn try_parse_clocktime(_now: &Instant, state: &mut ParsingState) -> Result<Option<()>> {

    let reg = Regex::new(r"^([0-9]{1,2})(:([0-9]{1,2}))?([ap]m)?$").unwrap();

    (|| -> Option<(u64, u64)> {

        let word = state.words.get(0)?;
        let captures = reg.captures(word)?;

        let hour_val: u64 = captures.get(1)?
            .as_str()
            .parse()
            .ok()?;

        let am_pm_str = captures.get(4);

        let hour: u64 = match am_pm_str {

            None => Some(hour_val).filter(|v| *v < 24),
            
            Some(s) => Some(hour_val)
                            .filter(|v| *v <= 12)
                            .map(|hour_val| match s.as_str() {
                                "am" => hour_val % 12,
                                "pm" => (hour_val % 12) + 12,
                                _ => unreachable!()
                            })
        }?;

        let minute: u64 = captures.get(3)
            .map(|s| s.as_str().parse().ok())
            .flatten()
            .unwrap_or(0);

        Some((hour, minute))
    })()
    .map(|(hour, minute)| {

        state.update(CronColumn::Hour, CronValue::On(hour))?;
        state.update(CronColumn::Minute, CronValue::On(minute))?;
        state.words = (&state.words[1..]).to_vec();
        Ok(())
    })
    .transpose()
}

fn try_parse_year(_now: &Instant, state: &mut ParsingState) -> Result<Option<()>> {

    let reg = Regex::new(r"^[0-9]{4}$").unwrap();

    (|| -> Option<u64> {

        let word = state.words.get(0)?;

        reg.captures(word)?
            .get(0)?
            .as_str()
            .parse()
            .ok()
    })()
    .map(|year| {
        state.update(CronColumn::Year, CronValue::On(year))?;
        state.words = (&state.words[1..]).to_vec();
        Ok(())
    })
    .transpose()
}

fn try_parse_every(_now: &Instant, state: &mut ParsingState) -> Result<Option<()>> {

    (|| -> Option<&CronColumn> {

        state.words.get(0)
            .filter(|w| *w == "every")?;
        
        let w = state.words.get(1)?;

        CRON_COLUMNS
            .iter()
            .find(|col| col.unit() == w)

    })()
    .map(|col| {
        state.update(*col, CronValue::Every)?;
        state.words = (&state.words[2..]).to_vec();
        Ok(())
    })
    .transpose()
}


fn try_parse_date_digits(_now: &Instant, state: &mut ParsingState) -> Result<Option<()>> {

    let reg = Regex::new(r"^([0-9]{1,2})/([0-9]{1,2})(/([0-9]{4}))?$").unwrap();

    (|| -> Option<(u64, u64)> {

        let word = state.words.get(0)?;

        let captures = reg.captures(word)?;
        let day: u64 = captures.get(1)?.as_str().parse().ok()?;
        let month: u64 = captures.get(2)?.as_str().parse().ok()?;

        Some((day, month))
    })()
    .map(|(day, month)| {
        state.update(CronColumn::Day, CronValue::On(day))?;
        state.update(CronColumn::Month, CronValue::On(month))?;
        state.words = (&state.words[1..]).to_vec();
        Ok(())
    })
    .transpose()
}


fn try_parse_relative(now: &Instant, state: &mut ParsingState) -> Result<Option<()>> {

    let date_now = now.date();
    let date_tomorrow = date_now + Duration::days(1);

    let word = match state.words.get(0) {
        None => return Ok(None),
        Some(word) => word
    };

    match word.as_str() {
        "today" => Some(date_now),
        "tomorrow" => Some(date_tomorrow),
        _ => None
    }
    .map(|date| {

        let (day, month, year) = (
            date.day().try_into().unwrap(),
            date.month().try_into().unwrap(),
            date.year().try_into().unwrap()
        );

        state.update(CronColumn::Day, CronValue::On(day))?;
        state.update(CronColumn::Month, CronValue::On(month))?;
        state.update(CronColumn::Year, CronValue::On(year))?;
        state.words = (&state.words[1..]).to_vec();
        Ok(())
    })
    .transpose()
}
