## 🚀 Description / 描述
<!-- 
Please include a summary of the changes and the related issue. 
请提供关于这些更改和相关 issue 的简短总结。
-->

Fixes / 修复 # (issue number)

## 📋 Type of Change / 变更类型
<!-- Please check the one that applies to this PR / 请勾选适用的选项: -->

- [ ] 🐛 Bug fix (non-breaking change which fixes an issue) / 修复 Bug
- [ ] ✨ New feature (non-breaking change which adds functionality) / 新功能
- [ ] 🛠️ Breaking change (fix or feature that would cause existing functionality to not work as expected) / 破坏性变更
- [ ] 📚 Documentation update / 文档更新
- [ ] 🧹 Code refactor & cleanup / 代码重构与清理

## 📸 Screenshots (if applicable) / 截图 (如果适用)
<!-- 
If your change affects the UI, please provide before/after screenshots.
如果你的更改影响了 UI，请提供更改前后的截图。
-->

## ✅ Checklist / 检查清单
<!-- 
Please verify that you've completed the following / 请确认你已完成以下各项:
-->

- [ ] My code follows the style guidelines of this project / 我的代码遵循了本项目的代码风格
- [ ] I have performed a self-review of my own code / 我已经对自己的代码进行了自我审查
- [ ] I have commented my code, particularly in hard-to-understand areas / 我在代码难以理解的地方添加了注释
- [ ] I have made corresponding changes to the documentation (if needed) / 我已经更新了相关的文档 (如果需要)
- [ ] My changes generate no new warnings (e.g. `cargo clippy` and `pnpm lint` pass) / 我的更改没有产生新的警告
- [ ] Any dependent changes have been merged and published / 任何依赖的更改都已经合并发布

## 🧭 Provider Protocol Compliance / Provider 协议符合性
<!-- 
Required when this PR changes provider/proxy/usage/live-config behavior.
当 PR 涉及 provider/proxy/usage/live-config 行为时必填。
Reference / 参考:
- docs/architecture/provider-development-protocol.md
- docs/architecture/provider-checklist.md
-->

- [ ] N/A (this PR does not touch provider/proxy/usage/live-config paths)
- [ ] Provider logic remains inside adapter boundaries (no leakage into handlers)
- [ ] Runtime success does not depend on request-log DB writes
- [ ] Live config consistency path is covered (`switch/start/stop` where applicable)
- [ ] Usage parsing covers provider response format variants
- [ ] Added/updated tests for changed provider contract behavior
