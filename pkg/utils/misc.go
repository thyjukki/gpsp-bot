package utils

import (
	"fmt"
	"log/slog"
	"os"
	"os/exec"
	"strconv"
)

// Unsafe conversion. Mainly used for mapping chat ids back and forth
// as discord and telebot are using strings and integres respectively.
func S2I(s string) int {
	i, _ := strconv.Atoi(s)
	return i
}

func EnsureTmpDirExists(tmpDir string) {
	err := os.MkdirAll(tmpDir, 0755)
	if err != nil {
		panic(fmt.Sprintf("Couldn't create tmp dir for yt-dlp, %s", err))
	}
}

func CleanupTmpDir(tmpDir string) {
	cmd := exec.Command("find", tmpDir, "-type", "f", "-mtime", "+2", "-delete")
	err := cmd.Run()
	if err != nil {
		slog.Error(fmt.Sprintf("Error cleaning up tmp dir %s: %v\n", tmpDir, err))
	} else {
		slog.Info(fmt.Sprintf("Cleaned up files older than 2 days in %s\n", tmpDir))
	}
}
