package main

import (
	"os"

	"github.com/napuu/gpsp-bot/internal/platforms"
)

func main() {
	platforms.EnsureBotCanStart()
	switch os.Args[1] {
	case "telegram":
		platforms.RunTelegramBot()
	case "discord":
		platforms.RunDiscordBot()
	}
}
