# Contributing / 参与贡献

## Pull requests

Create a topic branch or fork, make the change, and open a pull request against `main`. All code, documentation, dependency, and workflow changes use this process, including maintainer changes. Do not push directly to `main` or bypass its protections.

Required checks must pass and review conversations must be resolved before a squash merge. No second-person approval is currently required, so a solo maintainer can merge a tested PR. This does not exempt a maintainer from using a PR.

Describe the problem, resulting behavior, and relevant verification. Use synthetic fixtures. Do not commit media, real task reports, personal paths, secrets, local settings, generated builds, or metadata from a user's card. Keep Chinese and English UI strings and READMEs aligned.

Run the relevant commands in the README. Changes to copying, recovery, paths, time interpretation, or verification need focused regression coverage. Browser tests alone cannot validate native I/O. Do not describe compilation as proof of real hardware behavior.

Contributions are licensed under GPL-3.0-only, the same terms as the project, unless separately agreed in writing. Contributors retain their copyright. Submitting a PR does not transfer copyright or grant a separate proprietary license. Include the origin and license of third-party material.

## 中文说明

从新分支或 fork 提交 PR，目标为 `main`。源码、文档、依赖和工作流修改均通过 PR，维护者也一样，不直接推送主分支或绕过保护。

自动检查通过、讨论解决后使用 squash 合并。目前不强制第二人批准，便于个人维护，但仍必须经过 PR。

PR 请说明问题、修改后的行为和验证结果。使用示例测试数据，不提交真实素材、任务报告、个人路径、密钥、本地设置、构建产物或卡上元数据。中英文界面与 README 保持一致。

按 README 运行相关检查。复制、恢复、路径、时间解释和校验逻辑的修改应有针对性的回归测试；浏览器测试不能代替原生读写验证，编译通过也不等于实机验证。

除非另有书面约定，贡献采用与项目一致的 GPL-3.0-only。贡献者保留版权；提交 PR 不代表转让版权，也不自动授予额外的闭源商业授权。引入第三方内容时需说明来源及许可证。
