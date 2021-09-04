use std::collections::HashMap;
use std::convert::TryInto;
use log::debug;
use regex::Regex;
use chrono::{DateTime, Datelike, Duration, Timelike};
use super::super::cron::{CronColumn, CronValue, CRON_COLUMNS};
use super::{ParsingState, ParseUpdate};

pub(super) fn parse<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> where 'a: 'b {

    let parsers: Vec<&ParserFunc> = vec![
        &try_parse_preposition,
        &try_parse_day,
        &try_parse_month,
        &try_parse_clocktime,
        &try_parse_year,
        &try_parse_every,
        &try_parse_date_digits,
        &try_parse_relative,
        &try_parse_weekday,
    ];

    parsers
        .iter()
        .find_map(|func| func(state))
}


type ParserFunc<'a, 'b> = dyn Fn(&'b ParsingState<'a>) -> Option<ParseUpdate<'a>>;


fn try_parse_preposition<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    let (first_word, remaining_words) = state.remaining_words.split_first()?;

    let update = match *first_word {
        "at" | "in" | "on" => ParseUpdate {
            cron_updates: vec![],
            remaining_words
        },
        _ => return None
    };

    debug!("Parsed: preposition");
    Some(update)
}

fn try_parse_day<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    let reg = Regex::new(r"^([0-9]{1,2})((st)|(nd)|(rd)|(th))?$").unwrap();

    let (word, remaining_words) = {
        let (first_word, remaining_words) = state.remaining_words.split_first()?;
        match *first_word {
            "the" => remaining_words.split_first()?,
            _ => (first_word, remaining_words)
        }
    };

    let day: u64 = reg
        .captures(word)?
        .get(1)?
        .as_str()
        .parse()
        .ok()?;

    let update = ParseUpdate {
        cron_updates: vec![(CronColumn::Day, CronValue::On(day))],
        remaining_words
    };

    debug!("Parsed: day");

    Some(update)
}

fn try_parse_month<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

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

    let (first_word, remaining_words) = state.remaining_words.split_first()?;

    let month = {
        let month_0 = MONTHS.iter().position(|m| m == first_word)?;
        month_0 as u64 + 1
    };

    let update = ParseUpdate {
        cron_updates: vec![(CronColumn::Month, CronValue::On(month))],
        remaining_words
    };

    debug!("Parsed: month");

    Some(update)
}

fn try_parse_clocktime<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    let reg = Regex::new(r"^([0-9]{1,2})(:([0-9]{1,2}))?([ap]m)?$").unwrap();

    let (first_word, remaining_words) = state.remaining_words.split_first()?;
    let captures = reg.captures(first_word)?;

    let hour: u64 = {

        let raw_val: u64 = captures.get(1)?
            .as_str()
            .parse()
            .ok()?;

        let am_pm_str = captures.get(4)?.as_str();

        match am_pm_str {
            "am" if raw_val <= 12  => Some(raw_val % 12),
            "pm" if raw_val <= 12  => Some((raw_val % 12) + 12),
            _ if raw_val < 24      => Some(raw_val),
            _                       => None
        }?
    };

    let minute: u64 = captures
        .get(3)
        .map(|s| s.as_str().parse().ok())
        .flatten()
        .unwrap_or(0);

    let update = ParseUpdate {
        cron_updates: vec![
            (CronColumn::Hour, CronValue::On(hour)),
            (CronColumn::Minute, CronValue::On(minute))
        ],
        remaining_words
    };

    debug!("Parsed: clock time");

    Some(update)
}

fn try_parse_year<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    let reg = Regex::new(r"^[0-9]{4}$").unwrap();

    let (first_word, remaining_words) = state.remaining_words.split_first()?;

    let year: u64 = reg.captures(first_word)?
        .get(0)?
        .as_str()
        .parse()
        .ok()?;

    let update = ParseUpdate {
        cron_updates: vec![(CronColumn::Year, CronValue::On(year))],
        remaining_words
    };

    debug!("Parsed: year");

    Some(update)
}

fn try_parse_every<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    let (w1, remaining_words) = state.remaining_words.split_first()?;

    if *w1 != "every" {
        return None
    }

    let (w2, remaining_words) = remaining_words.split_first()?;

    let cron_col = CRON_COLUMNS
        .iter()
        .find(|col| col.unit() == *w2)?;

    let update = ParseUpdate {
        cron_updates: vec![(*cron_col, CronValue::Every)],
        remaining_words
    };

    debug!("Parsed: \"every\"");

    Some(update)
}



fn try_parse_date_digits<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    let reg = Regex::new(r"^([0-9]{1,2})/([0-9]{1,2})(/([0-9]{4}))?$").unwrap();

    let (word, remaining_words) = state.remaining_words.split_first()?;
    let captures = reg.captures(word)?;

    let day: u64 = captures.get(1)?.as_str().parse().ok()?;
    let month: u64 = captures.get(2)?.as_str().parse().ok()?;

    let update = ParseUpdate {
        cron_updates: vec![
            (CronColumn::Day, CronValue::On(day)),
            (CronColumn::Month, CronValue::On(month))
        ],
        remaining_words
    };

    debug!("Parsed: date digits");

    Some(update)
}


fn try_parse_relative<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    let date_now = state.now.date();

    let (word, remaining_words) = state.remaining_words.split_first()?;

    let date = match *word {
        "today" => Some(date_now),
        "tomorrow" => Some(date_now + Duration::days(1)),
        _ => None
    }?;

    let (day, month, year) = (
        date.day().try_into().unwrap(),
        date.month().try_into().unwrap(),
        date.year().try_into().unwrap()
    );

    let update = ParseUpdate {
        cron_updates: vec![
            (CronColumn::Day, CronValue::On(day)),
            (CronColumn::Month, CronValue::On(month)),
            (CronColumn::Year, CronValue::On(year))
        ],
        remaining_words
    };

    debug!("Parsed: relative");

    Some(update)
}

fn try_parse_weekday<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    const DAYS: [&str; 7] = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday"
    ];

    let (first_word, remaining_words) = state.remaining_words.split_first()?;

    let event_offset: u32 = DAYS
        .iter().position(|d| d.to_lowercase() == first_word.to_lowercase())?
        .try_into().unwrap();
    let current_offset = state.now.date().weekday().num_days_from_monday();

    let nb_days = if current_offset < event_offset {
        event_offset - current_offset
    } else {
        7 - (current_offset - event_offset)
    };

    let time = state.now + Duration::days(nb_days.into());
    let cron_vals = time_to_cron(time);

    let update = ParseUpdate {
        cron_updates: vec![(CronColumn::Day, cron_vals[&CronColumn::Day])],
        remaining_words
    };

    debug!("Parsed: weekday");

    Some(update)
}

fn time_to_cron(time: DateTime<chrono::Local>) -> HashMap<CronColumn, CronValue> {

    let (minute, hour, day, month, year) = (
        time.minute().try_into().unwrap(),
        time.hour().try_into().unwrap(),
        time.date().day().try_into().unwrap(),
        time.date().month().try_into().unwrap(),
        time.date().year().try_into().unwrap()
    );

    vec![
        (CronColumn::Minute, CronValue::On(minute)),
        (CronColumn::Hour, CronValue::On(hour)),
        (CronColumn::Day, CronValue::On(day)),
        (CronColumn::Month, CronValue::On(month)),
        (CronColumn::Year, CronValue::On(year)),
    ]
    .into_iter()
    .collect()
}
