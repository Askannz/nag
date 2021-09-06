use chrono::{Duration, TimeZone};
use super::{Cronline, parse_cronline};

#[test]
fn fixed_durations() {

    let now = chrono::Local.ymd(2000, 01, 01).and_hms(08, 00, 00);

    test_parse(TestParams {
        now,
        msg: "in 2 min test1 test2",
        exp_cronline: Cronline::from_time(&(now + Duration::minutes(2))),
        exp_rem_words: &["test1", "test2"]
    });

    test_parse(TestParams {
        now,
        msg: "in 3h test1 test2",
        exp_cronline: Cronline::from_time(&(now + Duration::hours(3))),
        exp_rem_words: &["test1", "test2"]
    });
}

#[test]
fn weekdays() {

    // That date is a Saturday
    let now = chrono::Local.ymd(2000, 01, 01).and_hms(08, 00, 00);

    test_parse(TestParams {
        now,
        msg: "on saturday at 8am test1 test2",
        exp_cronline: Cronline::from_time(&(now + Duration::weeks(1))),
        exp_rem_words: &["test1", "test2"]
    });

    test_parse(TestParams {
        now,
        msg: "on sunday at 8am test1 test2",
        exp_cronline: Cronline::from_time(&(now + Duration::days(1))),
        exp_rem_words: &["test1", "test2"]
    });
}

#[test]
fn today_tomorrow() {

    let date_now = chrono::Local.ymd(2000, 01, 01);
    let now = date_now.and_hms(08, 00, 00);
    let t1 = date_now.and_hms(22, 00, 00);

    test_parse(TestParams {
        now,
        msg: "today at 10pm test1 test2",
        exp_cronline: Cronline::from_time(&t1),
        exp_rem_words: &["test1", "test2"]
    });

    let t2 = (date_now + Duration::days(1)).and_hms(22, 00, 00);

    test_parse(TestParams {
        now,
        msg: "tomorrow at 10pm test1 test2",
        exp_cronline: Cronline::from_time(&t2),
        exp_rem_words: &["test1", "test2"]
    });
}

#[test]
fn single_digits() {

    let date_now = chrono::Local.ymd(2000, 01, 01);
    let now = date_now.and_hms(08, 00, 00);
    let t1 = date_now.and_hms(9, 00, 00);

    test_parse(TestParams {
        now,
        msg: "at 9 test1 test2",
        exp_cronline: Cronline::from_time(&t1),
        exp_rem_words: &["test1", "test2"]
    });

    let t2 = chrono::Local.ymd(2000, 01, 09).and_hms(11, 00, 00);

    test_parse(TestParams {
        now,
        msg: "on the 9 at 11am test1 test2",
        exp_cronline: Cronline::from_time(&t2),
        exp_rem_words: &["test1", "test2"]
    });
}

#[test]
#[should_panic]
fn fail_every() {

    let now = chrono::Local.ymd(2000, 01, 01).and_hms(08, 00, 00);
    let msg = "every year at 7am";

    let words: Vec<&str> = msg.split_whitespace().collect();
    parse_cronline(&now, &words).unwrap();

}

struct TestParams<'a> {

    // Inputs
    now: chrono::DateTime<chrono::Local>,
    msg: &'a str,

    // Expected outputs
    exp_cronline: Cronline,
    exp_rem_words: &'a[&'a str] 
}

#[cfg(test)]
fn test_parse(params: TestParams) {

    let words: Vec<&str> = params.msg.split_whitespace().collect();
    let res = parse_cronline(&params.now, &words).unwrap();

    assert_eq!(res.cronline, params.exp_cronline);
    assert_eq!(res.remaining_words, params.exp_rem_words);
}