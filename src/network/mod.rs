use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

pub mod event;

/// 检查端口是否已经被使用
pub fn is_port_in_use(address: ([u8; 4], u16)) -> bool
{
    let ip = Ipv4Addr::from(address.0);
    let socket = SocketAddrV4::new(ip, address.1);
    TcpListener::bind(socket).is_err()
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// 测试当端口已经被占用时，`is_port_in_use` 函数是否返回 `true`。
    #[test]
    fn is_port_in_use_should_return_true_if_port_is_in_use()
    {
        // 绑定一个随机可用的端口
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        // 获取绑定的端口号
        let port = listener.local_addr().unwrap().port();
        // 检查该端口是否被占用，期望返回 true
        assert!(is_port_in_use(([127, 0, 0, 1], port)));
    }

    /// 测试当端口未被占用时，`is_port_in_use` 函数是否返回 `false`。
    #[test]
    fn is_port_in_use_should_return_false_if_port_is_not_in_use()
    {
        // 绑定一个随机可用的端口
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        // 获取绑定的端口号
        let port = listener.local_addr().unwrap().port();
        // 关闭监听器，以释放端口
        drop(listener); // 关闭监听器，释放端口
                        // 检查该端口是否被占用，期望返回 false
        assert!(!is_port_in_use(([127, 0, 0, 1], port)));
    }

    /// 测试对于无效的 IP 地址，`is_port_in_use` 函数是否返回 `false`。
    #[test]
    fn is_port_in_use_should_return_false_for_invalid_ip()
    {
        // 假设端口 12345 未被占用
        let port = 12345;
        // 检查无效 IP 地址下的端口是否被占用，期望返回 false
        assert!(!is_port_in_use(([0, 0, 0, 0], port)));
    }

    /// 测试当绑定一个通配符 IPv4 地址（0.0.0.0）时，`is_port_in_use` 函数是否返回 `true`。
    #[test]
    fn is_port_in_use_should_return_true_for_bound_ipv4_address()
    {
        // 绑定一个随机可用的端口到 0.0.0.0（表示所有 IPv4 接口）
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        // 获取绑定的端口号
        let port = listener.local_addr().unwrap().port();
        // 检查该端口是否被占用，期望返回 true
        assert!(is_port_in_use(([0, 0, 0, 0], port)));
    }
}
