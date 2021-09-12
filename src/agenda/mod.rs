use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use crossbeam_channel::Sender;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Context, bail};
use chrono::Timelike;
use log::{debug, info, warn};

use crate::{Opts, BotUpdate};

mod cron;
mod time_parsing;
mod event;

use time_parsing::{parse_cronline, CronlineResult};
use event::AgendaEvent;

pub(super) struct Agenda {
    state: Arc<Mutex<AgendaState>>,
    sender: Sender<BotUpdate>,
    state_path: PathBuf,
    opts: Opts
}

type Instant = chrono::DateTime<chrono::offset::Local>;

impl Agenda {

    pub(super) fn new(opts: &Opts, sender: &Sender<BotUpdate>) -> Self {

        let state_path = opts.data_path.join("agenda.json");
        debug!("Agenda state path: {}", state_path.to_string_lossy());

        let state = AgendaState::restore(&state_path)
            .unwrap_or_else(|err| {
                warn!("No agenda state restored: {}", err);
                info!("Creating new empty agenda");
                AgendaState::new()
            });
        let state = Arc::new(Mutex::new(state));
    
        Agenda { 
            state,
            sender: sender.clone(),
            state_path,
            opts: opts.clone()
        }
    }

    pub(super) fn get_loop(&self) -> impl FnOnce() {

        const INTERVAL: Duration = Duration::from_millis(500);

        let state = self.state.clone();
        let state_path = self.state_path.clone();
        let sender = self.sender.clone();

        move || {

            let mut prev_t = chrono::Local::now();

            info!("Starting agenda event loop");
        
            loop {
        
                let curr_t = chrono::Local::now();
        
                if curr_t.minute() != prev_t.minute() {
        
                    let mut state = state.lock().unwrap();
        
                    let keys_list: Vec<u64> = state.events
                        .keys()
                        .cloned()
                        .collect();
        
                    let mut state_changed = false;
                    for id in keys_list {
                        let event = &state.events[&id];
                        if event.check_fires(&curr_t) {

                            info!("It's {}, firing event {}", curr_t, id);
        
                            let notification = format!("⏰ {}", event.text);
                            sender.send(BotUpdate::MsgOut(notification)).unwrap();
        
                            if event.get_next_occurence(&curr_t).is_none() {
                                info!("Event {} never occurs again, removing", id);
                                state.events.remove(&id);
                                state_changed = true;
                            }
                        }
                    }
        
                    if state_changed {
                        state.save(&state_path);
                    }
        
                    prev_t = curr_t;
                }
        
                std::thread::sleep(INTERVAL);
            }
        }
    }

    pub(super) fn process(&mut self, msg: &str) {

        let words: Vec<&str> = msg.split_whitespace().collect();

        debug!("words {:?}", words);

        let command_res = || -> Option<(&str, &[&str])> {
            let (&w, rew_words) = words.split_first()?;
            let c = w.chars().nth(0)?;
            match c {
                '/' => Some((w, rew_words)),
                _ => None
            }
        }();

        let msg = match command_res {

            Some((w, rem_words)) => match (w, rem_words) {
                ("/help", _)     => self.print_help(),
                ("/events", [])   => self.print_events(),
                ("/events", args) => self.print_tagged_events(args),
                ("/del", args)   => self.remove_events(args),
                ("/tag", args)   => self.tag_event(args),
                ("/untag", args) => self.untag_event(args),
                _                => Ok("Unknown command".into())
            },

            None => self.add_event(&words)
        }
        .unwrap_or_else(format_error);

        self.sender.send(BotUpdate::MsgOut(msg)).unwrap();
    }

    fn add_event(&self, words: &[&str]) -> anyhow::Result<String> {

        info!("Adding new event");

        let now = chrono::Local::now();

        debug!("Time now is {}", now);

        let CronlineResult {
            cronline,
            remaining_words,
            comment
        } = parse_cronline(&self.opts, &now, words)
            .context("cannot parse time")?;

        debug!("Parsed cronline {:?}", cronline);
        debug!("Remaining words {:?}", remaining_words);

        if remaining_words.is_empty() {
            bail!("no message specified")
        }

        let mut state = self.state.lock().unwrap();

        let new_id = (0..).find(|id| !state.events.contains_key(&id)).unwrap();

        debug!("New event ID {}", new_id);

        let agenda_event = AgendaEvent {
            text: remaining_words.join(" "),
            cronline,
            tag: None
        };

        let now = chrono::Local::now();
        let occ_t = agenda_event.get_next_occurence(&now)
            .ok_or(anyhow!("Invalid time: never occurs"))?;

        debug!("Event occurs at {}", occ_t);

        state.events.insert(new_id, agenda_event);
        state.save(&self.state_path);

        let occ_text = format_time_diff(occ_t - now);

        let text = {
            let mut text = String::new();
            if let Some(comment) = comment {
                text = format!("{}\n", comment);
            }
            format!(
                "{}New event added (number {}).\nNext occurence in {}.",
                text, new_id, occ_text
            )
        };


        Ok(text)
    }

    fn remove_events(&self, words: &[&str]) -> anyhow::Result<String> {

        if words.is_empty() {
            bail!("No event number supplied");
        }

        let event_ids = words.iter()
            .map(|w| w.parse::<u64>().context("Invalid event number"))
            .collect::<anyhow::Result<Vec<u64>>>()?;

        info!("Removing events {:?}", event_ids);

        let mut state = self.state.lock().unwrap();

        let out_lines = event_ids.iter().map(
            |ev_id| match state.events.remove(&ev_id) {
                Some(event) => format!("Removed event \"{}\"", event.text),
                None => format!("Error: no event at number \"{}\"", ev_id)
            })
            .collect::<Vec<String>>();

        state.save(&self.state_path);

        Ok(out_lines.join("\n"))
    }

    fn tag_event(&self, words: &[&str]) -> anyhow::Result<String> {

        let (id_str, tag_words) = match words {
            []                       => Err(anyhow!("No arguments specified")),
            [_]                      => Err(anyhow!("No tag specified")),
            [id_str, tag_words @ ..] => Ok((id_str, tag_words))
        }?;

        let id: u64 = id_str
            .parse()
            .context("Invalid event number")?;

        let mut state = self.state.lock().unwrap();

        let event = state.events.get_mut(&id)
            .ok_or(anyhow!("No event at this number"))?;

        info!("Tagging event {}", id);

        let tag = tag_words.join(" ");
        let out_str = format!("Tagged event \"{}\" with \"{}\"", event.text, tag);
        event.tag = Some(tag);

        state.save(&self.state_path);

        Ok(out_str)
    }

    fn untag_event(&self, words: &[&str]) -> anyhow::Result<String> {

        let id: u64 = words.get(0)
            .ok_or(anyhow!("No event number supplied"))?
            .parse()
            .context("Invalid event number")?;

        info!("Untagging event {}", id);

        let mut state = self.state.lock().unwrap();

        let event = state.events.get_mut(&id)
            .ok_or(anyhow!("No event at this number"))?;

        event.tag = None;

        state.save(&self.state_path);

        Ok("Untagged event".to_string())
    }

    fn print_events(&self) -> anyhow::Result<String> {

        info!("Printing events");

        let state = self.state.lock().unwrap();

        if state.events.is_empty() {
            return Ok("No events".to_owned())
        }

        let untagged_events: HashMap<&u64, &AgendaEvent> = state.events
            .iter()
            .filter(|(_id, event)| event.tag.is_none())
            .collect();

        let msg = [
            make_tags_print_list(&state.events),
            vec!["\n<b>Untagged events:</b>".to_owned()],
            make_events_print_list(&self.opts, untagged_events)
        ]
        .concat().join("\n");

        Ok(msg)
    }

    fn print_tagged_events(&self, words: &[&str]) -> anyhow::Result<String> {

        if words.is_empty() {
            return Err(anyhow!("No tag supplied"))
        }

        info!("Printing tagged events");

        let tag = words.join(" ");

        let state = self.state.lock().unwrap();
        let selected_events:HashMap<&u64, &AgendaEvent> = state.events
            .iter()
            .filter(|(_id, event)| match &event.tag {
                None => false,
                Some(event_tag) => *event_tag == tag
            }).collect();

        if selected_events.is_empty() {
            return Err(anyhow!("No events with tag \"{}\"", tag))
        }

        let msg = [
            vec![format!("<b>Tag</b> <code>{}</code>:", tag)],
            make_events_print_list(&self.opts, selected_events)
        ]
        .concat().join("\n");

        Ok(msg)
    }

    fn print_help(&self) -> anyhow::Result<String> {

        info!("Printing help");

        let commands = [
            (
                "/help",
                "Show this message"
            ),
            (
                "/events",
                "Lists upcoming events"
            ),
            (
                "/events &lt;tag&gt",
                "Lists upcoming events tagged with &lt;tag&gt;"
            ),
            (
                "/del  &lt;n&gt;",
                "Delete event number &lt;n&gt;"
            ),
            (
                "/tag &lt;n&gt; &lt;tag&gt;",
                "Tag event number &lt;n&gt; with tag &lt;tag&gt;"
            ),
            (
                "/untag &lt;n&gt;",
                "Untag event number &lt;n&gt;"
            )
        ];

        let commands_msg = commands.iter()
            .map(|(cmd, txt)| {
                format!("<b>{}</b>\n    {}", cmd, txt)
            })
            .collect::<Vec<String>>()
            .join("\n");

        let agenda_msg = "\
            To add a new event, send \
            <code>&lt;time&gt; &lt;message&gt;</code> \
            without a leading <b>/</b>.";

        let examples = [
            "at 9am pick up groceries",
            "every year on April 4th Dan's birthday",
            "on 21/08 dentist appointment",
            "in 30 minutes check cake in oven",
            "every day at 5pm do some exercise"
        ];

        let examples_msg = examples.iter()
            .map(|txt| { format!("    {}", txt) })
            .collect::<Vec<String>>()
            .join("\n");


        let msg = format!(
            "{}\n\n{}\nExamples:\n\n<code>{}</code>",
            commands_msg, agenda_msg, examples_msg);

        Ok(msg)
    }
}

fn format_error(err: anyhow::Error) -> String {

    let mut msg = String::new();
    for (i, err2) in err.chain().enumerate() {
        let prefix = if i == 0 { "" } else { ": " };
        msg = format!("{}{}{}", msg, prefix, err2);
    }

    format!("Error: {}", msg)
}

fn make_tags_print_list(events: &HashMap<u64, AgendaEvent>) -> Vec<String> {

    let mut tags_count = HashMap::<&String, usize>::new();

    events
        .values()
        .for_each(|event| if let Some(ref tag) = event.tag {
            let counter = tags_count.entry(tag).or_insert(0);
            *counter += 1;
        });

    tags_count.iter()
        .map(|(tag, count)| format!("<b>Tag</b> <code>{}</code>: {} events", tag, count))
        .collect()
}


fn make_events_print_list(
    opts: &Opts, events: HashMap<&u64, &AgendaEvent>
) -> Vec<String> {

    let mut ids_list: Vec<&u64> = events.keys().cloned().collect();
    ids_list.sort();
    
    ids_list.iter().map(|id| {

        let event = &events[id];
        
        format!(
            "<pre>  {} - [{}] {}</pre>",
            event.cronline.msg_format(opts),
            id,
            event.text
        )
    }).collect()
}

fn format_time_diff(dt: chrono::Duration) -> String {

    let (weeks, days, hours, minutes, seconds) = (
        dt.num_weeks(),
        dt.num_days(),
        dt.num_hours(),
        dt.num_minutes(),
        dt.num_seconds()
    );

    let mut text = vec![];
    if weeks > 0 { text.push(format!("{} weeks", weeks)); }
    if days > 0 { text.push(format!("{} days", days % 7)); }
    if hours > 0 { text.push(format!("{} hours", hours % 24)); }
    if minutes > 0 {
        let mut minutes = minutes;
        if seconds >= 30 { minutes += 1; }
        text.push(format!("{} minutes", minutes % 60));
    } else {
        text.push("less than a minute".to_owned());
    }

    text.join(" ")
}


#[derive(Clone, Serialize, Deserialize)]
struct AgendaState {
    events: HashMap<u64, AgendaEvent>
}


impl AgendaState {

    fn restore(state_path: &Path) -> anyhow::Result<Self> {

        let path_str = state_path.to_string_lossy();

        info!("Attempting to restore agenda from {}", path_str);
        let data = std::fs::read_to_string(&state_path)?;
        let state: Self = serde_json::from_str(&data)
            .map_err(anyhow::Error::new)
            .expect(&format!("Error parsing agenda data from {}", path_str));
        Ok(state)
    }

    fn new() -> Self {
        AgendaState { events: HashMap::new() }
    }

    fn save(&self, state_path: &Path) {

        let path_str = state_path.to_string_lossy();

        || -> anyhow::Result<()> {
            info!("Saving agenda to: {}", path_str);
            let data = serde_json::to_string_pretty(self)?;
            std::fs::write(&state_path, data)?;
            Ok(())
        }()
        .expect(&format!("Cannot save agenda data to {}", path_str));
    }
}
