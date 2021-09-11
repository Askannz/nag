mod telegram;
mod agenda;
mod http;

use std::path::PathBuf;
use std::str::FromStr;
use crossbeam_channel::unbounded;
use clap::{Clap, AppSettings, ArgEnum};
use log::{debug};
use telegram::Telegram;
use agenda::Agenda;
use http::HTTP_Notifier;

fn main() {

    env_logger::Builder::new()
        .filter_module("nag", log::LevelFilter::Debug)
        .parse_default_env()
        .init();

    let opts = Opts::parse();

    let (sender, receiver) = unbounded();

    let mut telegram = Telegram::new(&opts, &sender);
    let mut agenda = Agenda::new(&opts, &sender);
    let http_notifier = HTTP_Notifier::new(&opts, &sender);

    rayon::spawn(telegram.get_loop());
    rayon::spawn(agenda.get_loop());
    rayon::spawn(http_notifier.get_loop());

    telegram.send(&format!("Nag version {}", env!("CARGO_PKG_VERSION"))).unwrap();

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

#[derive(Clap, Debug, Clone)]
#[clap(version, author)]
#[clap(setting = AppSettings::ColoredHelp)]
#[clap(setting = AppSettings::DeriveDisplayOrder)]
pub struct Opts {

    #[clap(required=true, group="aaa")]
    data_path: PathBuf,

    #[clap(long, parse(try_from_str), default_value="true")]
    http_endpoint: bool,

    #[clap(long, default_value="0.0.0.0")]
    endpoint_host: String,

    #[clap(long, parse(try_from_str), default_value="8123")]
    endpoint_port: u16
}

#[derive(ArgEnum, Clap, Clone, Debug)]
pub enum DateFormat {
    MDY,
    DMY
}

impl FromStr for DateFormat {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dmy" => Ok(Self::DMY),
            "mdy" => Ok(Self::MDY),
            _ => anyhow::bail!("Cannot parse {}", s)
        }
    }
}
