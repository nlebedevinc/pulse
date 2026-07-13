// Package ui renders the pulse terminal interface.
package ui

import "github.com/charmbracelet/lipgloss"

// Basic ANSI colors only, so pulse inherits the user's terminal theme.
var (
	accent = lipgloss.Color("4") // graph, brand
	green  = lipgloss.Color("2")
	yellow = lipgloss.Color("3")
	red    = lipgloss.Color("1")
	gray   = lipgloss.Color("8") // labels, hints

	brand   = lipgloss.NewStyle().Foreground(accent).Bold(true)
	dim     = lipgloss.NewStyle().Foreground(gray)
	value   = lipgloss.NewStyle()
	ok      = lipgloss.NewStyle().Foreground(green)
	warn    = lipgloss.NewStyle().Foreground(yellow)
	bad     = lipgloss.NewStyle().Foreground(red)
	graphSt = lipgloss.NewStyle().Foreground(accent)

	pad = lipgloss.NewStyle().Padding(0, 2)
)
