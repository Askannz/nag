mod telegram;
mod agenda;
mod http;
mod config;

use std::path::Path;
use std::default::Default;
use crossbeam_channel::unbounded;
use clap::{Clap, AppSettings};
use log::{debug, info, warn};
use config::Config;
use telegram::Telegram;
use agenda::Agenda;
use http::HTTP_Notifier;

fn main() {

    env_logger::Builder::new()
        .filter_module("nag", log::LevelFilter::Debug)
        .parse_default_env()
        .init();

    let opts = Opts::parse();

    let config_path = Path::new(&opts.config);
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

const DEFAULT_CONFIG_PATH: &str = "config.json";
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const AUTHOR: &'static str = env!("CARGO_PKG_AUTHORS");

#[derive(Clap, Debug)]
#[clap(version=VERSION, author=AUTHOR)]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(short, long, default_value=DEFAULT_CONFIG_PATH)]
    config: String
}
