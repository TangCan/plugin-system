---
title: 'technical research: Rust in-process plugin framework post-0.1 improvements'
type: 'technical'
topic: 'Rust in-process plugin framework post-0.1 improvements'
decision: 'Prioritize next improvements after shipping an in-process Rust plugin crate at 0.1.x'
source: 'native-run'
status: complete
preset: 'standard'
validation: 'normal'
created: '2026-08-18'
updated: '2026-08-18'
verified_claims: 8
unverified_claims: 3
---

# technical research: Rust in-process plugin framework post-0.1 improvements

**Decision this research serves:** 0.1.x 已上 crates.io 之后，下一步该改什么、按什么顺序改。

## 执行摘要

下一步应做**发布与文档质量**，而不是新的加载原语。证据指向三件事：为后续版本接上 Trusted Publishing；用 cargo-hack 测互斥 feature，而不是 `--all-features`；给 docs.rs 写明确的 feature 列表。热插拔保持 **load → use → dispose → 丢弃 `Library` → 再 load**，不要加 `reload()`。[7][14] WASM 继续分两条 ABI：Extism 字节协议与 Wasmtime Component Model 不能合成一条宿主路径。[4][12] Extism「近期不做 CM」只有维护者议题一条源，spot-check 标为 **unverified**。[4]

最大 caveat：WASI 0.3.0 规范已于 2026-06-11 稳定，但 wit-bindgen / Wasmtime 45 仍钉在 `0.3.0-rc-2026-03-15`；此时把 guest 钉到已发布的 `0.3.0` 会在实例化时报错。[22][27] 公开材料里没有 CM / Extism / native 的可比性能数字。

## 格局与成熟度

当前代际把 in-process 插件分成三条互不兼容的产品线，而不是一个会合并的栈。

**Native `dlopen`。** 生产向做法仍是宿主持有 `libloading::Library`，逻辑注销后再 Drop/`close`，让 OS 执行 `dlclose`/`FreeLibrary`。[14][9] `hot-lib-reloader` 是开发期文件监视 + 卸载再加载，文档写明：不能改签名、泛型不可热重载、可重载代码里的全局状态危险。它不是生产插件 ABI，也不能绕过 UCG 对残留引用的判定。[24][7] `abi_stable` 明确写「插件系统（不支持卸载）」——要跨 rustc 布局检查就放弃物理卸载。[11]

**Extism。** 定位是跨语言、以 `.wasm` 字节模块为插件、宿主 SDK 调用；能力由宿主授予。维护者在 #666（2025-09-21 仍开放）中把 Component Model 列为近期不做：要可移植多 runtime 宿主 + 字节 ABI，可选 WASI Preview 2，但不采用 CM。[3][4] 该「不做 CM」结论 **unverified**（单一出版方）。

**Wasmtime + WIT + Component Model。** Wasmtime 是 CM 参考实现。[1] 2025-08 的实践文把它写成相对 `dlopen` 的可选项：沙箱、WIT world、语言无关组件 ABI。[2] WASI 0.3.0 已批准：`async func` / `stream` / `future` 进入 canonical ABI。[22][23] 工具链尚未对齐：guest 教程仍用 `wasi:cli/run@0.3.0-rc-2026-03-15`，并警告 wit-bindgen 0.58 与 Wasmtime 45 仍发该 snapshot。[27]

**同代产品：Fidius。** 用 trait 宏生成稳定 C ABI cdylib（带 hash 校验入口），同一套宏可生成 wasmtime 组件；WASM 默认 deny-all，能力写在 package.toml，`http` 必须有宿主 egress 策略。[25] **unverified** 作为市场信号（仅项目 README，无独立生产复盘）。这是「签名包 + 沙箱分层」产品，不是「在现有 TypeId DI 上再加一个 `reload()`」能对齐的方向。

未找到：crates.io 下载榜上的插件/DI 领导crate；CM vs Extism vs native 的独立性能数字。

## 架构实践模式

**物理卸载。** UCG #526：要让 `dlclose` 有机会是 sound，必须清掉对 dylib 数据/代码的外部引用、出站函数指针、跨 DSO 符号，以及该库拥有的线程；带非平凡析构的 TLS 会单独挡住卸载（macOS 常直接拒绝；Linux 即使 `dlclose` 成功也可能没有 sound 路径）。[7] rustc #28794 记录过 TLS 析构与映射生命周期错位；后续 dyld 会把带 TLV 的 dylib 标成永不卸载，`dlclose` 成功不等于真正 unmap。[8]

libloading 0.9 与该模式一致：`Drop` 关闭库并忽略卸载错误；只有需要观察错误时才调 `close(self)`。`close` 在部分平台/打开方式下可能是 no-op。[14] Windows 实现里 `Drop` 调用 `FreeLibrary` 并忽略 BOOL；`close` 在失败后 `mem::forget`，避免 Drop 再试一次。[28]

**Windows 文件锁。** `FreeLibrary` 只减进程内模块引用计数；计数到零才卸载。返回成功不代表文件已解锁——其它 `LoadLibrary`、PIN、依赖模块仍会占着文件。[9][10] 因此宿主文档应写：卸载成功 ≠ 可覆盖 `.dll`。

**不要 `reload()`。** 没有任何被抽到的主源把「保留旧符号再换映射」写成 sound API。热插拔就是 dispose → drop handle → 以后再 `Library::new`。[7][14] 开发期可用 hot-lib-reloader 的 `update()`，它仍受同一套 OS 约束。[24]

**WASM 实例生命周期 ≠ `dlclose`。** Extism：`Plugin::reset` 作废 guest 内存/状态；C API 有 `extism_plugin_free` / `extism_plugin_reset`。[12] Wasmtime pooling：预分配 memory/table/instance，drop Store 把槽还池；`PoolingAllocationConfig` 主要按 Unix 调，且不是默认。[13] 这是实例回收，不是 native 卸载。

**两条 WASM ABI。** Extism 字节签名与 Wasmtime component/bindgen guest 不能互相加载。[4] 选一个宿主适配器就没有另一个。Fidius 用自己的 interface-hash + WIT 胶合把「同一 trait」编到 cdylib 或组件——那是第三种合约，不是 Extism 或裸 CM 的超集。[25]

## 发布质量与生态

**Trusted Publishing。** 已上线（RFC 3691 / 2025-07 公告）。CI 用平台 OIDC 换约 30 分钟的发布 token；**每个 crate 的第一次发布仍要 API token**。之后在 crates.io 配置 GitHub workflow（`id-token: write` + `rust-lang/crates-io-auth-action`）。GitLab 为 public beta。迁移期两种方式可并存。[5][6][16] Forge 把 Trusted Publishing 列为 rust-lang crate 的推荐路径。[16]

**不可变产物。** Cargo：版本永不覆盖；yank 不是删除，已有 lockfile 仍能拉到被 yank 的版本；密钥泄露要立即吊销，yank 挡不住。[17][18] 整 crate 删除政策很窄（<72h，或单 owner ∧ 生命周期内每月 <1000 下载 ∧ 无反向依赖）；2025-02 博文截图写过 500/月，现行政策写 1000。[19] README 绑在该版本的 `.crate` 上；维护者拒绝「不升版本改 README」，因为会破坏产物不可变。[20] 该 README 结论 **unverified**（#1750 老线程，无更新政策文）。docs.rs 文档可以通过 crates.io 版本列表触发重建，不必为此发新版。[5][26]

**docs.rs 构建。** 默认**不会** `--all-features`。用 `[package.metadata.docs.rs]` 的 `features` / `all-features` / `no-default-features`。[21] 互斥 feature 时不要 `all-features = true`，应列可同时打开的子集。`[workspace.metadata.docs.rs]` 尚不支持。

**CI。** cargo-hack 提供 `--feature-powerset` 与 `--mutually-exclusive-features`；`--each-feature` 在多 feature 时还会跑一次 `--all-features`，互斥时应加 `--exclude-all-features`。[15] 社区常见矩阵含 Windows、`fail-fast: false`、MSRV、clippy `-D warnings`；后一项没有官方强制文档。

未找到：6–12 个月维护负担调查；Rust 官方单一「RC 清单」。

## 跨维度洞察

1. **卸载是 native 卖点，也是 native 的天花板。** 架构源说 physical unload 有条件才 sound；格局源把 CM/WASM 写成避开 `dlopen` 的路。下一步若强化「可替换 `.so`」，收益在 Windows 文件锁与 TLS 文档，不在新 API。
2. **发布通道已经比加载通道更值得投。** 0.1.x 已满足「第一次 token 发布」；Trusted Publishing、docs.rs metadata、cargo-hack 是官方刚铺好的轨道，和加载器无关，却直接决定 0.1.2 的可信度。
3. **WASM 加深 = 钉 WIT，不是换品牌。** Extism 明确不做 CM；WASI 0.3 规范稳定但工具链仍在 RC pin。加深 Wasmtime 适配器要跟 pin 走，而不是引入 Extism 式字节 ABI，也不是现在就跟 Fidius 的签名包模型对标。

## 建议

绑定决策「0.1.x 之后先改什么」。置信度写在同一句。

1. **P0 — Trusted Publishing 接到 tag 发布工作流。** 第一次发布已完成，后续应用 OIDC，去掉长期 `CARGO_REGISTRY_TOKEN`。[6] 置信：**high**（crates.io 现行文档 + 2025-07 公告互证）。下游：发布/CI 文档、release.yml。
2. **P0 — CI 用 cargo-hack 表达互斥 feature，禁止把 `--all-features` 当唯一门禁。** `--mutually-exclusive-features` + `--exclude-all-features`。[15] 置信：**high**（工具主文档）。下游：CI 矩阵、测试说明。
3. **P0 — `plugctx` 的 `[package.metadata.docs.rs]` 列出可同时打开的 feature，不要 `all-features = true`。** [21] 置信：**high**。docs 重建可走 crates.io，不必为 rustdoc 失败而发版。[5]
4. **P1 — 中文用户文档写清 native 卸载限度：** `FreeLibrary` 成功 ≠ 文件可覆盖；macOS TLS 可能永不卸载；sound 卸载要求无残留引用。[9][7][8] 置信：**high**（OS/UCG 主源）；Windows 文件锁 anecdata 为 **medium**。下游：架构约束（保持无 `reload()`）、指南。
5. **P1 — WASM 适配器保持分裂；CM 路径钉死 WIT 版本。** 在 wit-bindgen/Wasmtime 刷新到 `0.3.0` 之前，guest/host 跟工具链 RC pin，不要提前钉发布标签。[27][22] 置信：**high**，但 **versions 类会在约一个月内过期**。下游：架构（FR26 仍是实例 close/free）。
6. **P2 — 不要为「热重载人体工学」加 `reload()`，也不要把 hot-lib-reloader 收进生产 API。** [7][24] 置信：**high**。
7. **P2 — 不要把 Fidius 式 trait→C ABI→签名 WASM 包当作 0.1.x 补丁。** 那是新产品史诗（宏 ABI、hash、deny-all、egress）。[25] 置信：**medium**（仅官方 README，无独立生产复盘）。
8. **P2 — 改 crates.io 上展示的 README/指南必须升版本。** yank 不删内容；docs.rs 重建只刷新 rustdoc，不换 README。[20][17] 置信：**medium**（#1750 线程老，立场在评论中重申，无更新政策文）。

## 未决问题

| 问题 | 若要回答需要 |
| --- | --- |
| CM / Extism / native 调用延迟与内存 | 同一工作负载的独立基准，而不是运行时博客 |
| wit-bindgen / Wasmtime 何时改钉 `0.3.0` | 跟 wit-bindgen #1554 与 Wasmtime 46 发行说明（versions，约每月） |
| Fidius 是否代表用户会要的类别 | 用户访谈或 crates.io 反向依赖，本轮没有 |
| 典型 Rust `cdylib` 在 Windows 上 Drop 后是否仍锁文件 | 用代表插件（含/不含 TLS、静态链接 vs 动态 OpenSSL）做一次实测 |
| 社区对 0.1.x API 的反馈 | 本轮无 user-voice 维；需要单独调研 |

## 来源附录

| | 支撑的发现 | 出版方 | 出版日期 | 访问 | 置信 |
| --- | --- | --- | --- | --- | --- |
| [1] | Wasmtime 为 CM 参考实现；WASI 0.3 ABI 在 Wasmtime 43+ | [Bytecode Alliance](https://component-model.bytecodealliance.org/running-components/wasmtime.html) | 2026-08-18（live） | 2026-08-18 | high |
| [2] | CM+WIT 作为相对 dlopen 的实践选项 | [Sy Brand](https://tartanllama.xyz/posts/wasm-plugins) | 2025-08-05 | 2026-08-18 | high |
| [3] | Extism：字节 WASM 插件 + 宿主 SDK | [Extism](https://extism.org/docs/concepts/plug-in-system/) | 2026-08-18（live） | 2026-08-18 | high |
| [4] | Extism 近期不做 Component Model | [Extism #666](https://github.com/extism/extism/issues/666) | 2025-09-21（最后活动） | 2026-08-18 | high |
| [5] | Trusted Publishing 公告；docs.rs 可从 crates.io 重建 | [Rust Blog](https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/) | 2025-07-11 | 2026-08-18 | high |
| [6] | Trusted Publishing：首次仍要 token；OIDC 约 30 分钟 | [crates.io](https://crates.io/docs/trusted-publishing) | 2026-08-18（live） | 2026-08-18 | high |
| [7] | dlclose soundness：无残留引用/TLS 限制 | [UCG #526](https://github.com/rust-lang/unsafe-code-guidelines/issues/526) | 2024-08-13 | 2026-08-18 | high |
| [8] | macOS TLS / dyld 可能永不卸载 | [rust#28794](https://github.com/rust-lang/rust/issues/28794) | 2018-01-01（线程 2015–2018） | 2026-08-18 | medium |
| [9] | FreeLibrary 只减引用计数 | [Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-freelibrary) | 2026-08-18（live API） | 2026-08-18 | high |
| [10] | 卸载「成功」但模块仍在：引用计数诊断 | [Raymond Chen](https://devblogs.microsoft.com/oldnewthing/20170915-00/?p=97035) | 2017-09-15 | 2026-08-18 | medium |
| [11] | abi_stable：插件系统且不支持卸载 | [docs.rs abi_stable](https://docs.rs/abi_stable/latest/abi_stable/) | 2026-08-18（live） | 2026-08-18 | high |
| [12] | Extism reset/free：实例拆除而非 dlclose | [docs.rs extism](https://docs.rs/extism/latest/extism/struct.Plugin.html) | 2026-08-18（live） | 2026-08-18 | high |
| [13] | Wasmtime pooling 非默认、偏 Unix | [Wasmtime docs](https://docs.wasmtime.dev/api/wasmtime/struct.PoolingAllocationConfig.html) | 2026-08-18（live） | 2026-08-18 | high |
| [14] | Library::close / Drop 关闭动态库 | [docs.rs libloading 0.9](https://docs.rs/libloading/latest/libloading/struct.Library.html) | 2026-08-18（live） | 2026-08-18 | high |
| [15] | cargo-hack 互斥 feature 与排除 --all-features | [taiki-e/cargo-hack](https://github.com/taiki-e/cargo-hack) | 2026-08-18（live） | 2026-08-18 | high |
| [16] | rust-lang crate 推荐 Trusted Publishing | [Rust Forge](https://forge.rust-lang.org/infra/docs/trusted-publishing.html) | 2026-08-18（live） | 2026-08-18 | high |
| [17] | 发布永久、yank≠删除、changelog+tag | [Cargo Book](https://doc.rust-lang.org/cargo/reference/publishing.html) | 2026-08-18（live） | 2026-08-18 | high |
| [18] | yank 不阻止已有 lockfile | [cargo yank](https://doc.rust-lang.org/stable/cargo/commands/cargo-yank.html) | 2026-08-18（live） | 2026-08-18 | high |
| [19] | crate 删除阈值现行为 1000/月 | [crates.io policies](https://crates.io/policies) | 2026-08-18（live） | 2026-08-18 | high |
| [20] | README 随版本不可变 | [crates.io #1750](https://github.com/rust-lang/crates.io/issues/1750) | 2019-01-01 | 2026-08-18 | medium |
| [21] | docs.rs metadata；默认非 all-features | [docs.rs metadata](https://docs.rs/about/metadata) | 2026-08-18（live） | 2026-08-18 | high |
| [22] | WASI 0.3.0 批准；工具链须同 pin | [WASI.dev](https://wasi.dev/releases/wasi-p3) | 2026-06-11 | 2026-08-18 | high |
| [23] | Wasmtime 45=RC，46 默认 CM async | [Bytecode Alliance](https://bytecodealliance.org/articles/WASI-0.3) | 2026-06-11 | 2026-08-18 | high |
| [24] | hot-lib-reloader 为开发工具及其限制 | [docs.rs hot-lib-reloader](https://docs.rs/hot-lib-reloader/latest/hot_lib_reloader/index.html) | 2026-08-18（live） | 2026-08-18 | high |
| [25] | Fidius：trait 宏、C ABI、WASM deny-all | [colliery-io/fidius](https://github.com/colliery-io/fidius) | 2026-08-18（live） | 2026-08-18 | high |
| [26] | docs.rs 重建 API/产品化 | [crates.io PR 11169](https://github.com/rust-lang/crates.io/pull/11169) | 2025-05-20 | 2026-08-18 | medium |
| [27] | guest 教程仍钉 0.3.0-rc-2026-03-15 | [CM book / Rust](https://component-model.bytecodealliance.org/language-support/creating-runnable-components/rust.html) | 2026-08-18（live） | 2026-08-18 | high |
| [28] | Windows Drop → FreeLibrary；close 失败则 forget | [libloading windows/mod.rs](https://github.com/nagisa/rust_libloading/blob/master/src/os/windows/mod.rs) | 2026-08-18（live） | 2026-08-18 | high |

## 过期地图

窗口（技术包）：versions/compat 1 个月 · ecosystem 6 个月 · landscape 12 个月 · pattern / failure / ops 24 个月。live 文档的 `pub_date` 取访问日 2026-08-18。由 `recon_kit.py staleness` 对 `claims.json` 计算（today=2026-08-18）：**stale_count=8**。

机械最早复查日是 **2020-01-01**（[8] macOS TLS 历史线程，failure 窗口）。行动上应忽略这条历史账，优先刷新已经到期的 **WASI / Wasmtime pin（复查日 2026-07-11）**。[22][23]

| 声明 | 类 | 复查日 | 状态 |
| --- | --- | --- | --- |
| WASI 0.3.0 批准 / Wasmtime 45–46 | versions/compat | 2026-07-11 | stale |
| Extism 不做 CM（#666） | versions/compat | 2025-10-21 | stale |
| Trusted Publishing 公告（博文） | ecosystem | 2026-01-11 | stale |
| CM+WIT 实践文 | landscape | 2026-08-05 | stale |
| UCG #526 dlclose soundness | pattern | 2026-08-13 | stale |
| README 随版本不可变 | ops | 2021-01-01 | stale（历史政策线程） |
| macOS TLS 永不卸载 | failure | 2020-01-01 | stale（历史） |
| wit-bindgen / Wasmtime 45 仍发 RC WIT | versions/compat | 2026-09-18 | 未到期；下次刷新优先 |
| Trusted Publishing live 文档 | versions/compat | 2026-09-18 | 未到期 |
