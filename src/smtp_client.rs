use lettre::{
    transport::smtp::{authentication::Credentials, client::Tls},
    Message, SmtpTransport, Transport,
};
use secrecy::{ExposeSecret, Secret};

pub struct SmtpMailer {
    mailer: SmtpTransport,
}

impl SmtpMailer {
    pub fn new(host: &str, port: u16, username: &str, password: Secret<String>) -> Self {
        let credentials =
            Credentials::new(username.to_string(), password.expose_secret().to_owned());

        let mailer = SmtpTransport::relay(host)
            .unwrap()
            .port(port)
            .tls(Tls::None)
            .credentials(credentials)
            .build();

        Self { mailer }
    }

    pub fn send(&self, email: &Message) -> Result<(), String> {
        let _ = self.mailer.send(email).map_err(|e| e.to_string());

        Ok(())
    }
}
// pub fn send(
//     from: &str,
//     to: &str,
//     subject: String,
//     body: String,
//     _reply_to: Option<&str>,
//     _no_tls: Option<bool>,
//     host: &str,
//     port: u16,
//     username: String,
//     password: Secret<String>,
// ) {
//     let email = Message::builder()
//         .from(from.parse().unwrap())
//         .to(to.parse().unwrap())
//         .subject(subject)
//         .body(body)
//         .unwrap();

//     let credentials = Credentials::new(username, password.expose_secret().clone());

//     let mailer = SmtpTransport::relay(host)
//         .unwrap()
//         .port(port)
//         .tls(Tls::None)
//         .credentials(credentials)
//         .build();

//     match mailer.send(&email) {
//         Ok(_) => println!("Success!"),
//         Err(e) => panic!("Fail: {}", e)
//     }
// }
