use std::process::{Command, Stdio};
use uuid::Uuid;
use std::fs;

pub async fn download_video(url: String) -> Option<String> {
    let video_id = Uuid::new_v4();
    let file_path = format!("/tmp/{}.mp4", video_id);
    let output = Command::new("yt-dlp")
        .arg("-S")
        .arg("+size,+br,+res,+fps")

        .arg("--max-filesize")
        .arg("48M") // TG max is 50M
        
        // Filter videos above certain bitrate, fallback to best available 
        // video. Usually fallback is used only for direct links when no multiple
        // formats would have been available anyway.
        .arg("-f")
        .arg("[vbr>400] / b")
        
        // .arg("-f")
        // .arg("filesize<=45M")


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

pub fn delete_file(file: &str) -> Result<(), std::io::Error> {
    fs::remove_file(file)?;
    Ok(())
}
