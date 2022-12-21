#!/usr/bin/env bash
set -x
set -eo pipefail
# if a container is already running, print instructions to kill it and exit
RUNNING_CONTAINER=$(docker ps --filter 'name=mailhog' --format '{{.ID}}')
if [[ -n $RUNNING_CONTAINER ]]; then
    echo >&2 "there is a MailHog container already running, kill it with"
    echo >&2 "
        docker kill ${RUNNING_CONTAINER}"
    exit 1
fi

# Launch mailhog using Docker
# Allow to skip Docker if a dockerized Mailhog is already running
if [[ -z "${SKIP_DOCKER}" ]]
then
    docker run --name "mailhog_$(date '+%s')" -p 8025:8025 -p 1025:1025 -d \
        mailhog/mailhog
fi
