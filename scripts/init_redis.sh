#!/usr/bin/env bash
set -x
set -eo pipefail
# if a container is already running, print instructions to kill it and exit
RUNNING_CONTAINER=$(docker ps --filter 'name=redis' --format '{{.ID}}')
if [[ -n $RUNNING_CONTAINER ]]; then
echo >&2 "there is a redis container already running, kill it with"
echo >&2 "
docker kill ${RUNNING_CONTAINER}"
exit 1
fi
# Launch Redis using Docker
if [[ -z "${SKIP_DOCKER}" ]]
then
    docker run \
    --name "redis_$(date '+%s')" \
    -p "6379:6379" \
    -d \
    redis:7
    >&2 echo "Redis is ready to go!"
fi