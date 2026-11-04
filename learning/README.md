# SolidCraft 源码入门与调试指南（新手版）

> 适用版本：本仓库当前 HEAD `a5b71ca20`（0.40.0-rc.1）。
> 文中行号按这个提交写的，代码改动后会有偏移，**以函数名/符号名为准**。
> 写给"第一次读这个项目、想真正跑起来和调试它"的人。

---

## 目录

1. [先看结论：这个项目怎么运转](#1-先看结论这个项目怎么运转)
2. [仓库地图：哪个 crate 干什么](#2-仓库地图哪个-crate-干什么)
3. [15 分钟跑起来](#3-15-分钟跑起来)
4. [调试环境：RustRover 和 Zed](#4-调试环境rustrover-和-zed)
5. [一条命令的完整生命周期（以 `solid.extrude` 为例）](#5-一条命令的完整生命周期以-solidextrude-为例)
6. [数据模型：Document、Timeline、Feature、求值](#6-数据模型documenttimelinefeature求值)
7. [渲染链路：为什么有两套网格](#7-渲染链路为什么有两套网格)
8. [把测试当文档用](#8-把测试当文档用)
9. [贡献规则速查（来自 AGENTS.md）](#9-贡献规则速查来自-agentsmd)
10. [两周学习计划](#10-两周学习计划)
11. [练习清单（从易到难）](#11-练习清单从易到难)
12. [常见问题 FAQ](#12-常见问题-faq)
13. [速查表](#13-速查表)
14. [术语表](#14-术语表)
15. [配套练习文件](#15-配套练习文件)

---

## 1. 先看结论：这个项目怎么运转

### 1.1 一句话

**SolidCraft 里"一切皆命令"**：工具栏、快捷键、CLI、脚本、MCP、AI 助手，最后都落到同一个入口：

```rust
// crates/engine/src/lib.rs:1614
pub fn execute(&mut self, id: &str, params: &Value) -> Result<Value>
```

命令有 id（如 `solid.extrude`）、有 JSON 参数、有 JSON 结果，并且**有输入/输出 schema**。所以：

- 学会一个命令怎么实现，就学会了全部命令；
- 调试时在任何入口（UI/CLI/MCP）打一个断点，都能看到同一条路径；
- 不打开界面也能验证功能（`solidcraft-cli`）。

### 1.2 三种数据形态

| 形态 | 在哪 | 是什么 |
|---|---|---|
| **设计** | `crates/doc` | 参数 + 草图 + **特征时间线**（Feature），可以随时重算 |
| **几何** | `crates/kernel`（B-rep）+ `crates/geom` | 精确的实体：面、边、顶点 |
| **显示** | `crates/geom::tess` + `crates/gpu` | 三角网格，只用于画出来 |

### 1.3 一次改动的数据流

```
用户操作
  └─ UI（crates/ui-egui）或 CLI/MCP
      └─ 命令 id + JSON 参数
          └─ Session::execute（crates/engine）
              ├─ 改 Document：加/改一个 Feature（crates/doc）
              └─ 重新求值：
                  ├─ doc::eval 走时间线
                  ├─ kernel 建/改 B-rep（crates/kernel，基于 truck）
                  └─ geom 生成 TessPlan（显示网格计划）
                      └─ cpu 求值 或 gpu 求值（crates/gpu/tess.wgsl）
                          └─ ui-egui 视口画出（crates/ui-egui + crates/gpu）
```

记住这张图，后面所有内容都是往它上面挂细节。

---

## 2. 仓库地图：哪个 crate 干什么

### 2.1 分层表（来自 `docs/architecture.md`）

| 层 | crate | 一句话 |
|---|---|---|
| L0 | `crates/geom` | 向量、平面、曲线、网格、BVH、HLR —— 纯数学 |
| L1 | `crates/sketch` | 草图约束求解器 + 轮廓 |
| L1 | `crates/kernel` | **B-rep 内核边界**（唯一允许用 `truck` 的层） |
| L1 | `crates/render` | 视图数学与无头场景描述 |
| L2 | `crates/doc` | 参数、表达式、**特征时间线**、增量求值 |
| L3 | `crates/io` | 设计文件、STEP/STL/OBJ/3MF 导入导出 |
| L4 | `crates/engine` | **Session + 命令注册表**（产品的大脑） |
| L5 | `crates/gpu` | 显示网格的 GPU 求值与渲染管线（wgpu） |
| L5 | `crates/ui-egui` | egui 桌面前端（最大的 crate） |
| L5 | `crates/mcp` | MCP 服务器（无头或桥接到 app） |
| apps | `apps/solidcraft` | 桌面 app（窗口、启动、平台细节） |
| apps | `apps/solidcraft-cli` | 无头 CLI（`run` / `eval` / `snapshot` / `mcp` …） |
| apps | `apps/solidcraft-web` | 浏览器版本 |
| 工具 | `xtask` | `cargo xtask …` 仓库检查与测试编排 |

分层的意义：**只能往下依赖，不能往上**。`cargo xtask layers` 会强制检查。

### 2.2 新手最该先看的 10 个文件

按顺序读，别贪多：

1. `AGENTS.md` —— 硬规则（不许 `unwrap`/`panic`/`unsafe`，命令要有 schema，测试放哪……）
2. `docs/architecture.md` —— 只有 13 行，先背下来
3. `docs/book/src/dev/commands.md` —— 命令系统怎么设计的
4. `apps/solidcraft/src/main.rs` —— 程序怎么启动、参数、窗口
5. `crates/engine/src/lib.rs` —— `Session`，尤其 `execute`（:1614）
6. `crates/engine/src/cmd/mod.rs` —— 命令注册表 `CommandSpec`（:144）
7. `crates/engine/src/cmd/features.rs` —— 特征命令（`solid.extrude` 在 :35，实现 :877）
8. `crates/doc/src/document.rs` —— `Feature`（:2892）、`Document`（:3079），时间线
9. `crates/doc/src/eval.rs` —— 求值：时间线怎么跑出几何
10. `crates/geom/src/tess.rs` —— `TessPlan`（:84），显示网格的"计划"

### 2.3 找东西的三个常用 grep

```bash
# 某个命令在哪注册、实现在哪
grep -rn 'CommandSpec::new("solid.extrude"' crates/engine/src

# 某个命令的所有参数（文档字符串就在注册处）
grep -rn '"solid.fillet"' crates/engine/src/cmd

# 某个 UI 文案/设置项
grep -rn 'display_quality' crates/ui-egui/src
```

`solidcraft-cli commands` 会打印**所有命令的完整参数说明**（就是产品全貌），比翻源码快：

```bash
solidcraft-cli commands | less
solidcraft-cli commands | grep -i fillet
```

---

## 3. 15 分钟跑起来

### 3.1 编译

```bash
cd /docs/lxy/documents/github/solidcraft

cargo build -p solidcraft --bin solidcraft      # dev profile，输出 target/debug/solidcraft
```

- 本机已经有一份 RustRover 编好的 dev 产物，这条命令应该 **0.x 秒就 `Finished`**（共用同一份）。
- 如果它开始 `Compiling` 一大堆 crate，说明 feature/工作目录和之前的构建不一致（见 FAQ「为什么 target 里有两份」）。

### 3.2 运行

```bash
# 普通启动
cargo run -p solidcraft

# 带一个示例设计 + 控制通道（推荐：可以脚本驱动）
cargo run -p solidcraft -- --sample --control 8801

# 显卡/渲染有问题时（软件渲染 + CPU 网格 + 默认设置）
cargo run -p solidcraft -- --safe-mode --control 8801
```

常用参数（完整列表看 `apps/solidcraft/src/main.rs` 头注释）：

| 参数 / 环境变量 | 作用 |
|---|---|
| `--control <port>` | 开本地控制通道（JSON lines） |
| `--sample` | 打开内置示例设计 |
| `--safe-mode` | 默认设置 + 无恢复 + 软件渲染 + CPU 网格 |
| `SOLIDCRAFT_SETTINGS_DIR` | 指定设置目录（不污染真实设置） |
| `SOLIDCRAFT_PROFILE_STARTUP=1` | 打印启动各阶段耗时 |
| `SOLIDCRAFT_WAYLAND=0` | 强制 XWayland |

### 3.3 不打开界面也能玩：CLI

> ⚠️ 本机 `/usr/bin/solidcraft-cli` 是 **0.30.1**，比仓库（0.40.0-rc.1）旧，几何结果会有差异。
> 想和仓库一致，用仓库构建的版本，并加 `--features bundled-render-library`（和 app 用同一套 engine 产物，
> 否则会多编一份）：
>
> ```bash
> cargo run -p solidcraft-cli --features bundled-render-library -- commands
> ```

```bash
# 所有命令
solidcraft-cli commands | less

# 跑一个脚本，导出 STEP
solidcraft-cli run examples/bracket.json --out /tmp/bracket.step

# 看结果数字（体积、包围盒、面数……）
solidcraft-cli eval examples/bracket.json

# 渲染一张图
solidcraft-cli snapshot examples/bracket.json --out /tmp/bracket.png --view iso
```

`examples/` 下有现成脚本，是很好的"可执行文档"。

### 3.4 直接和运行中的 app 对话（控制通道）

app 启动后，`~/.local/share/solidcraft/control/<port>.token` 里有 token：

```bash
python3 - <<'EOF'
import json, socket, pathlib
port = 8801
tok = pathlib.Path.home().joinpath(f".local/share/solidcraft/control/{port}.token").read_text().strip()
req = {"id": 1, "method": "document.inspect", "params": {}, "token": tok}
s = socket.create_connection(("127.0.0.1", port), timeout=30)
s.sendall((json.dumps(req) + "\n").encode())
print(json.loads(s.makefile().readline()))
EOF
```

常用方法：`document.inspect`、`engine.execute`、`engine.batch`、`ui.screenshot`、`ui.render`、`ui.view`、`ui.camera`。
完整协议见 `docs/control-protocol.md`。

> MCP 也是同一套：`solidcraft-cli mcp --allow-path <目录>`。如果桌面 app 正在运行，MCP 服务器会**桥接**到它，工具直接作用在你的窗口上。

---

## 4. 调试环境：RustRover 和 Zed

两种 IDE 都可以，而且**共用同一份 `target/` 产物**。

### 4.1 关键原则：只保留一套 dev 产物

- 你的 app 构建：`cargo build --bin solidcraft --manifest-path apps/solidcraft/Cargo.toml`（RustRover 默认）
- 等价的命令行：`cargo build -p solidcraft --bin solidcraft`（仓库根）
- 这两条解析出的 feature 集合相同 → 同一个编译单元 → 共用。

验证方法（任何时候）：

```bash
cargo build -p solidcraft --bin solidcraft
# Finished `dev` profile ... in 0.4s   ← 共用
# 或开始 Compiling ...                  ← 分叉了
```

**不要**在仓库根跑 `cargo test -p <单个包>`（默认 dev profile）——那会生成另一套 dev 产物（约 +10 GB）。测试统一用：

```bash
cargo test -p solidcraft-geom --profile ci     # 单包
cargo xtask test                                # 全工作区（内部也是 --profile ci）
```

### 4.2 RustRover

1. 用 RustRover 打开**仓库根目录**。
2. Run → Edit Configurations → `+` → **Cargo**：

```
Command:            run
Package:            solidcraft
Working directory:  <仓库根>
Additional args:    --control 8801
Environment:        RUST_BACKTRACE=1;SOLIDCRAFT_PROFILE_STARTUP=1
```

3. 或者最省事：打开 `apps/solidcraft/src/main.rs`，点 `fn main` 左边的 **Debug**。
4. 断点打在 `crates/` 任何地方都生效（dev profile 有完整 debuginfo）。
5. 二进制约 3 GB，LLDB 首次加载符号要几秒到几十秒，正常。

### 4.3 Zed

仓库里已经配好了 `.zed/`（本地配置，未提交）：

| 文件 | 内容 |
|---|---|
| `.zed/debug.json` | 4 个 CodeLLDB 调试配置 |
| `.zed/tasks.json` | 编译/运行/测试/ci 任务 |
| `.zed/settings.json` | rust-analyzer 只检查当前 crate |

调试按 **`F4`**（`debugger: start`），选：

- `SolidCraft app (build + debug)` — GPU
- `SolidCraft app (GPU, control 8801)`
- `SolidCraft app (software renderer, control 8801)` — **safe mode，画面一定正常**
- `SolidCraft CLI (debug)`

运行任务用 `task: spawn`（和调试列表不是同一个弹窗），任务里的端口也统一是 **8801**。

### 4.4 调试技巧

| 目标 | 做法 |
|---|---|
| 看每个命令 | 在 `Session::execute`（`engine/src/lib.rs:1614`）下断点，看 `id` 和 `params` |
| 看某个特征为什么失败 | 在 `crates/engine/src/cmd/features.rs` 对应函数下断点，或在 `doc::eval` 里看 `FeatureResult.error` |
| 看渲染网格 | `crates/geom/src/tess.rs` 的 `TessPlan::evaluate`，对比 CPU/GPU 结果 |
| 打印而不打断 | `eprintln!` 或 `log::info!`（仓库禁止 `dbg!`） |
| 崩溃栈 | 环境变量 `RUST_BACKTRACE=1` |
| 启动慢 | `SOLIDCRAFT_PROFILE_STARTUP=1` |
| 只跑一个测试 | `cargo test -p <crate> --profile ci <名字> -- --nocapture` |
| 条件断点 | RustRover 右键断点 → Condition，例如 `id == "solid.extrude"` |

### 4.5 本机特有的坑：GPU 网格

这台机器是 **NVIDIA GT 710 + 驱动 470.256.02**。实测：GPU 曲面求值（`crates/gpu/src/tess.wgsl`）在这个驱动上算错，导致窗口里圆柱被画成"下大上小的喇叭 + 接缝尖角"，而内核几何是对的。

证据（两条命令，建议亲手跑一遍）：

```bash
# 失败：GT 710
cargo test -p solidcraft-gpu --lib -- gpu_evaluation_matches_the_cpu

# 通过：同一份着色器，换软件 Vulkan（llvmpipe）
SOLIDCRAFT_GPU=software cargo test -p solidcraft-gpu --lib -- gpu_evaluation_matches_the_cpu
```

所以：

- 调 UI/几何逻辑时用 **`--safe-mode`**（CPU 网格），避免被错误的显示误导；
- 内核/几何对不对，以 `measure`、`solidcraft-cli eval`、`ui.render` 的结果为准，不要以窗口截图为准；
- 这个 bug 本身就是一个很好的入门任务（见练习 L5）。

---

## 5. 一条命令的完整生命周期（以 `solid.extrude` 为例）

跟着这条线走一遍，你就懂了 80%。

### 5.1 入口

```rust
// crates/engine/src/lib.rs:1614
pub fn execute(&mut self, id: &str, params: &Value) -> Result<Value>
```

它做三件事：查注册表 → 校验参数（schema）→ 调用处理函数；失败会带 `param_warnings`。

### 5.2 注册表

```rust
// crates/engine/src/cmd/mod.rs:144
pub struct CommandSpec { /* id, label, run, params 文档, 启用条件, 工具栏位置 */ }
```

每个命令在这里登记，例如：

```rust
// crates/engine/src/cmd/features.rs:35
CommandSpec::new("solid.extrude", "Extrude", crate::checked!("solid.extrude", extrude))
```

`params` 的文档字符串（参数说明、默认值、可选值）就在注册处，这是最权威的接口文档。

### 5.3 实现

```rust
// crates/engine/src/cmd/features.rs:877
fn extrude(s: &mut Session, p: &Value) -> Result<Value>
```

典型流程：

1. 解析参数（长度/角度表达式 → 数值，`crates/engine/src/params.rs`）；
2. 找草图/轮廓（`crates/doc`、`crates/geom`）；
3. 往时间线追加一个 `Feature`；
4. 触发重新求值；
5. 返回 `{feature, name, bodies, created_bodies, changed_bodies, removed_bodies, warning, ms}`。

### 5.4 记录

```rust
// crates/doc/src/document.rs:2892  Feature
// crates/doc/src/document.rs:3079  Document  （参数 + 时间线 + 草图 + 体）
```

一个设计文件（`.sc`）本质就是这些结构序列化后的结果（格式在 `crates/doc/src/format/`）。

### 5.5 求值

`crates/doc/src/eval.rs`：按时间线顺序执行特征，产出 `FeatureResult`（体、警告、错误）。

- 参数/表达式：`crates/doc/src/expr.rs`
- 名字与引用（面/边/草图曲线的稳定引用）：`crates/doc/src/names.rs`、`feature_refs.rs`
- 求值失败时：`FeatureResult.error`，UI 在时间线里显示

### 5.6 几何

`crates/kernel/`（B-rep，truck）负责布尔、倒角、抽壳、扫掠等；`crates/geom/` 负责数学与显示网格计划。

### 5.7 显示

```rust
// crates/geom/src/tess.rs:84
pub struct TessPlan { ... }          // 每个面的 (u,v) 采样点
// crates/geom/src/tess.rs:93
impl TessPlan { pub fn evaluate(&self) -> Mesh }   // CPU 求值
```

窗口里的曲面默认走 **GPU 求值**（`crates/gpu/tess.wgsl`），`--safe-mode` 时走 CPU。

### 5.8 渲染

- 视口管线：`crates/gpu/src/view.rs`（`Renderer::with_window_target` :2523）
- egui 胶水：`crates/ui-egui/src/gpu.rs`（`install` :42）
- 前端：`crates/ui-egui/src/viewport.rs`（很大，按函数名搜）

### 5.9 用断点验证这条链

1. `Session::execute` 断点 → 确认 `id == "solid.extrude"`，看 `params`；
2. `features.rs` 的 `extrude` 断点 → 看解析后的数值；
3. `doc::eval` 里断点 → 看新 Feature 的求值结果；
4. `TessPlan::evaluate` 断点 → 看生成的三角数量；
5. 打开 `--safe-mode` 对照窗口 → 确认几何。

---

## 6. 数据模型：Document、Timeline、Feature、求值

### 6.1 参数与表达式

```bash
solidcraft-cli eval design.sc        # 看参数、特征、体
```

- 参数是"名字 = 表达式"，可带单位（`"40 mm"`），在 `crates/doc/src/expr.rs` 求值；
- 命令里写数字 ≠ 参数：想让尺寸跟着参数走，字段要写表达式（如 `"L/2 - 10"`）。

### 6.2 时间线

- `Feature` 有 `id`、`name`、`kind`、`suppressed`、`rolled_back`；
- 求值从第一个特征走到最后，`rolled_back` 之后的不算；
- 时间线里显示的 `error`/`warning` 就是 `FeatureResult` 里的内容。

### 6.3 稳定引用（这个项目的特色）

面和边会**跨修改保持名字**，所以倒角/抽壳能在参数变化后仍然指向同一条边：

- `crates/doc/src/names.rs`（`FeatureId` 等）
- `crates/doc/src/feature_refs.rs`
- 用户侧表现：改 `d1` 之后，倒角还在原来那条边上

### 6.4 撤销/重做

每个命令是一个 undo 步骤（部分命令有分组），见 `crates/engine` 的 edit 相关模块与 `Session` 的 undo 栈。

---

## 7. 渲染链路：为什么有两套网格

| 路径 | 谁用 | 网格来源 | 特点 |
|---|---|---|---|
| **CPU 网格** | `solidcraft-cli snapshot`、MCP 截图、`ui.render` | `TessPlan::evaluate`（Rust, f64） | 准，慢 |
| **GPU 网格** | 桌面窗口视口 | `crates/gpu/tess.wgsl`（f32） | 快，依赖驱动正确性 |

两条路径按理应给出同样结果（仓库里有逐顶点对比测试）。本机 GT 710 上 GPU 路径算错，于是出现：

- 窗口里圆柱畸形；
- `ui.render` / CLI 截图正常；
- `measure` / 内核几何正常。

**判断原则**：几何问题看内核/CPU 结果，显示问题才怀疑渲染。

---

## 8. 把测试当文档用

### 8.1 测试放哪（仓库规矩）

- 单元/模块测试：`crates/<crate>/src/<模块>_tests.rs`（**区域已有的文件**，不要新建 per-bug 文件）
- 集成测试：`crates/<crate>/tests/*.rs`
- 端到端：`apps/solidcraft-cli/tests/`、`tests/`

### 8.2 跑测试

```bash
cargo test -p solidcraft-geom --profile ci            # 单包
cargo test -p solidcraft-geom --profile ci 名字 -- --nocapture
cargo xtask test                                       # 全工作区，按二进制并行
cargo xtask test -p solidcraft-gpu                     # 透传 cargo 参数
```

`--profile ci` 是仓库自带的测试 profile：`debug = "line-tables-only"`，比 dev 小很多，**专门用来避免生成第二套 dev 产物**。

### 8.3 读测试学接口

一个测试往往就是最好的用法示例，例如：

- `crates/engine/src/cmd/script_tests.rs` —— 命令脚本长什么样
- `crates/gpu/src/tests.rs` —— GPU 求值 vs CPU 求值（就是这次抓到 bug 的那个）
- `crates/kernel/src/*_tests.rs` —— 内核边界情况

### 8.4 CI 门禁

```bash
cargo xtask ci
```

依次跑：`cargo fmt --check` → 资源 → 图标 → 许可证 → 排序列表 → skills → 分层 → UI 容器 → "模型决策必须用精确几何" → `clippy -D warnings` → wasm 编译检查 → 测试 → web-offline。提交前必须过。

---

## 9. 贡献规则速查（来自 AGENTS.md）

读一遍原文最好，这里只列最容易踩的：

| 规则 | 说明 |
|---|---|
| 不许崩溃 | 非测试代码禁止 `unwrap`/`expect`/`panic`/`unsafe`，错误用 `Result` |
| 命令必须有 schema | 新命令要有输入/输出 schema，`schema/unconverted.txt` 只能变短 |
| 一个设计格式变更 = 一个新文件 | `crates/doc/src/format/pending/<slug>.rs` |
| 新 UI 字符串 / attribution = pending 文件 | `assets/locales/pending/<slug>.json`、`attribution/pending/<slug>.md` |
| 排序列表保持有序 | 消息目录、`mod` 行、语言列表：`cargo xtask lists` |
| 生成文件不要手改 | `docs/features.md`、`THIRD-PARTY-LICENSES.txt` |
| 测试放已有文件 | 不要为每个 bug 新建测试文件 |
| 模型决策用精确几何 | 平面/圆柱/锥/球/环面的判定不能用网格：`cargo xtask mesh-decisions` |
| 默认行为变更 | 旧行为要留在偏好项后面，并填 PR 的 `Changes a default:` |
| PR 规矩 | 先开 draft PR claim；一个 PR 一个主题；标题/第一句要自解释；改 UI 要真实截图 |
| 提交前 | `cargo xtask ci`，跑的时候别切分支 |

独立开发/不抄别家代码/资产要有授权记录——这些在 `skills/solidcraft-ground-rules/SKILL.md`。

---

## 10. 两周学习计划

每天 1–2 小时。每步都有"验收"，做完才算过。

### 第 1–2 天：跑起来，建立直觉
- 跑 `cargo run -p solidcraft -- --sample --control 8801`，画圆、拉伸、倒角、抽壳各一次。
- 跑 `solidcraft-cli commands | less`，随便挑 5 个命令看参数。
- 用 `solidcraft-cli run/eval/snapshot` 处理 `examples/bracket.json`。
- **验收**：你能不点鼠标，用 CLI 或控制通道建出一个圆柱 + 底部倒角，并读出体积。

### 第 3–4 天：架构和入口
- 读 `docs/architecture.md`、`docs/book/src/dev/commands.md`。
- 在 `Session::execute` 下断点，随便点几下界面，观察 `id`/`params`。
- **验收**：能画出一张"UI 点击 → 命令 → doc → kernel"的流程图，并说出每层的 crate。

### 第 5–6 天：追一条完整命令
- 选中 `solid.extrude`，从 `CommandSpec::new` 读到 `features.rs:877` 的 `extrude`，再读到 `doc::eval`。
- 用断点验证每个阶段的输入输出。
- **验收**：你能不看代码，说出 `solid.extrude` 的 5 个关键参数和它们落到数据结构里的哪。

### 第 7–8 天：时间线与参数
- 读 `crates/doc/src/document.rs`（`Feature`/`Document`）、`eval.rs`、`expr.rs`。
- 做一个实验：参数 `d1 = 100` 的圆，改到 60，观察时间线重算和体的变化。
- **验收**：能解释"为什么改了参数，倒角还在原来那条边上"（stable naming）。

### 第 9–10 天：渲染
- 读 `crates/geom/src/tess.rs`、`crates/gpu/tess.wgsl` 头部注释、`crates/ui-egui/src/gpu.rs:install`。
- 跑第 4.5 节那两条 `cargo test`（GT 710 失败 / llvmpipe 通过）。
- **验收**：能解释"为什么窗口畸形而 `ui.render` 正常"。

### 第 11–12 天：测试与第一次修改
- 找一个已有测试，加一个边界用例（放在同区域的 `*_tests.rs` 里）。
- 跑 `cargo test -p <crate> --profile ci`。
- **验收**：测试从红变绿，且没有新建测试文件。

### 第 13–14 天：走一遍 PR 流程（可选）
- 选一个 `good first issue`，开 draft PR claim。
- 跑 `cargo xtask ci`，按规则写 PR 标题和说明。
- **验收**：CI 绿、PR 描述让没看过聊天记录的人也能看懂。

---

## 11. 练习清单（从易到难）

| # | 练习 | 验收标准 |
|---|---|---|
| L1 | 用 CLI 建圆柱（直径 100、高 41），底部倒角 10，导出 STEP | `eval` 输出 bbox `100×100×41`，体积 ≈ 315 560 mm³ |
| L2 | 在 `Session::execute` 下断点，用界面完成"拉伸 + 倒角"，说出命令顺序 | 断点命中顺序与操作一致 |
| L3 | 找到 `solid.fillet` 的实现，解释 `fit: exact/reduce/taper` 的区别 | 能举出各自失败/成功的一个例子 |
| L4 | 给 `crates/geom` 里某个已有测试加一个边界用例 | `cargo test -p solidcraft-geom --profile ci` 通过 |
| L5 | 复现并定位本机 GPU 网格 bug（第 4.5 节），写一份 issue 草稿 | 有命令、GPU/驱动版本、失败/通过对照 |
| L6 | 改进：让 app 在检测到该 adapter 时用 CPU 网格（需产品决策，先开 issue） | 有 issue + 方案，再动代码 |

> 关于 L1 的期望值：圆柱体积 = π·50²·41 ≈ 322 013 mm³；底部 R10 倒角后约 315 560 mm³（以 `eval` 实测为准，别背数字，背方法）。

---

## 12. 常见问题 FAQ

**Q：为什么 `target/` 里同一个 crate 有两份？**
A：cargo 按"包 + 目标种类 + feature 集合 + profile + RUSTFLAGS + 依赖"算 hash。RustRover 的 app 构建启用了 `apps/solidcraft` 的默认 feature（`raytrace-cpu`、`bundled-render-library`、`profile`），而 `cargo test -p 单个包` 的依赖图没有这些 feature，于是整条链各存一份。验证：`ls target/debug/deps/libsolidcraft_engine-*.rlib` 会看到两个 hash。清理：`cargo clean`（全清）或 `cargo clean -p <包>`。

**Q：为什么我改的 UI 文案没生效？**
A：新增字符串要走 `assets/locales/pending/<slug>.json`，然后 `cargo xtask i18n` 检查；生成目录不要手改。

**Q：测试为什么不用默认 profile？**
A：默认 dev 会产生第二套 dev 产物（+10 GB 级别）。仓库自带 `ci` profile（line-tables-only，无增量），用 `--profile ci` 或 `cargo xtask test`。

**Q：`cargo nextest` 报找不到？**
A：本机没装 nextest。仓库的 `cargo xtask test` 自己编排（`cargo test --workspace --profile ci --no-run` 后按二进制跑），不需要 nextest。

**Q：窗口里模型显示不对，但 `measure` 正常？**
A：先怀疑显示路径。用 `ui.render`（CPU 网格）对照窗口；本机已知 GPU 网格在 GT 710 上算错，用 `--safe-mode`。

**Q：端口 8799 / 8801 有什么区别？**
A：只是控制通道端口。`.zed/` 里统一用 8801，避免和你手动启动的 8799 冲突。`current.port` 指向最后启动的实例，MCP 会桥接到它。

**Q：`--safe-mode` 会丢我的设计吗？**
A：它用默认设置且不恢复上次会话，所以不会自动打开你上次的设计；手动打开 `.sc` 即可。它不会删除磁盘上的设计。

**Q：为什么不能 `unwrap()`？**
A：这个是硬规则（`#![deny(clippy::unwrap_used)]` 等），因为产品不能崩溃。用 `?`、`ok_or`、`let Some(x) = … else { return … }`。

**Q：改完要不要跑 `cargo xtask ci`？**
A：提交前必须。它比较慢（全量测试 + clippy + wasm），可以先跑局部的 `cargo check -p <crate>`、`cargo test -p <crate> --profile ci`。

---

## 13. 速查表

### 13.1 常用命令

```bash
cargo build -p solidcraft --bin solidcraft        # 编 app（与 RustRover 共用）
cargo run  -p solidcraft -- --control 8801        # 运行 + 控制通道
cargo run  -p solidcraft -- --safe-mode           # 软件渲染
cargo check -p <crate> --all-targets              # 快速查错
cargo test -p <crate> --profile ci                # 单包测试
cargo xtask test                                  # 全工作区测试
cargo xtask ci                                    # 提交前全部门禁
cargo xtask layers                                # 分层检查
cargo xtask lists                                 # 排序列表检查
cargo xtask features                              # 生成命令清单
cargo xtask i18n --check                          # 翻译完整性
solidcraft-cli commands                           # 命令目录
solidcraft-cli run/eval/snapshot ...              # 无头使用
```

### 13.2 关键路径

| 想找 | 文件 |
|---|---|
| 命令入口 | `crates/engine/src/lib.rs`（`Session::execute`） |
| 命令注册 | `crates/engine/src/cmd/mod.rs`、`crates/engine/src/cmd/features.rs` |
| 时间线/文档 | `crates/doc/src/document.rs`、`eval.rs` |
| 表达式/参数 | `crates/doc/src/expr.rs`、`crates/engine/src/params.rs` |
| 内核（B-rep） | `crates/kernel/src/`（唯一用 truck 的层） |
| 显示网格计划 | `crates/geom/src/tess.rs` |
| GPU 网格/渲染 | `crates/gpu/tess.wgsl`、`crates/gpu/src/view.rs` |
| 视口前端 | `crates/ui-egui/src/viewport.rs`、`crates/ui-egui/src/gpu.rs` |
| 桌面启动 | `apps/solidcraft/src/main.rs` |
| CLI | `apps/solidcraft-cli/src/` |
| 控制协议 | `docs/control-protocol.md` |
| MCP | `docs/mcp.md`、`crates/mcp/src/` |
| 开发文档 | `docs/book/src/dev/` |
| 各主题速查 | `skills/*/SKILL.md` |

### 13.3 环境变量

| 变量 | 作用 |
|---|---|
| `SOLIDCRAFT_SAFE_MODE=1` | 同 `--safe-mode` |
| `SOLIDCRAFT_SETTINGS_DIR` | 设置目录 |
| `SOLIDCRAFT_CONTROL_PORT` | 同 `--control` |
| `SOLIDCRAFT_PROFILE_STARTUP=1` | 启动耗时 |
| `SOLIDCRAFT_PROFILE=1` | 性能分析器从启动开始记录 |
| `SOLIDCRAFT_WAYLAND=0/1` | 强制 XWayland / Wayland |
| `RUST_BACKTRACE=1` | panic 栈 |
| `SOLIDCRAFT_GPU=software` | 无头渲染用软件 Vulkan（测试对比用） |
| `SOLIDCRAFT_TEST_JOBS` | `cargo xtask test` 的并发上限 |

---

## 14. 术语表

| 词 | 含义 |
|---|---|
| **Command** | 一切操作的统一接口：id + JSON 参数 + JSON 结果 |
| **Session** | 引擎的运行状态：打开的设计、执行命令、撤销栈 |
| **Document** | 一个设计：参数、草图、时间线、体 |
| **Feature** | 时间线上的一步（草图、拉伸、倒角……） |
| **Timeline** | 特征的顺序列表；可回滚、可抑制 |
| **B-rep** | 精确边界表示（面/边/顶点），内核（truck）维护 |
| **TessPlan** | 显示网格的"计划"：每个面在 (u,v) 上采样点，CPU/GPU 都能求值 |
| **Tessellation** | 把曲面变成三角网格 |
| **Stable naming** | 面和边跨修改保持名字，引用不丢 |
| **Control channel** | app 的本地 JSON lines 接口（默认端口来自设置/参数） |
| **MCP** | 给 AI/脚本用的工具协议；服务器可无头，也可桥接桌面 app |
| **Profile（cargo）** | dev / ci / release 等构建配置；`ci` 专给测试用 |
| **Adapter（wgpu）** | 实际干活的 GPU/软件后端（NVIDIA、llvmpipe…） |

---

## 15. 配套练习文件

| 文件 | 用途 |
|---|---|
| `learning/curvature_check.rs` | 单文件 Rust 程序：数值验证球/柱/环面的 K、H，检查滚动球圆角是 G1 还是 G2，并算弦高容差 |
| `learning/diff-geo-plan.md` | 配合 Pressley《Elementary Differential Geometry》的 6 周学习计划（章节、习题、验收、和仓库的对应） |

跑法：

```bash
rustc -O learning/curvature_check.rs -o /tmp/curvature_check && /tmp/curvature_check
```

## 附：这份文档本身

- 放在 `learning/`，默认不提交（`.git/info/exclude` 里没有它，所以 `git status` 会显示为未跟踪；想提交就正常 `git add`，不想就加进 `.git/info/exclude`）。
- 建议边读边改：遇到和代码不符的地方（版本升级），直接改这份文档，它就会变成你自己的笔记。
- 有疑问时优先看 `docs/book/src/dev/` 和 `skills/`，它们随代码一起维护。
