#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum ClientStatus {
    Connected, // 已连接
    // CancelOnly, // 仅取消
    Disconnected,
    Pending, // 待定，正在尝试连接
    // Suspended,    // 已暂停，暂时禁止所有操作
    Reconnecting, // 正在重连
    // Error,        // 发生错误，无法正常操作
    // Maintenance,  // 维护模式，系统暂时不可用
    Authenticated, // 已认证，已通过身份验证
    Unauthorized,  // 未授权，身份验证失败或权限不足
}
