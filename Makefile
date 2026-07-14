VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo dev)
LDFLAGS := -s -w -X main.version=$(VERSION)
# -l disables inlining: ~8% smaller binary, and pulse is I/O-bound anyway
GCFLAGS := all=-l

.PHONY: build install test vet clean

build:
	go build -trimpath -ldflags '$(LDFLAGS)' -gcflags '$(GCFLAGS)' -o pulse .

install:
	go install -trimpath -ldflags '$(LDFLAGS)' -gcflags '$(GCFLAGS)' .

test:
	go test ./...

vet:
	go vet ./...

clean:
	rm -f pulse
