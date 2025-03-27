package handlers

import (
	"log/slog"

	"github.com/napuu/gpsp-bot/pkg/utils"
)

type VideoDownloadHandler struct {
	next ContextHandler
}

func (u *VideoDownloadHandler) Execute(m *Context) {
	slog.Debug("Entering VideoDownloadHandler")
	if m.action == DownloadVideo {
		var videoString = m.url
		path := utils.DownloadVideo(videoString, 5)

		m.originalVideoPath = path
		m.finalVideoPath = path
	}
	u.next.Execute(m)
}

func (u *VideoDownloadHandler) SetNext(next ContextHandler) {
	u.next = next
}
