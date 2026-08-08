# ui/selection/render

- `../render.rs` 内联 `layout`：selection surface 内边距与推荐弹窗高度计算；facade 只外露推荐高度。
- `surface.rs`：selection surface 入口，负责 header/list/footer 区域分发。
- `list.rs`：过滤后列表、空态、行高和 subtitle 行渲染。
