use firestore_db_and_auth::{documents, errors::Result, Credentials, ServiceSession};
use serde::{Deserialize, Serialize};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::ServerMessage;
use twitch_irc::ClientConfig;
use twitch_irc::SecureTCPTransport;
use twitch_irc::TwitchIRCClient;

#[derive(Clone, Serialize, Deserialize)]
struct Question {
    id: String,
    username: String,
    message: String,
    timestamp: String,
}

fn write(session: &ServiceSession, obj: &Question) -> Result<()> {
    let result = documents::write(
        session,
        "comments",
        Some(obj.id.clone()),
        &obj,
        documents::WriteOptions::default(),
    )?;
    println!(
        "id: {}, created: {}, updated: {}",
        result.document_id,
        result.create_time.unwrap(),
        result.update_time.unwrap()
    );
    Ok(())
}

fn get_question(message: ServerMessage) -> Option<Question> {
    match message {
        ServerMessage::Privmsg(priv_message) => Some(Question {
            id: priv_message.message_id,
            username: priv_message.sender.login,
            message: priv_message.message_text,
            timestamp: priv_message.server_timestamp.to_string(),
        }),
        _ => None,
    }
}

#[tokio::main]
pub async fn main() {
    // Create credentials object. You may as well do that programmatically.
    let cred = Credentials::from_file("comments-on-stream-468cda9bc5a4.json")
        .expect("Read credentials file");

    // To use any of the Firestore methods, you need a session first. You either want
    // an impersonated session bound to a Firebase Auth user or a service account session.
    let session = ServiceSession::new(cred).expect("Create a service account session");

    // default configuration is to join chat as anonymous.
    let config = ClientConfig::default();
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

    // first thing you should do: start consuming incoming messages,
    // otherwise they will back up.
    let join_handle = tokio::spawn(async move {
        while let Some(message) = incoming_messages.recv().await {
            let question: Option<Question> = get_question(message);

            if question.is_some() {
                let values = question.unwrap();
                println!("{0}: {1}", values.username, values.message);
                if write(&session, &values).is_ok() {
                    println!("Inserted with success")
                }
            }
        }
    });

    // join a channel
    client.join("yoannfleurydev".to_owned());

    // keep the tokio executor alive.
    // If you return instead of waiting the background task will exit.
    join_handle.await.unwrap();
}
