use reqwest::Client;
use tokio::sync::oneshot;
use std::time::Duration;
use tokio::time;
use tokio::sync::Semaphore;
use std::sync::Arc;
use std::env;
use json::JsonValue;

mod telegram_client;
use telegram_client::*;

mod util;
use util::*;

async fn handle_update(update: &JsonValue) {
    let token = env::var("TOKEN").unwrap();
    if let JsonValue::Object(message) = update {
        let chat_id = message["message"]["chat"]["id"].as_i64().unwrap();
        let reply_to_message_id = message["message"]["reply_to_message"]["message_id"].as_i64();
        let message_id = message["message"]["message_id"].as_i64();
        let ending_string = " dl";
        match message["message"]["text"].as_str() {
            Some(s) => {
                if let Some(stripped) = s.strip_suffix(ending_string) {
                    let stripped = stripped.to_string(); // Convert &str to String to pass to download_video
                    println!("Downloading video from URL: {} to ", stripped);
                    send_chat_action(&token, &SendChatAction { chat_id: &chat_id, action: "upload_video" }).await;
                    let mut interval = time::interval(Duration::from_secs(4));
                    let (download_tx, mut download_rx) = oneshot::channel();
                    let mut download_finished = false;

                    // Spawn the download_video task
                    let t = stripped.clone();
                    let download_task = tokio::spawn(async move {
                        let result = download_video(t).await;
                        let _ = download_tx.send(result);
                    });

                    // Execute send_chat_action every 2 seconds until download is finished
                    interval.tick().await; // Start the first tick immediately

                    let mut path: String = "".to_string();
                    loop {
                        tokio::select! {
                            _ = interval.tick() => {
                                // Call send_chat_action every 2 seconds send_chat_action(&token, &SendChatAction { chat_id: &chat_id, action: "upload_video" }).await;
                            }
                            Ok(download_result) = &mut download_rx => {
                                // Download completed
                                download_finished = true;

                                if let Some(result) = download_result {
                                    // Video downloaded successfully
                                    println!("Video downloaded: {}", result);
                                    path = result;
                                } else {
                                    // Video download failed
                                    println!("Video download failed");
                                }
                            }
                        }

                        if download_finished {
                            break;
                        }
                    }
                    let _ = download_task.await;

                    if path == "" {
                        println!("dl failed for url: {}", stripped);
                        telegram_client::send_message(&token, &telegram_client::SendMessage {
                            chat_id: &chat_id,
                            reply_to_message_id: message_id,
                            text: "Hyvä linkki..."
                        }).await;
                    } else {
                        let actual_path = path;
                        let video = SendVideo {
                            chat_id: &chat_id,
                            reply_to_message_id,
                            video_location: &actual_path
                        };
                        let r = send_video(&token, &video).await;
                        println!("{:?}", r);
                        let delete = DeleteMessage {
                            chat_id: &chat_id,
                            message_id: &message_id.unwrap_or_default()
                        };
                        delete_message(&token, &delete).await;
                    }
                } else {
                    eprintln!("The message text does not end with the expected string.");
                }
            },
            _ => return,
        }
    }
}

async fn slow_poll(token: &str) -> ! {
    let client = Client::new();
    let mut last_update_id = 0;
    let max_threads = 2;
    let semaphore = Arc::new(Semaphore::new(max_threads));

    loop {
        let url = format!("https://api.telegram.org/bot{}/getUpdates?timeout=60&offset={}", token, last_update_id + 1);

        let res = match client.get(&url).send().await {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Error: {}", e);
                continue;
            }
        };


        let body = match res.text().await {
            Ok(body) => body,
            Err(e) => {
                eprintln!("Error: {}", e);
                continue;
            }
        };

        let parsed = match json::parse(&body) {
            Ok(parsed) => parsed,
            Err(e) => {
                eprintln!("Error: {}", e);
                continue;
            }
        };

        let ok = parsed["ok"].as_bool().unwrap_or_default();
        if !ok {
            continue;
        }

        let result = match &parsed["result"] {
            JsonValue::Array(arr) => arr,
            _ => panic!("'result' field is not an array")
        };


        for update in result.clone() {
            if let Some(update_id) = update["update_id"].as_i64() {
                let update_id = update_id;

                let semaphore_permit = semaphore.clone().acquire_owned().await.expect("Semaphore acquire error");

                tokio::spawn(async move {
                    handle_update(&update).await;
                    drop(semaphore_permit); // Release the semaphore permit when the thread is done
                });

                last_update_id = update_id;
            } else {
                eprintln!("'update_id' field is missing or not an integer");
                continue;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("TOKEN").unwrap();
    println!("Bot running");
    slow_poll(&token).await;
}
