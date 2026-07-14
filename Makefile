VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo dev)
LDFLAGS := -s -w -X main.version=$(VERSION)

.PHONY: build install test vet clean

build:
	go build -trimpath -ldflags '$(LDFLAGS)' -o pulse .

install:
	go install -trimpath -ldflags '$(LDFLAGS)' .

test:
	go test ./...

vet:
	go vet ./...

clean:
	rm -f pulse
