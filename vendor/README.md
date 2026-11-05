# 工作区快照

`vendor/` 保存 adapter 独立构建所需的共享源码和工作区契约：

- `music-web-provider` 0.2.0 与主工作区 `packages/music-web-provider` 当前文件一致，包含 Host 1.1 的最终 URL、重定向、无通用大小上限和随机访问音频校验行为。
- `music-adapter-sdk` 2.0.0 来自主工作区 `packages/music-adapter-sdk`，契约版本为 2。
- `wit/host/host.wit` 来自 `packages/adapter-host/wit`，包为 `nons:host@1.1.0`。
- `wit/plugin/plugin.wit` 来自 `plugins/wit`，包为 `nons:plugin@1.0.0`。

根目录 `Cargo.lock` 固定完整 adapter 构建图；vendor 仅保存源文件，不保存 `target/` 或临时锁文件。更新共享实现或 WIT 时同步更新快照和根锁文件。

当前快照为 25 个源文件、126,937 字节；除构建目录和本说明文件外的源码树 SHA-256 为 `ecfac6060e38ca07cad24c0c4c7bf7dc76ea4a6d77740db0a20795636f94dab5`。
