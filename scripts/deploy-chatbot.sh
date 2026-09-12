#!/bin/bash
# Deploys a Langolier chatbot to a remote host over SSH, in Docker.
#
# The server binary is identical for every chatbot: only the .langolier file
# and the models change. The prebuilt image is reused, so a deployment takes
# under a minute with no recompilation.
#
#   LANGOLIER_SSH_HOST=myhost LANGOLIER_NAS_IP=10.0.0.2 \
#     ./deploy-chatbot.sh <kit-folder> [--name name] [--port 8788]
#
# To update a deployed chatbot, run the same command again. The .langolier is
# replaced, picked up in under 30 s, and conversations are kept.
set -euo pipefail

SSH_HOST="${LANGOLIER_SSH_HOST:-}"
NAS_IP="${LANGOLIER_NAS_IP:-}"
DOCKER_ROOT="${LANGOLIER_DOCKER_ROOT:-/volume1/docker}"
IMAGE="${LANGOLIER_IMAGE:-langolier-chatbot:latest}"
DOCKER="sudo -n /usr/local/bin/docker"
COMPOSE="sudo -n /usr/local/bin/docker-compose"

KIT=""; NAME=""; PORT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --name) NAME="$2"; shift 2 ;;
    --port) PORT="$2"; shift 2 ;;
    --host) NAS_IP="$2"; shift 2 ;;
    -h|--help) sed -n '2,18p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) KIT="$1"; shift ;;
  esac
done
[ -n "$SSH_HOST" ] || { echo "Set LANGOLIER_SSH_HOST (ssh alias of the target host)." >&2; exit 2; }
[ -n "$NAS_IP" ] || { echo "Set LANGOLIER_NAS_IP (address the chatbot listens on)." >&2; exit 2; }
[ -n "$KIT" ] || { echo "usage: $0 <dossier-du-kit> [--name nom] [--port 8788]" >&2; exit 2; }
[ -d "$KIT" ] || { echo "Folder not found: $KIT" >&2; exit 2; }

BUNDLE=$(find "$KIT" -maxdepth 1 -name '*.langolier' | head -1)
[ -n "$BUNDLE" ] || { echo "No .langolier file in $KIT" >&2; exit 2; }
[ -n "$NAME" ] || NAME=$(basename "$BUNDLE" .langolier)
# Safe container name: lowercase, digits, dashes.
NAME=$(echo "$NAME" | tr '[:upper:]' '[:lower:]' | tr -c 'a-z0-9-' '-' | sed 's/--*/-/g; s/^-//; s/-$//')

# Free port: the requested one, else the first available from 8788.
if [ -z "$PORT" ]; then
  USED=$(ssh "$SSH_HOST" "$DOCKER ps --format '{{.Names}}' | while read c; do $DOCKER inspect \$c --format '{{range .Config.Entrypoint}}{{.}} {{end}}'; done" 2>/dev/null | grep -oE '\-\-port [0-9]+' | awk '{print $2}' | sort -u)
  PORT=8788
  while echo "$USED" | grep -qx "$PORT"; do PORT=$((PORT + 1)); done
fi

DEST="$DOCKER_ROOT/$NAME"
echo "> Chatbot $NAME -> $DEST, port $PORT on $NAS_IP"

# The shared image must exist: build it once on the target host.
ssh "$SSH_HOST" "$DOCKER image inspect '$IMAGE' >/dev/null 2>&1" || {
  echo "Image '$IMAGE' missing on the host." >&2
  echo "Build it once: ssh $SSH_HOST \"cd $DOCKER_ROOT/<kit> && $DOCKER build --network host -t $IMAGE .\"" >&2
  exit 1
}

ssh "$SSH_HOST" "mkdir -p '$DEST/data/models' '$DEST/state'"

echo "> Sending the profile and the models"
# --no-xattrs: no ._ AppleDouble files on the target.
COPYFILE_DISABLE=1 tar --no-xattrs -C "$KIT" -cf - \
  "$(basename "$BUNDLE")" $([ -d "$KIT/models" ] && echo models) \
  | ssh "$SSH_HOST" "tar -xf - -C '$DEST/data'"

echo "> Writing docker-compose.yml"
ssh "$SSH_HOST" "cat > '$DEST/docker-compose.yml'" <<COMPOSE_EOF
version: "3.8"
services:
  $NAME:
    image: $IMAGE
    container_name: $NAME
    restart: unless-stopped
    # Host network: avoids firewall rules on Docker subnets and lets the
    # Telegram bridge reach out.
    network_mode: host
    entrypoint: ["langolier", "--serve", "--host", "$NAS_IP", "--port", "$PORT", "--bundle", "/data", "--data", "/state"]
    volumes:
      # Replace data/*.langolier to update live (< 30 s, conversations kept).
      - ./data:/data:ro
      - ./state:/state
    environment:
      - LANGOLIER_BUNDLE_DIR=/data
    mem_limit: 2g
    healthcheck:
      test: ["CMD", "langolier", "--health-check", "$NAS_IP:$PORT"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 30s
COMPOSE_EOF

echo "> Starting"
ssh "$SSH_HOST" "cd '$DEST' && $COMPOSE up -d"

echo "> Checking"
for i in $(seq 1 30); do
  if curl -fsS -m 5 "http://$NAS_IP:$PORT/api/health" >/dev/null 2>&1; then
    echo
    curl -s -m 5 "http://$NAS_IP:$PORT/api/config" \
      | python3 -c "import sys,json;d=json.load(sys.stdin);d.pop('avatar',None);print('  profile:',json.dumps(d,ensure_ascii=False))"
    curl -s -m 5 "http://$NAS_IP:$PORT/api/health" \
      | python3 -c "import sys,json;d=json.load(sys.stdin);print('  engine:',d['engine'],'| ok =',d['ok'],'|','; '.join(d['problems']) or 'no problem')"
    echo "  page   : http://$NAS_IP:$PORT/"
    exit 0
  fi
  sleep 2
done
echo "The chatbot did not answer within 60 s. Log:" >&2
ssh "$SSH_HOST" "$DOCKER logs --tail 30 '$NAME'" >&2
exit 1
