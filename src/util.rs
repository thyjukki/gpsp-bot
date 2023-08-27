use lazy_static::lazy_static;
use regex::Regex;
use log::debug;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use uuid::Uuid;
use reqwest::Client;

pub async fn download_video(url: String) -> Option<String> {
    let video_id = Uuid::new_v4();
    let file_path = format!("/tmp/{}.mp4", video_id);
    let target_size_in_m = 50;
    let output = Command::new("yt-dlp")
        .arg("--max-filesize")
        .arg(format!("{}M", target_size_in_m))
        .arg("-f")
        .arg(format!(
            "((bv*[filesize<={}]/bv*)[height<=720]/(wv*[filesize<={}]/wv*)) + ba / (b[filesize<={}]/b)[height<=720]/(w[filesize<={}]/w)",
            target_size_in_m, target_size_in_m, target_size_in_m, target_size_in_m
        ))
        .arg("-S")
        .arg("codec:h264")
        .arg("--merge-output-format")
        .arg("mp4")
        .arg("--recode")
        .arg("mp4")
        .arg("-o")
        .arg(&file_path)
        .arg(url)
        // for debugging
        // .arg("--rate-limit")
        // .arg("0.05M")
        .output()
        .expect("failed to execute process");
    debug!(
        "yt-dlp stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success() {
        let output_path = std::fs::canonicalize(&file_path).unwrap();
        return Some(output_path.to_string_lossy().to_string());
    } else {
        debug!(
            "yt-dlp failed with\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return None;
    }
}

pub fn cut_video(
    path_in: &str,
    start_seconds: &f64,
    duration_seconds: Option<f64>,
) -> Option<String> {
    let video_id = Uuid::new_v4();
    let path_out = format!("/tmp/{}.mp4", video_id);
    /* https://stackoverflow.com/questions/18444194/cutting-the-videos-based-on-start-and-end-time-using-ffmpeg
         *
         * toSeconds() {
        awk -F: 'NF==3 { print ($1 * 3600) + ($2 * 60) + $3 } NF==2 { print ($1 * 60) + $2 } NF==1 { print 0 + $1 }' <<< $1
    }

    StartSeconds=$(toSeconds "45.5")
    EndSeconds=$(toSeconds "1:00.5")
    Duration=$(bc <<< "(${EndSeconds} + 0.01) - ${StartSeconds}" | awk '{ printf "%.4f", $0 }')
    ffmpeg -ss $StartSeconds -i input.mpg -t $Duration output.mpg
         */

    debug!(
        "cut_video called with {}, {}, {}, {}",
        path_in,
        path_out,
        start_seconds,
        duration_seconds.unwrap_or(-1.0)
    );

    let mut cmd = Command::new("ffmpeg");

    let sanitized_start_seconds = if start_seconds >= &0.0 {
        start_seconds.to_owned()
    } else {
        let output = Command::new("ffprobe")
            .arg("-v")
            .arg("error")
            .arg("-show_entries")
            .arg("format=duration")
            .arg("-of")
            .arg("default=noprint_wrappers=1:nokey=1")
            .arg(path_in)
            .output()
            .expect("Failed to execute ffprobe");
        let video_duration: f64 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .expect("Failed to parse video duration");
        video_duration + start_seconds
    };

    // debugging
    // cmd.arg("-loglevel").arg("debug").arg("-report");

    cmd.arg("-ss").arg(format!("{}", sanitized_start_seconds));

    if let Some(duration_seconds) = duration_seconds {
        cmd.arg("-t").arg(format!("{}", duration_seconds));
    }

    let cmd = cmd
        .arg("-i")
        .arg(path_in)
        .arg(path_out.clone())
        .output()
        .expect("ffmpeg failed");
        // .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

    debug!("ffmpeg output: {:?}", String::from_utf8(cmd.clone().stderr));

    if cmd.status.success() {
        Some(path_out)
    } else {
        debug!("FFMPEG FAILED");
        None
        // Err(format!(
        //     "ffmpeg failed with error code {}: {:?}",
        //     cmd.status,
        //     String::from_utf8(cmd.stderr)
        // ))
    }
}

pub fn get_video_dimensions(output_path: &str) -> Result<(u32, u32), String> {
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
            let width = dimensions[0]
                .parse()
                .map_err(|e| format!("Invalid width: {}", e))?;
            let height = dimensions[1]
                .parse()
                .map_err(|e| format!("Invalid height: {}", e))?;
            Ok((width, height))
        } else {
            Err(format!("Invalid output from ffprobe: {}", output_str))
        }
    } else {
        let error_str = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "ffprobe failed with error code {}: {}",
            output.status, error_str
        ))
    }
}

pub fn delete_file(file: &str) {
    if let Err(err) = fs::remove_file(file) {
        debug!("error deleting file: {}", err);
    }
}

pub enum EnvVariable {
    TelegramToken,
    OpenAiToken,
    OpenAiChats,
}

lazy_static! {
    static ref CONFIG_VALUES: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

impl EnvVariable {
    pub fn get(&self) -> (&str, Option<&str>) {
        match self {
            EnvVariable::TelegramToken => ("TELEGRAM_TOKEN", None),
            EnvVariable::OpenAiToken => ("OPENAI_TOKEN", Some("")),
            EnvVariable::OpenAiChats => ("OPENAI_CHATS", Some("")),
        }
    }
}

pub enum Platform {
    Discord,
    Telegram
}

pub fn get_platform() -> Platform {
    match std::env::args().nth(1).unwrap_or_default().as_str() {
        "telegram" => Platform::Telegram,
        "discord" => Platform::Discord,
        _ => panic!("Supported platform must be given as a first argument")
    }
}

pub fn get_config_value(env_variable: EnvVariable) -> String {
    let (env_variable_name, default_value) = env_variable.get();
    let env_variable_name_file = env::var(format!("{}_FILE", env_variable_name))
        .unwrap_or("/dev/null/nonexistent".to_string());

    if let Some(cached_value) = CONFIG_VALUES.lock().unwrap().get(env_variable_name) {
        debug!("read variable {}: {}", env_variable_name, cached_value);
        return cached_value.clone();
    }

    fn store_value(env_variable_name: &str, value: &String) {
        CONFIG_VALUES
            .lock()
            .unwrap()
            .insert(env_variable_name.to_string(), value.to_string());
    }

    if let Ok(env_variable_value) = env::var(env_variable_name.clone()) {
        store_value(env_variable_name, &env_variable_value);
    } else if let Ok(env_variable_file_content) = fs::read_to_string(env_variable_name_file.clone()) {
        store_value(env_variable_name, &env_variable_file_content.trim().to_string());
    } else if let Some(env_variable_default_value) = default_value {
        let env_variable_default_value_string = env_variable_default_value.to_string();
        store_value(env_variable_name, &env_variable_default_value_string);
    } else {
        panic!(
            "No {} or {} environment variables found",
            env_variable_name, env_variable_name_file
        );
    }

    get_config_value(env_variable)
}
/// Parse start seconds and duration from 
/// natural language query using OpenAI API 
///
/// # Arguments
///
/// * `msg` - Message as a &str.
///
/// # Returns
///
/// Starting timestamp and duration in seconds
///
/// # Examples
///
/// let result = parse_cut_args("Cut clip 10.5s-1m10s").await;
/// assert_eq!(result, (10.5, 60.0);
pub async fn parse_cut_args(msg: String) -> Option<(f64, Option<f64>)> {
    if msg.chars().count() <= 3 {
        return None
    }
    let request_body = json::object! {
        "model": "gpt-3.5-turbo-0613",
        "messages": [
            {
                "role": "user",
                "content": msg.clone()
            }
        ],
        "functions": [
            {
                "name": "cut_video",
                "description": "Cut video video with subsecond level accuracy. Instructions  are likely in english or finnish.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "start_minutes": {
                            "type": "number",
                            "description": "Start timestamp of cut the video in minutes. Negative if something like 'last 1m' is requested'."
                        },
                        "start_seconds": {
                            "type": "number",
                            "description": "Start timestamp of the cut video in seconds. Negative if something like 'last 30s' is requested'."
                        },
                        "end_seconds": {
                            "type": "number",
                            "description": "End timestamp of the cut video in seconds."
                        },
                        "end_minutes": {
                            "type": "number",
                            "description": "End timestamp of the cut video in minutes."
                        },
                        "duration_minutes": {
                            "type": "number",
                            "description": "Duration of the resulting clip in minutes."
                        },
                        "duration_seconds": {
                            "type": "number",
                            "description": "Duration of the resulting clip in seconds."
                        },
                    },
                    "required": ["start_minutes", "start_seconds"]
                }
            }
        ]
    };

    let client = Client::new();
    let token = get_config_value(EnvVariable::OpenAiToken);
    debug!("OPENAI REQUEST SENDING NEXT");
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .body(request_body.to_string())
        .send()
        .await.expect("openai parsing failed");
    debug!("OPENAI REQUEST DONE");

    let body = response.text().await.unwrap();
    let parsed = json::parse(&body).unwrap();

    let result_result =
        json::parse(&parsed["choices"][0]["message"]["function_call"]["arguments"].to_string()).unwrap();
    let start_seconds = result_result["start_seconds"].as_f64().unwrap_or_default();
    let start_minutes = result_result["start_minutes"].as_f64().unwrap_or_default();
    let end_seconds = result_result["end_seconds"].as_f64().unwrap_or_default();
    let end_minutes = result_result["end_minutes"].as_f64().unwrap_or_default();
    let duration_minutes = result_result["duration_minutes"]
        .as_f64()
        .unwrap_or_default();
    let duration_seconds = result_result["duration_seconds"]
        .as_f64()
        .unwrap_or_default();

    debug!("openai: {} {}", start_seconds, start_minutes);

    let start_only_seconds = start_minutes * 60. + start_seconds;
    let duration_only_seconds = if duration_minutes > 0. || duration_seconds > 0. {
        Some(duration_minutes * 60. + duration_seconds)
    } else if end_minutes > 0. || end_seconds > 0. {
        Some((end_minutes * 60. + end_seconds) - start_only_seconds)
    } else {
        None
    };
    Some((start_only_seconds, duration_only_seconds))
}

pub fn extract_urls(input: &str) -> Vec<String> {
    let url_regex = Regex::new(r#"(?i)\b((?:https?://|www\d{0,3}[.]|[a-z0-9.\-]+[.][a-z]{2,4}/)(?:[^\s()<>]+|\(([^\s()<>]+|(\([^\s()<>]+\)))*\))+(?:\(([^\s()<>]+|(\([^\s()<>]+\)))*\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))"#).unwrap();
    url_regex
        .captures_iter(input)
        .map(|capture| capture[1].to_string())
        .collect()
}

pub async fn better_wording(msg: String) -> Option<String> {
    if msg.chars().count() <= 3 {
        return None
    }
    let request_body = json::object! {
        "model": "gpt-3.5-turbo-0613",
        "messages": [
            {
                "role": "system",
                "content": "You are a bot that returns a sentence with opposite meaning. Change the wording as needed. You may only use names appearing in the latest message if any."
            },
            {
                "role": "user",
                "content": "<nimi> menee töihin"
            },
            {
                "role": "assistant",
                "content": "<nimi> ei mene töihin"
            },
            {
                "role": "user",
                "content": "laitat auton ostoon"
            },
            {
                "role": "assistant",
                "content": "et laita autoa ostoon"
            },
            {
                "role": "user",
                "content": "takaisin töihin"
            },
            {
                "role": "assistant",
                "content": "ei mennä takaisin töihin"
            },
            {
                "role": "user",
                "content": msg.clone()
            }
        ]
    };

    let client = Client::new();
    let token = get_config_value(EnvVariable::OpenAiToken);
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .body(request_body.to_string())
        .send()
        .await.expect("openai parsing failed");

    let body = response.text().await.unwrap();
    let parsed = json::parse(&body).unwrap();

    let result_result =
        &parsed["choices"][0]["message"]["content"].to_string();
    Some(result_result.to_string())
}
