#!/bin/bash
docker build . -t gengar 
docker compose down --remove-orphans
docker compose up -d
