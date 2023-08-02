use anyhow::Result;
use json::JsonValue;
use log::{debug, error};
use reqwest::{multipart, Body};
use serde::Serialize;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Serialize)]
pub struct DeleteMessage<'a> {
    pub chat_id: &'a i64,
    pub message_id: &'a i64,
}

#[derive(Serialize)]
pub struct GetUpdates<'a> {
    pub timeout: &'a i64,
    pub offset: &'a i64,
}

#[derive(Serialize)]
pub struct SendMessage<'a> {
    pub chat_id: &'a i64,
    pub text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_message_id: Option<i64>,
}

#[derive(Serialize)]
pub struct SendChatAction<'a> {
    pub chat_id: &'a i64,
    pub action: &'a str,
}

#[derive(Serialize)]
pub struct SendVideo<'a> {
    pub chat_id: &'a i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_message_id: Option<i64>,
    pub video_location: &'a str,
    pub width: u32,
    pub height: u32,
}

pub async fn delete_message(token: &str, message: &DeleteMessage<'_>) {
    let _ = send_request(token, "deleteMessage", message).await;
}

pub async fn send_message(token: &str, message: &SendMessage<'_>) {
    let _ = send_request(token, "sendMessage", message).await;
}

pub async fn send_chat_action(token: &str, message: &SendChatAction<'_>) {
    let _ = send_request(token, "sendChatAction", message).await;
}

pub async fn get_updates(token: &str, message: &GetUpdates<'_>) -> Result<JsonValue> {
    send_request(token, "getUpdates", message).await
}

pub async fn send_request<T>(token: &str, method: &str, payload: &T) -> Result<JsonValue>
where
    T: Serialize,
{
    debug!("Sending {} request to Telegram API", method);
    let api_endpoint = format!("https://api.telegram.org/bot{}/{}", token, method);
    let client = reqwest::Client::new();
    let response = client.post(api_endpoint).json(payload).send().await?;

    if response.status() != reqwest::StatusCode::OK {
        error!(
            "Telegram API request {} failed with status code {:?}",
            method,
            response.status()
        );
    }

    let body = response.text().await?;
    let parsed = json::parse(&body)?;
    Ok(parsed)
}

pub async fn send_video(token: &str, message: &SendVideo<'_>) {
    let client = reqwest::Client::new();
    let api_endpoint = format!("https://api.telegram.org/bot{}/sendVideo?chat_id={}&reply_to_message_id={}&allow_sending_without_reply=true&width={}&height={}", token, message.chat_id, message.reply_to_message_id.unwrap_or(-1), message.width, message.height);

    debug!("Video upload starting for video '{}'", message.video_location);
    if let Ok(file) = File::open(message.video_location).await {
        let stream = FramedRead::new(file, BytesCodec::new());
        let file_body = Body::wrap_stream(stream);

        if let Ok(some_file) = multipart::Part::stream(file_body)
            .file_name("video")
            .mime_str("video/mp4")
        {
            let form = multipart::Form::new().part("video", some_file);

            if let Ok(response) = client.post(api_endpoint).multipart(form).send().await {
                let _ = response.text().await;
            }
        }
    }
    debug!("Video upload done for video '{}'", message.video_location);
}
