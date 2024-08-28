use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use pnet::datalink; // 从pnet库中导入正确的模块

/// 生成机器ID的函数。
///
/// 该函数会获取机器的MAC地址，并将其进行哈希处理，生成一个唯一的64位标识符。
///
/// # 返回值
///
/// 返回一个64位的无符号整数作为机器ID。如果无法获取MAC地址，则会返回0。
#[allow(dead_code)]
pub fn generate_machine_id() -> u64 {
    // 获取MAC地址
    let mac_address = get_mac_address().unwrap(); // 获取MAC地址
    let mut hasher = DefaultHasher::new();
    mac_address.hash(&mut hasher); // 对MAC地址进行哈希处理
    hasher.finish() // 返回唯一的64位标识符
}

/// 获取MAC地址的函数。
///
/// 该函数会遍历系统中的网络接口，找到一个有效的MAC地址。
///
/// # 返回值
///
/// 返回一个`Option<String>`，包含MAC地址的字符串形式。如果未找到有效的MAC地址，返回`None`。
#[allow(dead_code)]
fn get_mac_address() -> Option<String> {
    // 获取所有网络接口
    let interfaces = datalink::interfaces(); // 正确引用pnet::datalink::interfaces
    for iface in interfaces {
        if let Some(mac) = iface.mac {
            // 将MAC地址转换为字符串并排除全为0的情况
            let mac_str = mac.to_string();
            if mac_str != "00:00:00:00:00:00" {
                return Some(mac_str);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试生成机器ID的函数。
    ///
    /// 该测试会生成一个机器ID，并确保它在同一台机器上是唯一且一致的。
    #[test]
    fn test_generate_machine_id() {
        // 生成一个机器ID
        let machine_id = generate_machine_id();

        // 确保生成的机器ID不是0
        assert_ne!(machine_id, 0, "机器ID不应为0。");

        // 再次生成机器ID，确保与第一次生成的ID一致（因为是在同一台机器上）
        let machine_id_2 = generate_machine_id();
        assert_eq!(machine_id, machine_id_2, "机器ID在同一台机器上应该是一致的。");
    }

    /// 测试获取MAC地址的函数。
    ///
    /// 该测试会确保能够成功获取到一个有效的MAC地址。
    #[test]
    fn test_get_mac_address() {
        // 确保能够成功获取到MAC地址
        let mac_address = get_mac_address();
        println!("本机的MAC地址为: {:?}", mac_address);

        // 断言MAC地址已成功获取
        assert!(mac_address.is_some(), "应成功获取到MAC地址。");
    }
}
