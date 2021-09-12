mod telegram;
mod agenda;
mod http;

use std::path::PathBuf;
use std::str::FromStr;
use crossbeam_channel::unbounded;
use clap::{Clap, AppSettings, ArgEnum};
use log::debug;
use telegram::Telegram;
use agenda::Agenda;
use http::HTTP_Notifier;

fn main() {

    let opts = Opts::parse();

    env_logger::Builder::new()
        .filter_module("nag", opts.verbosity.to_level())
        .init();

    let (sender, receiver) = unbounded();

    let mut telegram = Telegram::new(&opts, &sender);
    let mut agenda = Agenda::new(&opts, &sender);
    let http_notifier = HTTP_Notifier::new(&opts, &sender);

    rayon::spawn(telegram.get_loop());
    rayon::spawn(agenda.get_loop());
    rayon::spawn(http_notifier.get_loop());

    let version = env!("CARGO_PKG_VERSION");
    telegram.send(&format!("Nag version {}", version));

    loop {

        let update = receiver.recv().unwrap();
        debug!("BotUpdate: {:?}", update);

        match update {
            BotUpdate::MsgIn(msg) => agenda.process(&msg),
            BotUpdate::MsgOut(msg) => telegram.send(&msg)
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

    #[clap(required=true)]
    data_path: PathBuf,

    #[clap(
        long, arg_enum, default_value="dmy",
        about=
            "Day/Month/Year or Month/Day/Year.\n\
            Affects both parsing and displaying.\n"
    )]
    date_format: DateFormat,

    #[clap(long, parse(try_from_str), default_value="true")]
    http_endpoint: bool,

    #[clap(long, default_value="0.0.0.0")]
    endpoint_host: String,

    #[clap(long, parse(try_from_str), default_value="8123")]
    endpoint_port: u16,

    #[clap(long, short, arg_enum, default_value="info")]
    verbosity: Verbosity
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

#[derive(ArgEnum, Clap, Clone, Debug)]
enum Verbosity {
    Off, Trace, Debug, Info, Warn, Error
}

impl FromStr for Verbosity {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = match s.to_lowercase().as_str() {
            "off"   => Self::Off,
            "trace" => Self::Trace,
            "debug" => Self::Debug,
            "info"  => Self::Info,
            "warn"  => Self::Warn,
            "error" => Self::Error,
            _ => anyhow::bail!("Cannot parse {}", s)
        };
        Ok(v)
    }
}

impl Verbosity {
    fn to_level(&self) -> log::LevelFilter {
        match self {
            Self::Off   => log::LevelFilter::Off,
            Self::Trace => log::LevelFilter::Trace,
            Self::Debug => log::LevelFilter::Debug,
            Self::Info  => log::LevelFilter::Info,
            Self::Warn  => log::LevelFilter::Warn,
            Self::Error => log::LevelFilter::Error
        }
    }
}
