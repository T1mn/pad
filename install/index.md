# install

- `00_core.sh`：公共基础函数。
- `10_platform.sh`：平台识别。
- `20_release.sh`：release 资产选择、下载与 SHA256SUMS 校验。
- `30_dependencies.sh`：源码构建依赖与 Rust 工具链安装。
- `40_installers.sh`：安装执行细节。
- `50_prompt.sh`：交互提示。
- `90_main.sh`：安装主流程入口。
