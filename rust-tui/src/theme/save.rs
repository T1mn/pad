pub(crate) mod render;

use super::*;

impl Config {
    /// 原子写 config.toml（同目录临时文件 -> 0600 -> fsync -> rename）。
    ///
    /// 返回 `io::Result` 而不是吞错误：config.toml 里存着 api_key / bot_token，
    /// 静默失败会让用户以为改动已经保存。
    pub fn save(&self) -> std::io::Result<()> {
        crate::atomic_file::write_private(&Self::config_path(), self.to_toml_string())
    }

    /// 无法向用户弹提示的调用点（后台 daemon 等）用这个：失败写 debug log，不 panic。
    pub fn save_or_log(&self) -> bool {
        match self.save() {
            Ok(()) => true,
            Err(err) => {
                crate::log_debug!(
                    "config: save failed at {}: {}",
                    Self::config_path().display(),
                    err
                );
                false
            }
        }
    }

    pub fn to_toml_string(&self) -> String {
        render::render(self)
    }
}
