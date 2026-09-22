#!/usr/bin/env bash
set -euo pipefail
# Run on the Docker host, using an existing runtime image and home volume.
if [[ $# != 6 ]]; then
  echo 'usage: run-office-container.sh IMAGE HOME_VOLUME HOME_PATH UID:GID CHECKOUT CONFIG' >&2
  exit 2
fi
image_ref=$1
home_volume=$2
collector_home=$3
collector_user=$4
checkout_path=$5
config_path=$6
docker image inspect "$image_ref" >/dev/null
docker volume inspect "$home_volume" >/dev/null
if docker container inspect herdr-observatory >/dev/null 2>&1; then
  echo 'herdr-observatory already exists; inspect it before replacing the dashboard service.' >&2
  exit 1
fi
docker run -d --name herdr-observatory --restart unless-stopped \
  --network host --read-only --cap-drop ALL --security-opt no-new-privileges \
  --user "$collector_user" --env "HOME=$collector_home" --env PYTHONDONTWRITEBYTECODE=1 \
  --mount "type=volume,src=$home_volume,dst=$collector_home,readonly" \
  --workdir "$checkout_path" --entrypoint /usr/bin/python3 \
  "$image_ref" -m observatory --config "$config_path" --profile work
