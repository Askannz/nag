use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Context, Result};
use chrono::Timelike;
use log::{debug, info, warn};

use crate::BotUpdate;

mod cron;
mod time_parsing;
mod event;

use time_parsing::parse_cronline_spec;
use event::AgendaEvent;

pub struct Agenda {
    state: Arc<Mutex<AgendaState>>,
    sender: Sender<BotUpdate>,
    state_path: PathBuf
}

type Instant = chrono::DateTime<chrono::offset::Local>;

impl Agenda {

    pub fn new(data_path: &Path, sender: &Sender<BotUpdate>) -> Self {

        let state_path = data_path.join("agenda.json");
        debug!("Agenda state path: {}", state_path.to_string_lossy());

        let state = match AgendaState::restore(&state_path) {
            Ok(context) => context,
            Err(err) => {
                warn!("No agenda state restored: {}", err);
                info!("Creating new empty agenda");
                AgendaState::new()
            }
        };
        let state = Arc::new(Mutex::new(state));
    
        Agenda { 
            state,
            sender: sender.clone(),
            state_path
        }
    }

    pub fn get_loop(&self) -> impl FnOnce() {

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
        
                            let notification = format!("⏰ {}", event.text);
                            sender.send(BotUpdate::MsgOut(notification)).unwrap();
        
                            if event.get_next_occurence(&curr_t).is_none() {
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

    pub fn process(&mut self, msg: &str) {

        let words: Vec<&str> = msg.split_whitespace().collect();

        let msg = match words.as_slice() {
            ["/help"]             => self.print_help(),
            ["/print"]             => self.print_events(),
            ["/print", args2 @ ..] => self.print_tagged_events(args2),
            ["/del", args2 @ ..]   => self.remove_events(args2),
            ["/tag", args2 @ ..]   => self.tag_event(args2),
            ["/untag", args2 @ ..] => self.untag_event(args2),
            [args2 @ ..]          => self.add_event(args2)
        }
        .unwrap_or_else(format_error);

        self.sender.send(BotUpdate::MsgOut(msg)).unwrap();
    }
}


fn format_error(err: anyhow::Error) -> String {

    let mut msg = String::new();
    for (i, err2) in err.chain().enumerate() {
        let prefix = if i == 0 { "" } else { ": " };
        msg = format!("{}{}{}", msg, prefix, err2);
    }

    msg
}

impl Agenda {

    fn add_event(&self, words: &[&str]) -> Result<String> {

        let now = chrono::Local::now();

        let (cronline, remaining_words) = parse_cronline_spec(&now, words)
            .context("Invalid command")?;

        let mut state = self.state.lock().unwrap();

        let new_id = (0..).find(|id| !state.events.contains_key(&id)).unwrap();

        let agenda_event = AgendaEvent {
            text: remaining_words.join(" "),
            cronline,
            tag: None
        };

        let now = chrono::Local::now();
        let occ_t = agenda_event.get_next_occurence(&now)
            .ok_or(anyhow!("Invalid time: never occurs"))?;

        state.events.insert(new_id, agenda_event);
        state.save(&self.state_path);

        let occ_text = format_time_diff(occ_t - now);

        Ok(format!(
            "New event added (number {}).\nNext occurence in {}.",
            new_id, occ_text))
    }

    fn remove_events(&self, words: &[&str]) -> Result<String> {

        if words.is_empty() {
            return Err(anyhow!("No event number supplied"));
        }

        let event_ids = words.iter()
            .map(|w| w.parse::<u64>().context("Invalid event number"))
            .collect::<Result<Vec<u64>>>()?;

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

    fn tag_event(&self, words: &[&str]) -> Result<String> {

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

        let tag = tag_words.join(" ");
        let out_str = format!("Tagged event \"{}\" with \"{}\"", event.text, tag);
        event.tag = Some(tag);

        state.save(&self.state_path);

        Ok(out_str)
    }

    fn untag_event(&self, words: &[&str]) -> Result<String> {

        let id: u64 = words.get(0)
            .ok_or(anyhow!("No event number supplied"))?
            .parse()
            .context("Invalid event number")?;

        let mut state = self.state.lock().unwrap();

        let event = state.events.get_mut(&id)
            .ok_or(anyhow!("No event at this number"))?;

        event.tag = None;

        state.save(&self.state_path);

        Ok("Untagged event".to_string())
    }

    fn print_events(&self) -> Result<String> {

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
            make_events_print_list(untagged_events)
        ]
        .concat().join("\n");

        Ok(msg)
    }

    fn print_tagged_events(&self, words: &[&str]) -> Result<String> {

        if words.is_empty() {
            return Err(anyhow!("No tag supplied"))
        }

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
            vec![format!("<b>{}:</b>", tag)],
            make_events_print_list(selected_events)
        ]
        .concat().join("\n");

        Ok(msg)
    }
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
        .map(|(tag, count)| format!("<b>{}</b>: {} events", tag, count))
        .collect()
}


fn make_events_print_list(events: HashMap<&u64, &AgendaEvent>) -> Vec<String> {

    let mut ids_list: Vec<&u64> = events.keys().cloned().collect();
    ids_list.sort();
    
    ids_list.iter().map(|id| {

        let event = &events[id];
        
        format!(
            "<pre>  {} - [{}] {}</pre>",
            event.cronline.msg_format(),
            id,
            event.text
        )
    }).collect()
}

fn format_time_diff(dt: chrono::Duration) -> String {

    let (weeks, days, hours, minutes) = (
        dt.num_weeks(),
        dt.num_days(),
        dt.num_hours(),
        dt.num_minutes()
    );

    let mut text = vec![];
    if weeks > 0 { text.push(format!("{} weeks", weeks)); }
    if days > 0 { text.push(format!("{} days", days % 7)); }
    if hours > 0 { text.push(format!("{} hours", hours % 24)); }
    if minutes > 0 { 
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

    pub fn restore(state_path: &Path) -> anyhow::Result<Self> {
        info!(
            "Attempting to restore agenda from {}",
            state_path.to_string_lossy()
        );
        let data = std::fs::read_to_string(&state_path)?;
        let state: Self = serde_json::from_str(&data)?;
        Ok(state)
    }

    pub fn new() -> Self {
        AgendaState { events: HashMap::new() }
    }

    pub fn save(&self, state_path: &Path) {

        || -> anyhow::Result<()> {
            info!("Saving agenda to: {}", state_path.to_string_lossy());
            let data = serde_json::to_string_pretty(self)?;
            std::fs::write(&state_path, data)?;
            Ok(())
        }().expect("Cannot save agenda");
    }
}
