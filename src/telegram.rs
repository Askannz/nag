use std::sync::{Arc, Mutex};
use crossbeam_channel::Sender;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use log::{debug, info, warn, error};
use crate::BotUpdate;

const POLL_TIMEOUT: u32 = 120;

pub struct Telegram {
    api_url: String,
    context: Arc<Mutex<TelegramContext>>,
    context_path: PathBuf,
    sender: Sender<BotUpdate>
}

impl Telegram {

    pub fn new(data_path: &Path, sender: &Sender<BotUpdate>) -> anyhow::Result<Self> {

        let token = std::env::var("NAG_TELEGRAM_TOKEN")?;
        let api_url = format!("https://api.telegram.org/bot{}", token);

        let context_path = data_path.join("telegram.json");
        debug!("Telegram context path: {}", context_path.to_string_lossy());

        let context = match TelegramContext::restore(&context_path) {
            Ok(context) => context,
            Err(err) => {
                warn!("No Telegram context restored: {}", err);
                info!("Creating new context");
                TelegramContext::new()
            }
        };
        let context = Arc::new(Mutex::new(context));
        
        let telegram = Telegram { 
            api_url,
            context,
            context_path,
            sender: sender.clone()
        };

        Ok(telegram)
    }

    pub fn send(&mut self, text: &str) -> anyhow::Result<()> {

        let context = self.context.lock().unwrap();

        if let Some(chat_id) = context.chat_id {

            let url = format!("{}/sendMessage", self.api_url);
            let json = ureq::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "HTML"
            });

            ureq::post(&url)
                .send_json(json)
                .map_err(|err| anyhow::anyhow!("{:?}", err))?;

            Ok(())

        } else {
            anyhow::bail!("no known ChatID stored")
        }
    }

    pub fn get_loop(&self) -> impl FnOnce() {

        let api_url = self.api_url.clone();
        let context = self.context.clone();
        let sender = self.sender.clone();
        let context_path = self.context_path.clone();

        move || {

            let mut offset = 0u32;
    
            let mut relay_updates = move || -> anyhow::Result<()> {
    
                let poll_url = format!(
                    "{}/getUpdates?offset={}&timeout={}&allowed_updates=[\"message\"]",
                    api_url, offset, POLL_TIMEOUT
                );
            
                let api_res: ReturnedUpdates = ureq::get(&poll_url)
                    .call()?
                    .into_json()?;
    
                let mut updates = api_res.result;
                updates.sort_by_key(|update| update.update_id);
    
                if let Some(latest_update) = updates.last() {
    
                    offset = latest_update.update_id + 1;
                    let chat_id = latest_update.message.chat.id;
    
                    {
                        let mut context = context.lock().unwrap();
                        context.update_chat_id(chat_id);
                        context.save(&context_path);
                    }
                }
    
                updates.iter()
                    .for_each(|update| {
                        debug!("Telegram update: {:?}", update);
                        let text = update.message.text.clone();
                        info!("Received Telegram message: {}", text);
                        sender.send(BotUpdate::MsgIn(text)).unwrap()
                    });
    
                Ok(())
            };
    
            info!("Starting Telegram polling loop");
            loop {
                let res = relay_updates();
                if let Err(err) = res {
                    error!("Telegram: error retrieving updates: {}", err);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ReturnedUpdates {
    ok: bool,
    result: Vec<Update>
}

#[derive(Debug, Clone, Deserialize)]
struct Update {
    update_id: u32,
    message: Message
}

#[derive(Debug, Clone, Deserialize)]
struct Message {
    message_id: u32,
    text: String,
    chat: Chat
}
#[derive(Debug, Clone, Deserialize)]
struct Chat {
    id: u32
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TelegramContext {
    chat_id: Option<u32>
}

impl TelegramContext {

    fn restore(context_path: &Path) -> anyhow::Result<TelegramContext> {
        info!(
            "Attempting to restore Telegram context from {}",
            context_path.to_string_lossy()
        );
        let data = std::fs::read_to_string(&context_path)?;
        let context = serde_json::from_str(&data)?;
        Ok(context)
    }

    fn new() -> Self {
        TelegramContext { chat_id: None }
    }

    fn save(&self, context_path: &Path) {

        || -> anyhow::Result<()> {
            info!("Saving Telegram context to: {}", context_path.to_string_lossy());
            let data = serde_json::to_string_pretty(self)?;
            std::fs::write(&context_path, data)?;
            Ok(())
        }().expect("Cannot save Telegram context");
    }

    fn update_chat_id(&mut self, new_id: u32) {

        let update = self.chat_id.map_or(true, |chat_id| chat_id != new_id);
        if update {
            info!("Active ChatID changed to {}", new_id);
            self.chat_id = Some(new_id);   
        }
    }
}
