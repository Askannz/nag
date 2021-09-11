use std::path::{Path, PathBuf};
use std::default::Default;
use serde::{Deserialize, Serialize};
use log::{info};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {

    pub data_path: PathBuf,
    pub http_notifier_enabled: bool,
    pub http_notifier_host: String,
    pub http_notifier_port: u64
}

impl Config {

    pub fn restore(config_path: &Path) -> anyhow::Result<Config> {

        let path_str = config_path.to_string_lossy();

        info!(
            "Attempting to restore config from {}",
            path_str
        );
        let data = std::fs::read_to_string(&config_path)?;
        let config = serde_json::from_str(&data)
            .map_err(anyhow::Error::new)
            .expect(&format!("Error parsing config from {}", path_str));


        Ok(config)
    }

    pub fn save(&self, config_path: &Path) {

        let path_str = config_path.to_string_lossy();

        info!("Saving config to: {}", path_str);

        let parent = config_path.ancestors().skip(1).next().unwrap();
        std::fs::create_dir_all(parent).expect("Cannot create parent folder of config");

        || -> anyhow::Result<()> {
            let data = serde_json::to_string_pretty(self)?;
            std::fs::write(&config_path, data)?;
            Ok(())
        }()
        .expect(&format!("Cannot save config to {}", path_str));
    }
}

impl Default for Config {
    fn default() -> Self {
        Config { 
            data_path: get_default_data_path(),
            http_notifier_enabled: true,
            http_notifier_host: "0.0.0.0".into(),
            http_notifier_port: 8123,
            date_format: DateFormat::DMY
        }
    }
}

fn get_default_data_path() -> PathBuf {
    let home_dir_path = std::env::var("HOME")
        .expect("Cannot get default data path: no $HOME set");
    std::path::PathBuf::from(home_dir_path)
        .join(".local/share/nag/data/")
}
