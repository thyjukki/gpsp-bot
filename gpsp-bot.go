package main

import (
	"log"
	"os"

	"github.com/napuu/gpsp-bot/internal/platforms"
)

func main() {
	platforms.EnsureBotCanStart()
	if len(os.Args) == 1 {
		log.Fatalf("Usage: gpsp-bot <telegram/discord>")
	}
	switch os.Args[1] {
	case "telegram":
		platforms.RunTelegramBot()
	case "discord":
		platforms.RunDiscordBot()
	}
}
