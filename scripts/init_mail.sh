#!/usr/bin/env bash
set -x
set -eo pipefail

# Launch mailhog using Docker
# Allow to skip Docker if a dockerized Mailhog is already running
if [[ -z "${SKIP_DOCKER}" ]]
then
    docker run -p 8025:8025 -p 1025:1025 -d \
        mailhog/mailhog
fi
