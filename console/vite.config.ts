import { defineConfig, loadEnv } from "vite";

// JSX 走 preact 的自动运行时，配置在 tsconfig.json 里，esbuild 照着读，不用额外插件。
export default defineConfig(({ mode }) => {
  // 开发时把 /v1 转给本机的控制面。端口用 PLAYTEST_API 覆盖：
  //   PLAYTEST_API=http://127.0.0.1:48787 pnpm dev
  // 构建出来的静态站不带代理，那时 API 地址由 VITE_PLAYTEST_API 决定（见 src/api.ts）。
  const env = loadEnv(mode, ".", ["PLAYTEST_", "VITE_"]);
  const apiOrigin = env.PLAYTEST_API || "http://127.0.0.1:8787";

  return {
    // 线上挂在控制面域名的 /console/ 下（deploy/sites/api.caddy）；开发时仍是根路径。
    base: mode === "production" ? "/console/" : "/",
    server: {
      port: 5273,
      proxy: {
        "/v1": { target: apiOrigin, changeOrigin: true },
      },
    },
    build: {
      target: "es2022",
    },
  };
});
