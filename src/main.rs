use reqwest::{multipart, Client, Body, StatusCode};
use tokio::sync::oneshot;
use std::time::{Duration, Instant};
use tokio::time::{self, timeout};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::time::{interval, Interval};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use std::sync::Arc;
use std::env;
use tokio;
use tokio::task;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use json::JsonValue;
use std::process::{Command, Stdio};
use uuid::Uuid;
use serde::Serialize;

#[derive(Serialize)]
struct DeleteMessage<'a> {
    chat_id: &'a str,
    message_id: &'a i64,
}

#[derive(Serialize)]
struct SendMessage<'a> {
    chat_id: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to_message_id: Option<i64>,
}

#[derive(Serialize)]
struct SendChatAction<'a> {
    chat_id: &'a str,
    action: &'a str,
}

#[derive(Serialize)]
struct SendVideo<'a> {
    chat_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to_message_id: Option<i64>,
    video_location: &'a str
}

async fn delete_message(token: &str, message: &DeleteMessage<'_>) {
    send_request(token, "deleteMessage", message).await;
}

async fn send_message(token: &str, message: &SendMessage<'_>) {
    send_request(token, "sendMessage", message).await;
}

async fn send_chat_action(token: &str, message: &SendChatAction<'_>) {
    println!("chat action sent");
    send_request(token, "sendChatAction", message).await;
}

async fn send_request<T>(token: &str, method: &str, payload: &T)
where
    T: Serialize,
{
    let api_endpoint = format!("https://api.telegram.org/bot{}/{}", token, method);
    let client = Client::new();
    let response = client.post(api_endpoint).json(payload).send().await;
    if let Ok(response) = response {
        if response.status() != StatusCode::OK {
            println!("Request failed with status code: {:?}", response.status());
        }
    } else if let Err(err) = response {
        println!("Request error: {:?}", err);
    }
}

async fn send_video(token: &str, message: &SendVideo<'_>) -> anyhow::Result<String> {
    // async fn send_video(token: &str, chat_id: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let api_endpoint = format!("https://api.telegram.org/bot{}/sendVideo?chat_id={}&reply_to_message_id={}&allow_sending_without_reply=true", token, message.chat_id, message.reply_to_message_id.unwrap_or(-1));
    let file = File::open(message.video_location).await?;

    // read file body stream
    let stream = FramedRead::new(file, BytesCodec::new());
    let file_body = Body::wrap_stream(stream);

    //make form part of file
    let some_file = multipart::Part::stream(file_body)
        .file_name("video")
        .mime_str("video/mp4")?;

    let form = multipart::Form::new()
        .part("video", some_file);
    //send request
    let response = client.post(api_endpoint).multipart(form).send().await?;
    let result = response.text().await?;
    Ok(result)
}

fn get_video_dimensions(output_path: &str) -> Result<(u32, u32), String> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v")
        .arg("-show_entries")
        .arg("stream=width,height")
        .arg("-of")
        .arg("csv=p=0:s=x")
        .arg(output_path)
        .stdout(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to execute ffprobe: {}", e))?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        let dimensions: Vec<&str> = output_str.trim().split('x').collect();
        if dimensions.len() == 2 {
            let width = dimensions[0].parse().map_err(|e| format!("Invalid width: {}", e))?;
            let height = dimensions[1].parse().map_err(|e| format!("Invalid height: {}", e))?;
            Ok((width, height))
        } else {
            Err(format!("Invalid output from ffprobe: {}", output_str))
        }
    } else {
        let error_str = String::from_utf8_lossy(&output.stderr);
        Err(format!("ffprobe failed with error code {}: {}", output.status, error_str))
    }
}

async fn download_video(url: String) -> Option<String> {
    let video_id = Uuid::new_v4();
    let file_path = format!("/tmp/{}.mp4", video_id);
    let output = Command::new("yt-dlp")
        .arg("-S")
        .arg("res,ext:mp4:m4a")

        .arg("--recode")
        .arg("mp4")

        .arg("-o")
        .arg(&file_path)

        .arg(url)

        // for debugging
        .arg("--rate-limit")
        .arg("1.0M")

        .output()
        .expect("failed to execute process");
    if output.status.success() {
        let output_path = std::fs::canonicalize(&file_path).unwrap();
        return Some(output_path.to_string_lossy().to_string());
    } else {
        println!("Download failed:");
        println!("{}", String::from_utf8_lossy(&output.stderr));
        return None;
    }
}


async fn handle_update(update: &JsonValue) {
    let token = env::var("TOKEN").unwrap();
    if let JsonValue::Object(message) = update {
        let chat_id = match message["message"]["chat"]["id"].as_i64() {
            Some(id) => id.to_string(),
            None => {
                eprintln!("Invalid chat_id: {:?}", message["message"]["chat"]["id"]);
                return;
            }
        };
        let reply_to_message_id = message["message"]["reply_to_message"]["message_id"].as_i64();
        let message_id = message["message"]["message_id"].as_i64();
        let text = message["message"]["text"].as_str();
        let ending_string = " dl";
        match text {
            Some(s) => {
                if let Some(stripped) = s.strip_suffix(ending_string) {
                    let stripped = stripped.to_string(); // Convert &str to String to pass to download_video
                    println!("Downloading video from URL: {} to ", stripped);
                    send_chat_action(&token, &SendChatAction { chat_id: &chat_id, action: "upload_video" }).await;
                    // let path = download_video(&stripped).await;


                    /* polling start */
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
                                // Call send_chat_action every 2 seconds
                                send_chat_action(&token, &SendChatAction { chat_id: &chat_id, action: "upload_video" }).await;
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
                    // Ensure the download task completes
                    let _ = download_task.await;
                    /* polling end */ 
                    if path == "" {
                        println!("dl failed for url: {}", stripped);
                        send_message(&token, &SendMessage {
                            chat_id: &chat_id,
                            reply_to_message_id: message_id,
                            text: "Hyvä linkki..."
                        }).await;
                    } else {
                        let actual_path = path;
                        let dimensions = get_video_dimensions(&actual_path);
                        let video = SendVideo {
                            chat_id: &chat_id,
                            reply_to_message_id,
                            video_location: &actual_path
                        };
                        let r = send_video(&token, &video).await;
                        println!("{:?}", r);
                        println!("Downloaded video to {}, with dimensions: {:?}", actual_path, dimensions);
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
        println!("text: {}", text.unwrap_or("kissa"));
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
    // let rt = tokio::runtime::Builder::new_multi_thread()
    //     .worker_threads(2)
    //     .enable_all()
    //     .build()
    //     .unwrap();

    // rt.spawn(async move {
    //     task::spawn_blocking(move || {
    //         loop {
    //             // Perform your polling logic here

    //             println!("Polling...");

    //             // Sleep for a duration to control the loop frequency
    //             std::thread::sleep(Duration::from_secs(1));
    //         }
    //     }).await.unwrap();
    // });
    let token = env::var("TOKEN").unwrap();
    println!("Bot running");
    slow_poll(&token).await;
}
