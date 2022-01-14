#!/bin/sh

REPO="aitjcize/crypto-quote-server"
docker build -t ${REPO}:latest .
docker push ${REPO}:latest
