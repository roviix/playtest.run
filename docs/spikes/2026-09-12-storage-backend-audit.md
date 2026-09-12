# 2026-09-12 · 线上物料主存储核对

## 问题与范围

回答「现在物料是不是存在 S3 上」。本次仅只读检查香港服务器正在运行的 API、边缘容器挂载，以及宿主机存储目录所在的文件系统；不修改服务器，不发布，不重新上传作品。

## 真机观察

在本机运行：

```sh
ssh -o BatchMode=yes -o ConnectTimeout=8 playtest-hk 'cd "$HOME/workspace/playtest.run/deploy" && docker compose ps -q api edge | xargs -r docker inspect --format "{{.Name}} {{.Config.Image}} {{range .Mounts}}{{.Source}} -> {{.Destination}} ({{.Type}}) {{end}}"; if [ -d "$HOME/playtest-data/store" ]; then findmnt -T "$HOME/playtest-data/store" -o TARGET,SOURCE,FSTYPE; fi'
```

退出码为 0，输出：

```text
/playtest-api-1 playtest-server:20260909-134304 /home/ubuntu/playtest-data -> /data (bind)
/playtest-edge-1 playtest-server:20260909-134304 /home/ubuntu/playtest-data -> /data (bind)
TARGET SOURCE         FSTYPE
/      /dev/nvme0n1p1 ext4
```

API 与边缘共用宿主机 `/home/ubuntu/playtest-data`，容器内为 `/data`。宿主机 `store/` 所在文件系统是本机块设备上的 ext4，并非 S3 挂载。

## 与当前仓库实现对照

- `common/src/store.rs` 的实现是 `FsStore`：文件内容按哈希存为 `blobs/<前两位>/<hash>`，版本清单和当前版本指针也在同一个存储根下。
- `api/src/state.rs` 的 `from_config` 直接构造 `FsStore`；`api/src/config.rs` 将存储根设为数据目录下的 `store/`，SQLite 设为同目录下的 `api.sqlite`。
- `deploy/compose.yaml` 给 API、边缘配置相同的 `/data` 和宿主机挂载，与上述运行容器一致。
- `docs/DESIGN.md` §4.2 的香港区域 S3 兼容对象存储是目标架构，不能据此声称当前主存储已经接入 S3。

## 结论与未检查项

当前仓库实现及线上挂载证据指向服务器本地磁盘作为物料主存储，不是已接入的 S3 对象服务。「按对象键组织文件」不等于「使用 S3」。

这不是耐久性或灾备验收：未核对 EBS 快照、额外备份、S3 备份桶及恢复能力。线上镜像标签如上，未核对其构建来源是否等于当前工作区。未测试视频播放或文章发布，不据此宣称这些形态已支持。
