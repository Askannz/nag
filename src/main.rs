mod telegram;
mod agenda;
mod http;
mod config;

use std::path::Path;
use std::default::Default;
use crossbeam_channel::unbounded;
use log::{debug, info, warn};
use config::Config;
use telegram::Telegram;
use agenda::Agenda;
use http::HTTP_Notifier;

const CONFIG_PATH: &str = "config.json";
fn main() {

    env_logger::Builder::new()
        .filter_module("nag", log::LevelFilter::Debug)
        .parse_default_env()
        .init();

    let config_path = Path::new(CONFIG_PATH);
    let config = Config::restore(config_path)
        .unwrap_or_else(|err| {
            warn!("No config restored: {}", err);
            info!("Creating new default config");
            let config: Config = Default::default();
            config.save(config_path);
            config
        });

    if !config.data_path.exists() {
        panic!(
            "Data path {} does not exist",
            config.data_path.to_string_lossy()
        );
    }

    let (sender, receiver) = unbounded();

    let mut telegram = Telegram::new(&config, &sender).unwrap();
    let mut agenda = Agenda::new(&config, &sender);
    let http_notifier = HTTP_Notifier::new(&config, &sender);

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
