use chrono::{Duration, TimeZone};
use clap::Clap;
use crate::Opts;

use super::{Cronline, parse_cronline};

#[test]
fn fixed_durations() {

    let now = chrono::Local.ymd(2000, 01, 01).and_hms(08, 00, 00);

    test_parse(&TestParams::new(
        now,
        "in 2 min test1 test2",
        Cronline::from_time(&(now + Duration::minutes(2))),
        &["test1", "test2"]
    ));

    test_parse(&TestParams::new(
        now,
        "in 3h test1 test2",
        Cronline::from_time(&(now + Duration::hours(3))),
        &["test1", "test2"]
    ));
}

#[test]
fn weekdays() {

    // That date is a Saturday
    let now = chrono::Local.ymd(2000, 01, 01).and_hms(08, 00, 00);

    test_parse(&TestParams::new(
        now,
        "on saturday at 8am test1 test2",
        Cronline::from_time(&(now + Duration::weeks(1))),
        &["test1", "test2"]
    ));

    test_parse(&TestParams::new(
        now,
        "on sunday at 8am test1 test2",
        Cronline::from_time(&(now + Duration::days(1))),
        &["test1", "test2"]
    ));
}

#[test]
fn month() {

    let now = chrono::Local.ymd(2000, 01, 01).and_hms(08, 00, 00);
    let t1 = chrono::Local.ymd(2000, 09, 05).and_hms(08, 00, 00);

    test_parse(&TestParams::new(
        now,
        "in September on the 5th at 8am test1 test2",
        Cronline::from_time(&t1),
        &["test1", "test2"]
    ));
}

#[test]
fn year() {

    let now = chrono::Local.ymd(2000, 01, 01).and_hms(08, 00, 00);
    let t1 = chrono::Local.ymd(2001, 09, 05).and_hms(08, 00, 00);

    test_parse(&TestParams::new(
        now,
        "in 2001 on 05/09 at 8am test1 test2",
        Cronline::from_time(&t1),
        &["test1", "test2"]
    ));
}

#[test]
fn today_tomorrow() {

    let date_now = chrono::Local.ymd(2000, 01, 01);
    let now = date_now.and_hms(08, 00, 00);
    let t1 = date_now.and_hms(22, 00, 00);

    test_parse(&TestParams::new(
        now,
        "today at 10pm test1 test2",
        Cronline::from_time(&t1),
        &["test1", "test2"]
    ));

    let t2 = (date_now + Duration::days(1)).and_hms(22, 00, 00);

    test_parse(&TestParams::new(
        now,
        "tomorrow at 10pm test1 test2",
        Cronline::from_time(&t2),
        &["test1", "test2"]
    ));
}

#[test]
fn single_digits() {

    let date_now = chrono::Local.ymd(2000, 01, 01);
    let now = date_now.and_hms(08, 00, 00);
    let t1 = date_now.and_hms(9, 00, 00);

    test_parse(&TestParams::new(
        now,
        "at 9 test1 test2",
        Cronline::from_time(&t1),
        &["test1", "test2"]
    ));

    let t2 = chrono::Local.ymd(2000, 01, 09).and_hms(11, 00, 00);

    test_parse(&TestParams::new(
        now,
        "on the 9 at 11am test1 test2",
        Cronline::from_time(&t2),
        &["test1", "test2"]
    ));
}

#[test]
fn am_pm() {

    let date_now = chrono::Local.ymd(2000, 01, 01);
    let now = date_now.and_hms(08, 00, 00);
    let t1 = date_now.and_hms(9, 00, 00);

    test_parse(&TestParams::new(
        now,
        "at 9am test1 test2",
        Cronline::from_time(&t1),
        &["test1", "test2"]
    ));

    test_parse(&TestParams::new(
        now,
        "at 9 am test1 test2",
        Cronline::from_time(&t1),
        &["test1", "test2"]
    ));

    let t2 = date_now.and_hms(21, 00, 00);

    test_parse(&TestParams::new(
        now,
        "at 9pm test1 test2",
        Cronline::from_time(&t2),
        &["test1", "test2"]
    ));

    test_parse(&TestParams::new(
        now,
        "at 9 pm test1 test2",
        Cronline::from_time(&t2),
        &["test1", "test2"]
    ));
}


#[test]
fn date_formats() {

    let now = chrono::Local.ymd(2000, 01, 01).and_hms(08, 00, 00);

    // Unspecified date format (should default to DMY)

    let t1 = chrono::Local.ymd(2000, 08, 07).and_hms(08, 00, 00);

    let params = TestParams::new(
        now,
        "on 07/08 at 8am test1 test2",
        Cronline::from_time(&t1),
        &["test1", "test2"]
    );
    test_parse(&params);

    // DMY

    let params = params.with_args(&["--date-format", "dmy"]);
    test_parse(&params);
    
    // MDY

    let t2 = chrono::Local.ymd(2000, 07, 08).and_hms(08, 00, 00);

    let params = TestParams::new(
        now,
        "on 07/08 at 8am test1 test2",
        Cronline::from_time(&t2),
        &["test1", "test2"]
    )
    .with_args(&["--date-format", "mdy"]);

    test_parse(&params);
}


#[test]
#[should_panic]
fn fail_every() {

    let opts = Opts::parse_from(["placeholder", "placeholder"]);

    let now = chrono::Local.ymd(2000, 01, 01).and_hms(08, 00, 00);
    let msg = "every year at 7am";

    let words: Vec<&str> = msg.split_whitespace().collect();
    parse_cronline(&opts, &now, &words).unwrap();

}

#[test]
#[should_panic]
fn fail_pm() {

    let opts = Opts::parse_from(["placeholder", "placeholder"]);

    let now = chrono::Local.ymd(2000, 01, 01).and_hms(08, 00, 00);
    let msg = "at 13pm";

    let words: Vec<&str> = msg.split_whitespace().collect();
    parse_cronline(&opts, &now, &words).unwrap();
}

#[derive(Clone)]
struct TestParams<'a> {

    // Inputs
    opts: Opts,
    now: chrono::DateTime<chrono::Local>,
    msg: &'a str,

    // Expected outputs
    exp_cronline: Cronline,
    exp_rem_words: &'a[&'a str] 
}

impl<'a> TestParams<'a> {

    fn new(
        now: chrono::DateTime<chrono::Local>,
        msg: &'a str,
        exp_cronline: Cronline,
        exp_rem_words: &'a[&'a str] 
    ) -> Self {
        let opts = Opts::parse_from(["placeholder", "placeholder"]);
        TestParams {
            opts, now, msg, exp_cronline, exp_rem_words
        }
    }

    fn with_args(mut self, args: &[&str]) -> Self {
        let args = {
            let mut v = vec!["placeholder", "placeholder"];
            v.extend_from_slice(args);
            v
        };
        self.opts = Opts::parse_from(args);
        self
    }
}

#[cfg(test)]
fn test_parse(params: &TestParams) {

    let words: Vec<&str> = params.msg.split_whitespace().collect();
    let res = parse_cronline(&params.opts, &params.now, &words).unwrap();

    assert_eq!(res.cronline, params.exp_cronline);
    assert_eq!(res.remaining_words, params.exp_rem_words);
}
