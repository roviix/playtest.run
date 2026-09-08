#!/usr/bin/env bash
# 一台全新的 Ubuntu 24.04 主机只跑一次（可重复跑，幂等）：Docker、swap、日志上限、目录。
# 用法（本机）：ssh playtest-hk 'bash -s' < deploy/provision.sh
set -euo pipefail

echo "== 系统更新"
sudo apt-get update -q
sudo DEBIAN_FRONTEND=noninteractive apt-get upgrade -y -q
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -q ca-certificates curl gnupg rsync sqlite3 unattended-upgrades

echo "== Docker（官方 apt 源，带 compose 与 buildx 插件）"
if ! command -v docker >/dev/null 2>&1; then
  sudo install -m 0755 -d /etc/apt/keyrings
  curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
  sudo chmod a+r /etc/apt/keyrings/docker.gpg
  echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "$VERSION_CODENAME") stable" \
    | sudo tee /etc/apt/sources.list.d/docker.list >/dev/null
  sudo apt-get update -q
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -q docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
fi
sudo usermod -aG docker "$USER"
# 容器日志默认就轮转，compose 里没写 logging 的服务也不会把盘写满。
sudo mkdir -p /etc/docker
if [[ ! -f /etc/docker/daemon.json ]]; then
  echo '{ "log-driver": "json-file", "log-opts": { "max-size": "10m", "max-file": "3" } }' | sudo tee /etc/docker/daemon.json >/dev/null
  sudo systemctl restart docker
fi

echo "== 2 GB swap（机器只有 2 GB 内存，给突发的上传与编译一点余量）"
if ! swapon --show | grep -q '^/swapfile'; then
  sudo fallocate -l 2G /swapfile
  sudo chmod 600 /swapfile
  sudo mkswap /swapfile >/dev/null
  sudo swapon /swapfile
  grep -q '^/swapfile' /etc/fstab || echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab >/dev/null
fi
echo 'vm.swappiness=10' | sudo tee /etc/sysctl.d/90-playtest.conf >/dev/null
sudo sysctl -q -p /etc/sysctl.d/90-playtest.conf

echo "== journald 上限"
sudo mkdir -p /etc/systemd/journald.conf.d
printf '[Journal]\nSystemMaxUse=200M\n' | sudo tee /etc/systemd/journald.conf.d/90-playtest.conf >/dev/null
sudo systemctl restart systemd-journald

echo "== 目录"
mkdir -p "$HOME/workspace/playtest.run/deploy/sites.active" "$HOME/db-backups/playtest"
# 容器里的 playtest 用户是 uid 10001；数据目录归它，api/edge 才写得进去。
sudo mkdir -p "$HOME/playtest-data"
sudo chown -R 10001:10001 "$HOME/playtest-data"

echo "== 备份 cron（每小时）"
# 新用户还没有 crontab 时 crontab -l 会报错、grep 会因没有输出而非零，都不算失败。
{ { crontab -l 2>/dev/null || true; } | grep -v 'playtest.run/deploy/backup.sh' || true; echo "17 * * * * $HOME/workspace/playtest.run/deploy/backup.sh >> $HOME/db-backups/playtest/backup.log 2>&1"; } | crontab -

echo "== 完成"
docker --version
sudo docker compose version
free -h | sed -n '1,3p'
df -h / | tail -1
echo "提示：docker 组权限对当前 ssh 会话不生效，重新登录后才能不带 sudo 跑 docker。"
