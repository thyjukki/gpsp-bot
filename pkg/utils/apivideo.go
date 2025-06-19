package utils

import (
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"os"
	"time"

	apivideosdk "github.com/apivideo/api.video-go-client"
	"github.com/napuu/gpsp-bot/internal/config"
)

func secondsToTimecode(seconds float64) string {
	hours := int(seconds / 3600)
	minutes := int((seconds - float64(hours*3600)) / 60)
	secs := int(seconds - float64(hours*3600) - float64(minutes*60))
	return fmt.Sprintf("%02d:%02d:%02d", hours, minutes, secs)
}

func downloadFile(url string, filepath string) error {
	resp, err := http.Get(url)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	out, err := os.Create(filepath)
	if err != nil {
		return err
	}
	defer out.Close()

	_, err = io.Copy(out, resp.Body)
	return err
}

func UploadAndCutVideo(filePath string, startSeconds float64, durationSeconds float64) (string, error) {
	cfg := config.FromEnv()
	client := apivideosdk.ClientBuilder(cfg.APIVIDEO_API_KEY).Build()

	videoTitle := "gpsp-bot-upload-" + time.Now().String()

	payload := apivideosdk.VideoCreationPayload{
		Title: videoTitle,
	}

	if startSeconds > 0 && durationSeconds > 0 {
		startTime := secondsToTimecode(startSeconds)
		endTime := secondsToTimecode(startSeconds + durationSeconds)
		payload.Clip = &apivideosdk.VideoClip{
			StartTimecode: &startTime,
			EndTimecode:   &endTime,
		}
	}

	videoObject, err := client.Videos.Create(payload)
	if err != nil {
		return "", fmt.Errorf("failed to create video object: %w", err)
	}

	videoID := videoObject.VideoId

	slog.Info("Uploading video to api.video", "videoID", videoID)

	videoFile, err := os.Open(filePath)
	if err != nil {
		return "", fmt.Errorf("failed to open video file: %w", err)
	}
	defer videoFile.Close()

	_, err = client.Videos.UploadFile(videoID, videoFile)
	if err != nil {
		// Best effort to delete the video object if upload fails
		delErr := client.Videos.Delete(videoID)
		if delErr != nil {
			slog.Error("Failed to delete video object after upload failure", "videoID", videoID, "error", delErr)
		}
		return "", fmt.Errorf("failed to upload video file: %w", err)
	}

	slog.Info("Waiting for video to be processed", "videoID", videoID)
	// Poll for video status
	for {
		status, err := client.Videos.GetStatus(videoID)
		if err != nil {
			return "", fmt.Errorf("failed to get video status: %w", err)
		}

		if *status.Encoding.Playable {
			slog.Info("Video processing complete", "videoID", videoID)
			break
		}
		slog.Info("Video not ready yet, waiting...", "videoID", videoID)
		time.Sleep(1 * time.Second)
	}

	// Get the video object again to have asset information
	processedVideo, err := client.Videos.Get(videoID)
	if err != nil {
		return "", fmt.Errorf("failed to get processed video object: %w", err)
	}

	if processedVideo.Assets == nil || processedVideo.Assets.Mp4 == nil {
		return "", fmt.Errorf("no mp4 asset found for video")
	}

	mp4Url := *processedVideo.Assets.Mp4
	slog.Info("Downloading processed video", "url", mp4Url)

	tempFilePath := filePath + ".processed.mp4"
	err = downloadFile(mp4Url, tempFilePath)
	if err != nil {
		return "", fmt.Errorf("failed to download processed video: %w", err)
	}

	// Delete the video from api.video
	slog.Info("Deleting video from api.video", "videoID", videoID)
	err = client.Videos.Delete(videoID)
	if err != nil {
		slog.Error("Failed to delete video from api.video", "videoID", videoID, "error", err)
		// Don't fail the whole operation if deletion fails, just log it.
	}

	return tempFilePath, nil
}
