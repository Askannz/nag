use std::collections::HashMap;
use std::convert::TryInto;
use log::debug;
use regex::Regex;
use chrono::{DateTime, Datelike, Duration, Timelike};
use super::super::cron::{CronColumn, CronValue, CRON_COLUMNS};
use super::{ParsingState, ParseUpdate};

pub(super) fn parse<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> where 'a: 'b {

    let parsers: Vec<&ParserFunc> = vec![
        &try_parse_day,
        &try_parse_month,
        &try_parse_clocktime,
        &try_parse_duration,
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

fn try_parse_day<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    let (&word, remaining_words, has_prep) = match state.remaining_words {
        ["on", "the", word, rem_words @ ..] => (word, rem_words, true),
        ["the", word, rem_words @ ..] => (word, rem_words, true),
        [word, rem_words @ ..] => (word, rem_words, false),
        _ => return  None
    };

    let reg = Regex::new(r"^([0-9]{1,2})((st)|(nd)|(rd)|(th))?$").unwrap();
    let word = word.to_lowercase();
    let captures = reg.captures(word.as_str())?;

    let day: u64 = captures
        .get(1)?
        .as_str()
        .parse()
        .ok()?;

    let has_suffix = captures.get(2).is_some();

    // Make sure we have at least some indication that the
    // numbers represents a day
    if !(has_prep || has_suffix) {
        return None;
    }

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

    let (word, remaining_words) = match state.remaining_words {
        ["in", word, rem_words @ ..] => (word, rem_words),
        ["on", word, rem_words @ ..] => (word, rem_words),
        [word, rem_words @ ..] => (word, rem_words),
        _ => return  None
    };

    let month = {
        let month_0 = MONTHS.iter().position(|m| m == word)?;
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

    let (time_word, mut remaining_words, has_prep) = match state.remaining_words {
        ["at", time_word, rem_words @ ..] => (*time_word, rem_words, true),
        [time_word, rem_words @ ..] => (*time_word, rem_words, false),
        _ => return  None
    };

    let has_colon = time_word.contains(":");

    let time_reg = Regex::new(r"^([0-9]{1,2})(:([0-9]{1,2}))?([ap]m)?$").unwrap();
    let time_word = time_word.to_lowercase();
    let captures = time_reg.captures(time_word.as_str())?;

    let am_pm = captures.get(4)
        .map(|am_pm| am_pm.as_str().to_lowercase())
        .or_else(|| { // check for am/pm in the next word
            let mut res = None;
            if let Some((&w, new_rem)) = remaining_words.split_first() {
                let w = w.to_lowercase();
                if w == "am" || w == "pm" {
                    res = Some(w);
                    remaining_words = new_rem;
                }
            }
            res
        });

    let has_am_pm = am_pm.is_some();

    let hour: u64 = {

        let raw_val: u64 = captures.get(1)?
            .as_str()
            .parse()
            .ok()?;

        match am_pm.as_deref() {
            Some("am") if raw_val <= 12  => Some(raw_val % 12),
            Some("pm") if raw_val <= 12  => Some((raw_val % 12) + 12),
            _ if raw_val < 24            => Some(raw_val),
            _                            => None
        }?
    };

    let minute: u64 = captures
        .get(3)
        .map(|s| s.as_str().parse().ok())
        .flatten()
        .unwrap_or(0);

    // Make sure we have at least some indication that the
    // numbers represents a time of day
    if !(has_prep || has_colon || has_am_pm) {
        return None;
    }

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

fn try_parse_duration<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    let (word, mut remaining_words) = match state.remaining_words {
        ["in", word, rem_words @ ..] => (word, rem_words),
        _ => return None
    };

    let reg = Regex::new(r"^[0-9]+").unwrap();
    let reg_match = reg.find(word)?;

    let value: i64 = reg_match.as_str().parse().ok()?;

    let durations: Vec<_> = vec![
        (r"m(in(utes?)?)?", Duration::minutes(value)),
        (r"h(ours?)?", Duration::hours(value)),
        (r"d(ays?)?", Duration::days(value)),
        (r"w(eeks?)?", Duration::weeks(value)),
    ]
    .into_iter()
    .map(|(reg, dur)| (Regex::new(reg).unwrap(), dur))
    .collect();

    let suffix = &word[reg_match.end()..].to_lowercase();

    let get_duration = |text| {
        durations
            .iter()
            .find_map(|(reg, dur)| {
                if reg.is_match(text) { Some(dur) }
                else { None }
            })
    };

    let &duration = {
        let mut res = get_duration(suffix);
        if res.is_none() {
            let (word, rem_words) = remaining_words.split_first()?;
            remaining_words = rem_words;
            res = get_duration(word);
        }
        res
    }?;

    let time = state.now + duration;
    let cron_updates = time_to_cron(time).into_iter().collect();

    let update = ParseUpdate {
        cron_updates,
        remaining_words
    };

    debug!("Parsed: duration");

    Some(update)
}

fn try_parse_year<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    let (word, remaining_words) = match state.remaining_words {
        ["in", word, rem_words @ ..] => (word, rem_words),
        [word, rem_words @ ..] => (word, rem_words),
        _ => return None
    };

    let reg = Regex::new(r"^[0-9]{4}$").unwrap();

    let year: u64 = reg.captures(word)?
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

    let remaining_words = match state.remaining_words {
        ["on", rem_words @ ..] => rem_words,
        [rem_words @ ..] => rem_words
    };

    let (&w1, remaining_words) = remaining_words.split_first()?;

    if w1 != "every" {
        return None
    }

    let (&w2, remaining_words) = remaining_words.split_first()?;

    let &cron_col = CRON_COLUMNS
        .iter()
        .find(|col| col.unit() == w2)?;

    let update = ParseUpdate {
        cron_updates: vec![(cron_col, CronValue::Every)],
        remaining_words
    };

    debug!("Parsed: \"every\"");

    Some(update)
}



fn try_parse_date_digits<'a, 'b>(state: &'b ParsingState<'a>) -> Option<ParseUpdate<'a>> {

    let (word, remaining_words) = match state.remaining_words {
        ["on", "the", word, rem_words @ ..] => (word, rem_words),
        ["on", word, rem_words @ ..] => (word, rem_words),
        [word, rem_words @ ..] => (word, rem_words),
        _ => return None
    };

    let reg = Regex::new(r"^([0-9]{1,2})/([0-9]{1,2})(/([0-9]{4}))?$").unwrap();
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

    let (word, remaining_words) = match state.remaining_words {
        ["on", word, rem_words @ ..] => (word, rem_words),
        [word, rem_words @ ..] => (word, rem_words),
        _ => return None
    };

    let event_offset: u32 = DAYS
        .iter().position(|d| d.to_lowercase() == word.to_lowercase())?
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
