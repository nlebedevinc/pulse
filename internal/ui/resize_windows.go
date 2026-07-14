//go:build windows

package ui

import "os"

// Windows has no SIGWINCH; the layout is fixed at startup width.
func notifyResize(ch chan os.Signal) {}
