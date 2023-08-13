use lazy_static::lazy_static;
use log::debug;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use uuid::Uuid;

pub async fn download_video(url: String) -> Option<String> {
    let video_id = Uuid::new_v4();
    let file_path = format!("/tmp/{}.mp4", video_id);
    let output = Command::new("yt-dlp")
        .arg("-S")
        .arg("res:720,+size,+br,+res,+fps")
        .arg("--max-filesize")
        .arg("48M") // TG max is 50M
        .arg("--recode")
        .arg("mp4")
        .arg("-o")
        .arg(&file_path)
        .arg(url)
        // for debugging
        // .arg("--rate-limit")
        // .arg("1.0M")
        .output()
        .expect("failed to execute process");
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

pub fn delete_file(file: &str) -> Result<(), std::io::Error> {
    fs::remove_file(file)?;
    Ok(())
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

pub fn get_config_value(env_variable: EnvVariable) -> String {
    let (env_variable_name, default_value) = env_variable.get();
    let env_variable_name_file = env::var(format!("{}_FILE", env_variable_name))
        .unwrap_or("/dev/null/nonexistent".to_string());

    if let Some(cached_value) = CONFIG_VALUES.lock().unwrap().get(env_variable_name) {
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
    } else if let Ok(env_variable_file_content) = fs::read_to_string(env_variable_name_file.clone())
    {
        store_value(env_variable_name, &env_variable_file_content);
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
