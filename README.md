# 小白音乐备用音源适配器

来源 `xiaobai`，以 Rust WASM Component 接入统一 AdapterManager，随应用构建并在适配器设置中独立启停。为网易云、QQ 音乐、酷狗和酷我按已有歌曲 ID 解析音频，不参与搜索，不提供账号或歌词能力；曲目身份保持原平台。

协议依据 [musicdl 的 XiaoBaiMusicClient](https://github.com/CharlesPikachu/musicdl/blob/master/musicdl/modules/common/xiaobai.py)，站点为 `https://music.90svip.cn/`。使用共享网页请求和音频校验；本仓 `vendor/` 保存当前 provider、SDK、Host 与插件 WIT 快照，使 `--locked` 构建不依赖尚未合并的主仓改动。网络目标不限制主机、IP、端口、URL 凭证或通用请求与响应大小，manifest 不再声明宿主列表。所有 Host 请求期限为 `0`（无固定期限）并显式跟随重定向；首页、POST 响应和音频探测都继续使用 Host 返回的最终 URL。音频解析使用有限 Range 探测，不因资源总大小拒绝；完整时长和真实音质仍须通过检查，试听与未知完整性拒绝，Hi-Res 首选但返回有损时，只有开启允许音质降级才接受。公开 CDN 音频以不附加站点自定义头的请求探测，通过后交付最终 URI 播放管线。实际可用性取决于上游及歌曲。

```powershell
pwsh -WorkingDirectory packages/wasm-crypto -Command 'npm ci --ignore-scripts'
cargo test --manifest-path Cargo.toml --locked
cargo test --manifest-path vendor/music-web-provider/Cargo.toml
node scripts/build.mjs
node --test tests/component.test.mjs
```

WIT、provider 与 SDK 从仓库内 `vendor/` 读取；component harness 和 bundled 产物仍需要完整工作区，在非标准检出位置可用 `NONS_ROOT` 指向该工作区。

构建需要 `cargo-component` 和 Rust `wasm32-unknown-unknown` 目标，产物为 `dist/adapter.component.wasm` 并复制到应用的 bundled-adapters。上游许可见 licenses/musicdl/License，构建同时生成依赖许可告知。

真实宿主在线验收（构建小白音乐 Component 后运行）：

```powershell
./Nons/scripts/native.ps1 -Task test -ExtraArgs @('--lib','--locked','xiaobai_component_live_lemon','--','--ignored','--nocapture')
```

推送到 `main` 或提交 Pull Request 会运行 GitHub Actions；推送 `v*` 标签会在测试和 Component 构建通过后发布可直接安装的 ZIP 及 SHA-256 校验文件。

该验收使用独立临时数据库及空账号，检查小白音乐对 QQ 版 Lemon（`000akynZ2Rbro5`）的真实 WASM／Host 调用、实际提供器归属、320 kbps 完整资源，以及禁止降级时拒绝 Hi-Res 请求。它不代表全部歌曲、其他三平台或桌面出声已通过实机验收。

## 项目许可证

本项目代码采用 [MIT](License)。musicdl 的 PolyForm Noncommercial 1.0.0 条款保留在 `licenses/musicdl/License`，构建依赖告知位于产物的 `licenses/dependencies/License`；这些上游条款不被项目 MIT 授权替代。
