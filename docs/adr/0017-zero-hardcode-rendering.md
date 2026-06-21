# ADR-0017: 核心零硬编码渲染

- **状态**: 已采纳
- **日期**: 2026-06-21
- **版本**: v1.4

## 背景

v1.3.1 阶段实施 spec B1 时，3 处文件直接调 ratatui API（违反「零硬编码」原则），留 3 处 TODO 注释待 v1.4 重构。`atlas` 项目（独立失败项目）TUI 重构 3 次失败教训：架构分层没锁死是根因。

## 决策

eflow 渲染管线采用电脑主机三层架构（Core → RenderEngine → RenderBackend）：

1. **软件核心**（wizard/SelectList/TuiBackend 4 段布局）输出 **ViewModel**（纯数据，零坐标/面积信息）
2. **RenderEngine**（默认 impl：DefaultRenderEngine）翻译 ViewModel → **DrawCommand**（硬编码颜色/前缀/边框）
3. **RenderBackend**（默认 impl：TuiBackend = ratatui）机械执行 DrawCommand 画到屏幕

### 铁律

- 核心层（wizard step / SelectList / TuiState）**零 ratatui import**
- RenderBackend **零业务知识**（不判断配置状态、不渲染业务文本）
- Modal 弹窗走完整 `FrameViewModel::Modal { background, popup }` 路径
- 禁止任何「先 hardcode + TODO」临时路径

### 技术细节

- **ViewModel** 是纯数据 struct，不含 `Rect`/`Buffer` 等渲染类型
- **DrawCommand** 是 5 种指令 enum：Text, Block, Span, Line, ClearArea
- **RenderEngine trait** 定义 `render(&self, vm: &FrameViewModel) -> Vec<DrawCommand>`
- **TuiBackend 不 impl RenderBackend trait**：因 `Terminal<CrosstermBackend>` 生命周期与 trait 不兼容，改用模块内 `execute_draw_commands()` 辅助函数
- **WizardStep trait** 从 `render(Rect, Buffer, &State)` 改为 `view_model(&State) → StepViewModel`

## 后果

### 好的

- 换 RenderBackend（egui/web）只需重写最后一层
- 测试可在不启动 ratatui 的情况下验证 ViewModel 和 RenderEngine
- 业务代码（wizard step）零渲染知识

### 不好的

- 多一层转换（ViewModel → DrawCommand）
- 调试时需要跟踪多层数据流

## 备选方案（已否决）

| 方案 | 否决原因 |
|------|----------|
| A. 2 层架构（ViewModel → RenderBackend） | 失去多 backend 灵活性 |
| B. 4 层架构（+ ThemeToken 抽象） | 过度工程化 |
| C. 保留 v1.3.1 临时 hardcode | 重蹈 atlas 失败 |
