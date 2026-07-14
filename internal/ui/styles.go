// Package ui renders the pulse terminal interface.
package ui

// style is an ANSI SGR sequence. Basic colors only, so pulse inherits the
// user's terminal theme.
type style string

var (
	brand   = style("1;34") // bold blue
	dim     = style("90")   // labels, hints
	value   = style("")     // plain foreground
	ok      = style("32")
	okBold  = style("1;32")
	warn    = style("33")
	bad     = style("31")
	badBold = style("1;31")
	graphSt = style("34")
)

func (s style) Render(text string) string {
	if s == "" {
		return text
	}
	return "\x1b[" + string(s) + "m" + text + "\x1b[0m"
}
