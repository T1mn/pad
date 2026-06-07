# app/preview

- `cache.rs`：预览渲染缓存失效、导航 debounce 与 thread cache 裁剪。
- `detail_cache.rs`：Session detail 渲染缓存命中、LRU 提升与 turn allocation 快速匹配。
- `focus.rs`：预览/面板焦点切换，以及 Tab 返回时间窗口记录。
- `turns.rs`：Session turn 状态判断、打开详情与恢复列表。
- `turn_selection.rs`：Session turn 上下选择、折叠与返回逻辑。
- `scroll.rs`：普通预览、turn 列表与详情滚动入口。
- `tick.rs`：预览刷新暂停判断、busy 动画 tick 与目标帧率。
- `mouse.rs`：预览区鼠标选择生命周期。
- `preview_tests.rs`：预览测试入口与共享 helper。
- `preview_tests/`：预览打开、缓存、滚动、tick 行为测试分组。
