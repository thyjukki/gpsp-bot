use std::process::Command;
use uuid::Uuid;

pub async fn download_video(url: String) -> Option<String> {
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
        // .arg("--rate-limit")
        // .arg("1.0M")

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

