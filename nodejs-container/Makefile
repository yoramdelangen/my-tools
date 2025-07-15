.PHONY: build

build:
	CGO_ENABLED=0 go build -o nodejs-container
	ln -s ${PWD}/nodejs-container ${HOME}/.bin/nodejs
