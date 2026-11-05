# 更新日志

## Unreleased

- 适配器同步 contract 2：移除 manifest 网络宿主声明，网络目标、重定向、期限和最终 URL 统一由 Host 处理；请求期限为 0 并显式跟随重定向。
- 通用请求与响应不再设置固定大小上限，移除旧 64KiB／128KiB 与后续 12MiB 限制；音频仍使用有限 Range 探测和解析预算，不因资源总大小超过 512MiB 拒绝。
- 首页、POST 与音频探测使用 Host 返回的最终 URL，交付的播放地址同样取最终 URL。
- 可恢复调用错误按稳定 stage／class／code 写入宿主诊断日志。
- 当前 provider、SDK 与 Host／插件 WIT 快照随包保存，干净检出仍按 `Cargo.lock` 构建。

- 项目代码使用 MIT 许可证，顶层许可证文件统一命名为 License，保留三方许可原文。

- Release ZIP 根目录提供适配器 manifest 和 Component；musicdl 及依赖许可统一命名为 License。
