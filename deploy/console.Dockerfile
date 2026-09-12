# syntax=docker/dockerfile:1
#
# 控制台是一个纯静态站（Preact + Vite），构建产物由 Caddy 直接托管在控制面域名下的 /console/。
# 在服务器上构建，产物拷进一个只装 Caddy 的镜像的卷里——这样 Caddy 镜像本身不用重打。
#
#   docker build -f deploy/console.Dockerfile -t playtest-console:<tag> .

FROM node:22-slim AS build
RUN corepack enable && corepack prepare pnpm@11 --activate
WORKDIR /src/console
COPY console/package.json console/pnpm-lock.yaml console/pnpm-workspace.yaml ./
RUN --mount=type=cache,target=/root/.local/share/pnpm/store pnpm install --frozen-lockfile
COPY console/ ./
COPY ui/ /src/ui/
# API 地址留空 = 同源相对路径：控制台和 api 都在 API_HOST 这一个域名下（AGENTS 第 7 条）。
ENV VITE_PLAYTEST_API=
RUN pnpm build

# 只留产物：compose 用这个镜像把 /srv/console 拷到共享卷，Caddy 从卷里读。
FROM busybox:1.36
COPY --from=build /src/console/dist /srv/console
CMD ["sh", "-c", "rm -rf /out/* && cp -r /srv/console/. /out/ && echo 控制台静态文件已就位"]
