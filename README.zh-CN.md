<p align="center">
  <img src="branding/stickers/bug-in-a-jar.webp" alt="紫色千足虫 Spirolo，手里拿着一个装有虫子的罐子" width="520">
</p>

# Spirolo

[English](README.md) | 简体中文

[![CI](https://github.com/spirolo/spirolo/actions/workflows/ci.yml/badge.svg)](https://github.com/spirolo/spirolo/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/github/license/spirolo/spirolo)](LICENSE)
[![npm](https://img.shields.io/npm/v/spirolo)](https://www.npmjs.com/package/spirolo)

Spirolo 是面向 JavaScript 和 TypeScript 测试套件的静态分析器。

它结合测试代码、源代码和测试运行器配置，查找缺失的初始化防护、被跳过却未关联问题跟踪记录的测试、重复测试，以及可能在未实际检验任何内容的情况下通过的断言。分析由 Rust 二进制程序在本地完成，不会运行你的测试，也不会将代码发送到任何地方。

## Spirolo 不会做什么

- 不会运行你的测试或执行你的代码。
- 不会修复或重写源文件。
- 默认不会导致 CI 失败：在你提高规则的严重级别之前，发现项均为提示级（advisory）。
- 不会将代码发送到任何地方：分析无需访问网络。
- 仅覆盖 JavaScript 和 TypeScript 测试套件。

## 安装

需要 Node 20 或更高版本。在项目根目录运行：

```sh
npm install -D spirolo@alpha
npx spirolo check
```

无需配置 Spirolo。请先安装项目依赖，以便解析项目的 TypeScript 配置和导入。

npm 启动器会选择适用于 Linux 或 macOS（x64 和 arm64）的原生平台包，无需在 postinstall 阶段下载。

也可以使用当前稳定版 Rust 工具链，从源码构建。在本仓库根目录运行：

```sh
cargo build --release --locked -p spirolo
./target/release/spirolo check /path/to/your/project
```

## 一次实际运行

以下输出来自基于 Spirolo 提交 `4a8d80cd` 构建的 release 二进制程序，分析对象是 [Kaneo](https://github.com/usekaneo/kaneo) 的提交 [`afe97be2473e0cd97fe4a63e526fef9ac4df85b0`](https://github.com/usekaneo/kaneo/tree/afe97be2473e0cd97fe4a63e526fef9ac4df85b0)，且已安装该项目的依赖。将 `KANEO` 设为检出该提交的目录路径，然后在 Spirolo 源码目录运行：

```sh
./target/release/spirolo check "$KANEO"
```

以下为输出节选，`[...]` 表示省略的行。耗时仅来自这次运行，并非基准测试结果。

```text
1754 files · 351 test files · 1797 tests (1797 vitest) · 4309 expects · 0 handlers · 35 configs · 253ms
Health C (60)
[...]
● fail-closed-setup (10 files, 19 findings) · advisory: 19/19 (UNADJUDICATED)
  apps/api/vitest.config.ts (2)
    :0 setup does not guard the console (no throwing console.error/warn spy or fail-on-console package)
    :0 setup does not restore mocks between tests (no restoreMocks or afterEach restoreAllMocks)
[...]
34 findings in 3 of 16 rules · 0 warn · 0 error · 34 info · exit 0
 hidden 1226 findings: 1211 experimental (gate-truth 11, unused-mock 3, flake-candidate 1, untested-hotspot 1196), 15 from DROPPED or FAIL rules (weak-assertion 15); spirolo check --all shows them
 note   no churn input: untested-hotspot ranks by complexity × weakness alone. Save `git log --numstat --format= --since=180.days` to a file and pass it with `--churn <file>` or name it in `inputs.churn`
```

这次运行报告了缺失的初始化防护、仅检查内容不存在的断言，以及一处可能的重复测试。它没有执行 Kaneo 的测试，也没有逐一验证发现项。命令成功退出，是因为这些发现项均为提示级。隐藏的发现项数量包含未使用代码变更量数据的实验性（experimental）热点排名，并不代表已确认的缺陷数量。

## 能发现什么

默认报告会显示以下提示级规则：

| 规则 | 需要检查的内容 |
| --- | --- |
| `absence-anchor` | 测试只检查某些内容不存在，却没有检查预期内容是否已渲染。 |
| `dead-skip` | 测试被跳过，却未关联问题跟踪记录；或被设为独占运行，导致其他测试不会运行。 |
| `duplicate-test` | 测试主体及其建模后的初始化逻辑看起来等价。删除其中任何一个测试之前，请检查它们是否覆盖了不同的行为。 |
| `fail-closed-setup` | 测试运行器的初始化缺少针对 mock 清理、控制台错误或未处理请求的防护。 |

`check --all` 还会显示实验性规则，以及测量未通过或已被弃用的规则。这些规则包括合并门禁检查、静态分析发现的疑似不稳定测试、未使用的 mock，以及按代码变更量、复杂度和测试强度排序的源代码模块。使用 `spirolo explain <rule>` 可查看规则声称要检测的内容及其测量状态。

### 为什么还需要一个测试工具？

测试 lint 插件检查测试代码中的模式。覆盖率报告告诉你哪些代码被执行过。Stryker 等变异测试工具检查测试能否发现生产代码的改动。Spirolo 则结合测试、导入、初始化和测试运行器配置进行分析，也可以读取已有的覆盖率、变异测试和运行报告。请继续运行测试并使用这些工具：静态分析的发现项回答的是不同的问题。

[工具对比](docs/landscape.md)（英文）解释了这种分工。

## 可以信任到什么程度

**目前还没有任何规则达到已验证（proven）级别。** 以下测量结果记录的是抽样发现项被判定为正确的比例，并不说明 Spirolo 能在你的项目中发现多少缺陷。

- **提示级：** 规则至少有 15 个已判定的发现项，且测得的精确率至少为 0.80 时，默认显示。这些发现项以 `info` 严重级别报告。
- **实验性：** 除非你主动启用该规则，否则仅在使用 `--all` 时显示发现项。现有证据过于有限，或测得的精确率低于提示级的最低要求。
- **FAIL 或 DROPPED：** 仅在使用 `--all` 时显示发现项，且始终不会触发门禁，即使覆盖严重级别也不例外。
- **已验证：** 仅适用于达到测量标准并通过一次新的留出集检验的规则。目前尚无规则达到这一级别。

目前，发现项默认不会导致命令失败。要让某条规则触发门禁，可在 `.spirolorc.jsonc` 中将其严重级别设为 `warn` 或 `error`，但必须在该规则允许的严重级别范围内。例如：

```jsonc
{ "rules": { "dead-skip": { "severity": "warn" } } }
```

[规则分级](docs/tiers.md)（英文）说明了阈值、标签、退出码和覆盖设置的限制。`spirolo rule-status` 会输出内嵌于你所用二进制程序中的测量结果。

### 测得的精确率

此表根据[内嵌测量数据](corpus/rule-status.json)生成，仅使用公开仓库。小样本上的满分结果并不能作为证明；置信区间反映了其中的不确定性。这些测量结果对应所记录的提交，并不代表对后续每次改动都重新进行了评估。`n/a` 表示没有已判定的发现项，因此无法估计精确率。

<!-- precision:start -->
测量基于 Spirolo 提交 `939f0af519fd50318e092700b74094b71cfeca23`，覆盖 40 个开源项目开发集条目。
提示级：至少有 15 个已判定的发现项，且表中精确率至少为 0.80。

| 规则 | 级别 | 状态 | 判定为正确 / 已判定 | 精确率 | 95% Wilson 置信区间 | 测量条目数 |
| --- | --- | --- | ---: | ---: | --- | ---: |
| absence-anchor | advisory | UNADJUDICATED | 16 / 20 | 0.800 | 0.584–0.919 | 12 |
| dead-skip | advisory | INSUFFICIENT | 18 / 18 | 1.000 | 0.824–1.000 | 7 |
| duplicate-test | advisory | UNADJUDICATED | 15 / 18 | 0.833 | 0.608–0.942 | 19 |
| fail-closed-setup | advisory | UNADJUDICATED | 19 / 19 | 1.000 | 0.832–1.000 | 10 |
| flake-candidate | experimental | INSUFFICIENT | 3 / 6 | 0.500 | 0.188–0.812 | 7 |
| gate-truth | experimental | UNADJUDICATED | 12 / 18 | 0.667 | 0.437–0.837 | 8 |
| untested-hotspot | experimental | UNADJUDICATED | 5 / 16 | 0.313 | 0.142–0.556 | 40 |
| unused-mock | experimental | INSUFFICIENT | 0 / 0 | n/a | n/a | 1 |
| data-layer-mock | hidden | DROPPED | 0 / 0 | n/a | n/a | 0 |
| flake-observed | hidden | FAIL | 39 / 52 | 0.750 | 0.618–0.848 | 4 |
| generator-walk | hidden | DROPPED | 0 / 0 | n/a | n/a | 0 |
| handler-drift | hidden | DROPPED | 0 / 0 | n/a | n/a | 0 |
| orphan-test | hidden | DROPPED | 4 / 14 | 0.286 | 0.117–0.546 | 12 |
| redundant-test | hidden | DROPPED | 0 / 12 | 0.000 | 0.000–0.243 | 5 |
| stale-snapshot | hidden | DROPPED | 0 / 0 | n/a | n/a | 3 |
| weak-assertion | hidden | DROPPED | 12 / 34 | 0.353 | 0.215–0.521 | 17 |
<!-- precision:end -->
以上数值与英文 README 保持一致，以英文 README 为准。

## 运行环境

原生包面向使用 glibc 的 Linux 和 macOS，支持 x64 和 arm64。Windows 和 Alpine/musl 不在 alpha 版本的目标平台范围内。原生二进制程序在分析时既不需要 Node，也不需要访问网络。

已测量的范围是使用 Vitest 或 Jest 的 JavaScript 和 TypeScript React 项目。Spirolo 也会读取 Playwright 配置和测试模式，但这并不意味着所有框架或配置都经过了测量。

你可以在本地使用 CLI，在 CI 中使用 [Markdown 摘要或 GitHub 注解](docs/ci-reports.md)（英文），也可以在智能体客户端中使用 [MCP](docs/mcp.md)（英文）。通过 `--format json` 和 `--format sarif` 可输出 JSON 和 SARIF 报告。使用同一二进制程序的 VS Code 扩展也可以[在本地打包并安装](docs/release.md)（英文）。

## 功能范围

Spirolo 不会运行测试、启动变异测试引擎、收集覆盖率、生成测试或托管运行历史。它读取你已有的产物。缺少产物时，会将相应上下文标记为未验证，而不会将其视为测试套件正常工作的证据。

静态分析发现的疑似不稳定测试，不等于实际观察到的不稳定测试。重复测试发现项只是提示你检查这些测试，并不意味着可以删除它们。暂定的健康评分只是对各项信号的汇总，并不保证正确性。

Vue、Cypress、其他语言及任意非 React 框架均不在已测量范围内。

## 当前状态与限制

**版本 `0.1.0-alpha.1`：提示级 alpha 版本。** 已在 npm 上以 `spirolo@alpha` 发布；VS Code 扩展尚未发布。在 1.0 之前，任何 alpha 版本都可能更改命令、输出和规则行为；[更新日志](CHANGELOG.md)（英文）列出了每项变更。

静态分析可能遗漏动态配置和未建模的模式。如果 TypeScript 配置无法加载，Spirolo 会报告模块解析问题，并将受影响的发现项标记为不可靠。在依据这些条目作出判断前，请安装缺失的依赖并解决配置警告。

面向用户阅读的报告会限制所显示的文件和发现项数量。`spirolo check --rule <rule> --all` 可显示该规则的完整报告，包括实验性或已弃用规则的发现项。在主动将规则用于 CI 之前，请先审查证据。

欢迎贡献：请参阅 [CONTRIBUTING.md](CONTRIBUTING.md)（英文）。
如需报告安全漏洞，请参阅 [SECURITY.md](SECURITY.md)（英文）。

## 许可证

[Apache 2.0](LICENSE)（英文）。
