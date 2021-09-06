mod telegram;
mod agenda;
mod http;

use std::path::Path;
use crossbeam_channel::unbounded;
use log::debug;
use telegram::Telegram;
use agenda::Agenda;
use http::HTTP_Notifier;

fn main() {

    env_logger::Builder::new()
        //.filter_module("nag", log::LevelFilter::max())
        .parse_default_env()
        .init();

    let (sender, receiver) = unbounded();

    let data_path = Path::new("data/");
    let mut telegram = Telegram::new(data_path, &sender).unwrap();
    let mut agenda = Agenda::new(data_path, &sender);
    let http_notifier = HTTP_Notifier::new(&sender);

    rayon::spawn(telegram.get_loop());
    rayon::spawn(agenda.get_loop());
    rayon::spawn(http_notifier.get_loop());

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
