use crossbeam_channel::Sender;
use simple_server::{Server, Method, StatusCode};
use crate::BotUpdate;
use crate::config::Config;

#[allow(non_camel_case_types)]
pub struct HTTP_Notifier {
    config: Config,
    sender: Sender<BotUpdate>
}


impl HTTP_Notifier {

    pub fn new(config: &Config, sender: &Sender<BotUpdate>) -> Self {
        HTTP_Notifier {
            config: config.clone(),
            sender: sender.clone()
        }
    }

    pub fn get_loop(&self) -> impl FnOnce() {

        let sender = self.sender.clone();
        let config = self.config.clone();

        let server = Server::new(move |request, mut response| {

            match request.method() {
                &Method::POST => {

                    let text = String::from_utf8_lossy(request.body()).to_owned();

                    sender.send(BotUpdate::MsgOut(text.into())).unwrap();

                    response.status(StatusCode::OK);
                    Ok(response.body(vec![])?)
                },

                _ => {
                    response.status(StatusCode::METHOD_NOT_ALLOWED);
                    Ok(response.header("Allow", "POST").body(vec![])?)
                }
            }
        });

        move || {
            server.listen(
                &config.http_notifier_host,
                &format!("{}", config.http_notifier_port)
            );
        }
    }
}
