**SnapCode v0.2.1** — 更新功能修复 / Updater fixes

## 修复 / Fixes

- **更新检查支持系统代理**：v0.2.0 的更新请求始终直连、不读取 Windows 系统代理设置，在需要代理才能访问 GitHub 的网络下检查更新必然失败；现已支持系统代理（TUN 模式用户不受影响）。
  **System proxy support**: v0.2.0's update check ignored the Windows system proxy and always connected directly, so it inevitably failed on networks that require a proxy to reach GitHub. The updater now honors the system proxy (TUN mode users are unaffected).
- **更新失败可见具体原因**：检查更新失败时，错误信息附带真实原因（HTTP 状态码或网络错误详情），不再只有笼统的「无法获取更新信息」，便于排查 404、代理拦截、连接失败等不同情况。
  **Actionable error messages**: a failed update check now shows the concrete cause (HTTP status or network error) instead of a generic "could not fetch release info", making it easy to tell a 404 from a proxy block or a connection failure.

## 说明 / Notes

- 检查更新需要能访问 GitHub Releases；若你的网络直连 GitHub 不稳定，请开启代理软件的「系统代理」或 TUN 模式。新版本刚发布后的几分钟内，`releases/latest` 可能仍指向旧版本而返回 404，稍后重试即可。
- Update checks need access to GitHub Releases; if direct access is unstable on your network, enable your proxy client's "system proxy" or TUN mode. For a few minutes right after a release is published, `releases/latest` may still point to the previous release and return 404 — just retry later.
