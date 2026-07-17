// 任何构建形态下都不弹出额外控制台窗口（GUI 子系统）。
// 从终端 / tauri dev 启动时日志仍会经父进程管道输出，不影响开发调试。
#![windows_subsystem = "windows"]

fn main() {
    snapcode_lib::run();
}
