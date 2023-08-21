use json::JsonValue;
use log::{debug, error, info};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::sync::Semaphore;
use tokio::time;
use tokio::task;

mod telegram_client;
use telegram_client::*;

mod util;
use util::*;

async fn chat_action_loop(mut rx_done: oneshot::Receiver<()>, token: String, chat_id: i64) {
    loop {
        match rx_done.try_recv() {
            Err(oneshot::error::TryRecvError::Empty) => {
                send_chat_action(
                    &token,
                    &SendChatAction {
                        chat_id: &chat_id,
                        action: "upload_video",
                    },
                ).await;
                time::sleep(time::Duration::from_secs(4)).await;
            }
            _ => {
                debug!("Ending action loop");
                return
            }
        }

    }
}

async fn complain(token: &str, chat_id: &i64, message_id: Option<i64>) {
    telegram_client::send_message(
        &token,
        &telegram_client::SendMessage {
            chat_id: &chat_id,
            reply_to_message_id: message_id,
            text: "Hyvä linkki...",
        },
    )
    .await;
}

async fn send_video_and_delete_message(
    token: &str,
    chat_id: &i64,
    message_id: &i64,
    video_location: &str,
    reply_to_message_id: Option<i64>,
) {
    let dimensions = get_video_dimensions(video_location).unwrap_or((0, 0));
    let video = SendVideo {
        chat_id,
        reply_to_message_id,
        video_location,
        width: dimensions.0,
        height: dimensions.1,
    };
    let _r = send_video(token, &video).await;

    let delete = DeleteMessage {
        chat_id,
        message_id: &message_id,
    };
    delete_message(token, &delete).await;
}

async fn handle_video_download(
    stripped: String,
    token: &str,
    chat_id: &i64,
    message_id: Option<i64>,
    reply_to_message_id: Option<i64>,
    _is_private_conversation: bool
) {
    debug!("dl start");

    let (done_sender, done_receiver) = oneshot::channel();

    let url = extract_urls(&stripped);
    if url.len() == 0 {
        debug!("no url found");
        complain(&token, &chat_id, message_id).await;
        return
    }

    let send_chat_action_handle = task::spawn(chat_action_loop(done_receiver, token.clone().to_string(), chat_id.clone()));

    let download_video_handle = task::spawn(download_video(url[0].clone()));

    let leftovers = stripped.replace(&url[0], "");
    let whitelisted_chats: Vec<i64> = get_config_value(EnvVariable::OpenAiChats)
        .split(";")
        .map(|id| id.parse::<i64>().unwrap_or_default())
        .collect();
    // TODO - this whitelisting is not in use right now
    let _openai_allowed_in_this_chat = whitelisted_chats.contains(chat_id);
    let parse_cut_args_handle = task::spawn(parse_cut_args(leftovers.clone()));

    debug!("Downloading video from URL '{}'", stripped);

    let sending_video_succeeded = match tokio::join!(download_video_handle, parse_cut_args_handle) {
        (Ok(download_video_handle_consumed), Ok(parse_cut_args_handle_consumed)) => {
            match (download_video_handle_consumed, parse_cut_args_handle_consumed) {
                (None, _) => false,
                (Some(video_location), None) => { 
                    send_video_and_delete_message(token, chat_id, &message_id.unwrap_or_default(), &video_location, reply_to_message_id).await;
                    delete_file(&video_location);
                    true
                },
                (Some(video_location), Some(cut_args)) => {
                    if let Some(cut_video_location) = cut_video(&video_location, &cut_args.0, cut_args.1){
                        send_video_and_delete_message(token, chat_id, &message_id.unwrap_or_default(), &cut_video_location.as_str(), reply_to_message_id).await;
                        delete_file(&video_location);
                        delete_file(&cut_video_location);
                        true
                    } else {
                        false
                    }
                },
            }
        },
        (Ok(download_video_handle_consumed), Err(_)) => {
            match download_video_handle_consumed {
                Some(video_location) => {
                    send_video_and_delete_message(token, chat_id, &message_id.unwrap_or_default(), &video_location, reply_to_message_id).await;
                    delete_file(&video_location);
                    true
                },
                None => false,
            }
        },
        (Err(_), _) => {
            debug!("Downloading video has failed");
            false
        }
    };

    if !sending_video_succeeded {
        complain(&token, &chat_id, message_id).await;
    }


    let _ = done_sender.send(());
    send_chat_action_handle.await.expect("Send chat action panicked");
}

async fn handle_update(update: &JsonValue) {
    let token = get_config_value(EnvVariable::TelegramToken);
    if let JsonValue::Object(ref message) = update {
        let maybe_chat_id = message["message"]["chat"]["id"].as_i64();
        if maybe_chat_id.is_none() {
            debug!("Encountered update with no message.chat.id object");
            return;
        }
        let chat_id = maybe_chat_id.unwrap();
        let reply_to_message_id = message["message"]["reply_to_message"]["message_id"].as_i64();
        let message_id = message["message"]["message_id"].as_i64();
        let _is_private_conversation =
            message["message"]["chat"]["type"].as_str() == Some("private");

        let ending_string = " dl";
        let starting_string = "!";

        if let Some(text) = message["message"]["text"].as_str() {
            let text_lowercase = text.to_lowercase();
            if text_lowercase.starts_with(starting_string) || text_lowercase.ends_with(ending_string) {
                let stripped = if text.starts_with(starting_string) {
                    &text[starting_string.len()..]
                } else {
                    &text[..text.len() - ending_string.len()]
                };
                
                handle_video_download(
                    stripped.to_string(),
                    &token,
                    &chat_id,
                    message_id,
                    reply_to_message_id,
                    _is_private_conversation
                )
                .await;
            }
        } else {
            debug!("no text content");
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

    let token = get_config_value(EnvVariable::TelegramToken);

    info!("Bot running!");
    slow_poll(&token).await;
}
