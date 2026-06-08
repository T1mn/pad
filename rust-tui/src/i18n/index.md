# i18n

- `locale.rs`：语言枚举、配置字符串解析、显示名和轮换顺序。
- `en.rs` / `zh_cn.rs` / `zh_tw.rs` / `ja.rs` / `de.rs` / `fr.rs`：各语言键值表。
- `tests.rs`：静态文案键完整性扫描和基础翻译断言。
- `../i18n.rs`：声明语言表模块并提供 `t(locale, key)` 查询入口。
