#!/bin/bash
# 真 Safari 能不能被 WebDriver 驱动？safaridriver 只有在
# `safaridriver --enable`（要管理员认证）之后才会接受会话，所以这一步单独探一下。
PORT=${1:-9317}
nohup safaridriver -p "$PORT" >/tmp/sd-probe.log 2>&1 &
SD=$!
sleep 3
code=$(curl -s -m 40 -X POST "http://127.0.0.1:$PORT/session" \
  -H 'Content-Type: application/json' \
  -d '{"capabilities":{"alwaysMatch":{"browserName":"safari"}}}' \
  -o /tmp/sd-session.json -w '%{http_code}')
echo "http=$code"
echo "--- 响应 ---"
head -c 900 /tmp/sd-session.json 2>/dev/null; echo
echo "--- driver 日志 ---"
cat /tmp/sd-probe.log
sid=$(python3 -c "import json;print(json.load(open('/tmp/sd-session.json'))['value'].get('sessionId',''))" 2>/dev/null)
[ -n "$sid" ] && curl -s -m 20 -X DELETE "http://127.0.0.1:$PORT/session/$sid" >/dev/null
kill "$SD" 2>/dev/null
pkill -f "safaridriver -p $PORT" 2>/dev/null
echo "cleaned"
