use json::JsonValue;
use regex::Regex;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::sync::Semaphore;
use tokio::time;
use log::{debug, error, info};

mod telegram_client;
use telegram_client::*;

mod util;
use util::*;

async fn handle_video_download(
    stripped: String,
    token: &str,
    chat_id: &i64,
    message_id: Option<i64>,
    reply_to_message_id: Option<i64>,
) {
    debug!("Downloading video from URL '{}'", stripped);
    send_chat_action(
        &token,
        &SendChatAction {
            chat_id: &chat_id,
            action: "upload_video",
        },
    )
    .await;
    let mut interval = time::interval(Duration::from_secs(4));
    let (download_tx, mut download_rx) = oneshot::channel();
    let mut download_finished = false;

    let token_owned = token.to_string();
    let chat_id_owned = *chat_id;
    let t = stripped.clone();

    let download_task = tokio::spawn(async move {
        let result = download_video(t).await;

        if let Some(result) = result {
            debug!("Video downloaded to path {}", result);

            let actual_path = result;
            let dimensions = get_video_dimensions(&actual_path).unwrap_or((0, 0));
            let video = SendVideo {
                chat_id: &chat_id_owned,
                reply_to_message_id,
                video_location: &actual_path,
                width: dimensions.0,
                height: dimensions.1,
            };

            let _r = send_video(&token_owned, &video).await;
            let delete = DeleteMessage {
                chat_id: &chat_id_owned,
                message_id: &message_id.unwrap_or_default(),
            };
            delete_message(&token_owned, &delete).await;
            let _r = delete_file(&actual_path);
        } else {
            debug!("download_video failed for url {}", stripped);
            telegram_client::send_message(
                &token_owned,
                &telegram_client::SendMessage {
                    chat_id: &chat_id_owned,
                    reply_to_message_id: message_id,
                    text: "Hyvä linkki...",
                },
            )
            .await;
        }
        let _ = download_tx.send("".to_owned());
    });

    interval.tick().await;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                send_chat_action(&token, &SendChatAction { chat_id: &chat_id, action: "upload_video" }).await;
            }
            Ok(..) = &mut download_rx => {
                download_finished = true;
            }
        }

        if download_finished {
            break;
        }
    }

    let _ = download_task.await;
}

async fn handle_update(update: &JsonValue) {
    let token = env::var("TOKEN").unwrap();
    if let JsonValue::Object(message) = update {
        let chat_id = message["message"]["chat"]["id"].as_i64().unwrap();
        let reply_to_message_id = message["message"]["reply_to_message"]["message_id"].as_i64();
        let message_id = message["message"]["message_id"].as_i64();
        let is_private_conversation = message["message"]["chat"]["type"] == "private";
        let ending_string = " dl";
        match message["message"]["text"].as_str() {
            Some(s) => {
                if let Some(stripped) = s.strip_suffix(ending_string) {
                    let stripped = stripped.to_string(); // Convert &str to String to pass to download_video
                                                         // tmp(
                    handle_video_download(
                        stripped,
                        &token,
                        &chat_id,
                        message_id,
                        reply_to_message_id,
                    )
                    .await;
                } else {
                    let url_regex = Regex::new(r#"(?i)\b((?:https?://|www\d{0,3}[.]|[a-z0-9.\-]+[.][a-z]{2,4}/)(?:[^\s()<>]+|\(([^\s()<>]+|(\([^\s()<>]+\)))*\))+(?:\(([^\s()<>]+|(\([^\s()<>]+\)))*\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))"#).unwrap();

                    if let Some(capture) = url_regex.captures(s) {
                        let url = capture.get(0).unwrap().as_str();
                        if is_private_conversation {
                            debug!("Extracted URL from private conversation: {}", url);
                            handle_video_download(
                                url.to_string(),
                                &token,
                                &chat_id,
                                message_id,
                                reply_to_message_id,
                            )
                            .await;
                        }
                    } else {
                        debug!("No URL found in the message.");
                    }
                    debug!("The message text does not end with the expected string.");
                }
            }
            _ => return,
        }
    }
}

async fn slow_poll(token: &str) -> ! {
    let mut last_update_id = 0;
    let max_threads = 2;
    let semaphore = Arc::new(Semaphore::new(max_threads));
    let failed_request_grace_period = Duration::from_millis(2000);

    loop {
        let t = get_updates(
            &token,
            &GetUpdates {
                timeout: &60,
                offset: &(&last_update_id + 1),
            },
        )
        .await;
        let parsed = t.unwrap();

        let ok = parsed["ok"].as_bool().unwrap_or_default();
        if !ok {
            time::sleep(failed_request_grace_period).await;
            continue;
        }

        let result = match &parsed["result"] {
            JsonValue::Array(arr) => arr,
            _ => panic!("'result' field is not an array"),
        };

        for update in result.clone() {
            if let Some(update_id) = update["update_id"].as_i64() {
                let update_id = update_id;

                let semaphore_permit = semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("Semaphore acquire error");

                tokio::spawn(async move {
                    handle_update(&update).await;
                    drop(semaphore_permit);
                });

                last_update_id = update_id;
            } else {
                error!("'update_id' field is missing or not an integer");
                continue;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let maybe_token = env::var("TOKEN");
    if maybe_token.is_err() {
        error!("Environment variable 'TOKEN' not found");
        std::process::exit(1);
    }
    let token = maybe_token.unwrap();

    info!("Bot running!");
    slow_poll(&token).await;
}
