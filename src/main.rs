mod telegram;
mod agenda;

use std::path::Path;
use std::sync::mpsc::channel;
use log::debug;
use telegram::Telegram;
use agenda::Agenda;

fn main() {

    env_logger::Builder::new()
        .filter_module("nag", log::LevelFilter::max())
        .init();

    let (sender, receiver) = channel();

    let data_path = Path::new("data/");
    let mut telegram = Telegram::new(data_path, &sender).unwrap();
    let mut agenda = Agenda::new(data_path, &sender);

    rayon::spawn(telegram.get_loop());
    rayon::spawn(agenda.get_loop());

    loop {

        let update = receiver.recv().unwrap();
        debug!("BotUpdate: {:?}", update);

        match update {
            BotUpdate::MsgIn(msg) => agenda.process(&msg),
            BotUpdate::MsgOut(msg) => telegram.send(&msg).unwrap()
        }
    }

}

#[derive(Debug)]
pub enum BotUpdate {
    MsgIn(String),
    MsgOut(String)
}
