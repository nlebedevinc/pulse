package ui

import (
	"time"

	tea "github.com/charmbracelet/bubbletea"
)

// spinFrames is a minimal braille spinner, rendered dim.
var spinFrames = []string{"⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"}

type spinTickMsg struct{}

func spinTick() tea.Cmd {
	return tea.Tick(80*time.Millisecond, func(time.Time) tea.Msg {
		return spinTickMsg{}
	})
}
