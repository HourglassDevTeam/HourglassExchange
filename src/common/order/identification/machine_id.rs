use pnet::datalink;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
// 从pnet库中导入正确的模块

/// 生成机器ID的函数。
///
/// 该函数会获取机器的MAC地址，并将其进行哈希处理，生成一个唯一的64位标识符。
///
/// # 返回值
///
/// 返回一个64位的无符号整数作为机器ID。如果无法获取MAC地址，则会返回0。
#[allow(dead_code)]
pub fn generate_machine_id() -> Result<u64, String>
{
    // 获取MAC地址
    let mac_address = get_mac_address()?; // 使用 `?` 传播错误
    let mut hasher = DefaultHasher::new();
    mac_address.hash(&mut hasher); // 对MAC地址进行哈希处理
    Ok(hasher.finish()) // 返回唯一的64位标识符
}

/// 获取MAC地址的函数。
///
/// 该函数会遍历系统中的网络接口，找到一个有效的MAC地址。
///
/// # 返回值
///
/// 返回一个`Option<String>`，包含MAC地址的字符串形式。如果未找到有效的MAC地址，返回`None`。
#[allow(dead_code)]
fn get_mac_address() -> Result<String, String>
{
    // 获取所有网络接口
    let interfaces = datalink::interfaces(); // 正确引用pnet::datalink::interfaces
    for iface in interfaces {
        if let Some(mac) = iface.mac {
            // 将MAC地址转换为字符串并排除全为0的情况
            let mac_str = mac.to_string();
            if mac_str != "00:00:00:00:00:00" {
                return Ok(mac_str);
            }
        }
    }
    Err("未找到有效的MAC地址".into()) // 返回错误信息
}

#[cfg(test)]
mod tests
{
    use log::info;
    use super::*;

    /// 测试生成机器ID的函数。
    ///
    /// 该测试会生成一个机器ID，并确保它在同一台机器上是唯一且一致的。
    #[test]
    fn test_generate_machine_id()
    {
        // 生成一个机器ID
        let machine_id = generate_machine_id();
        match machine_id {
            | Ok(id) => {
                info!("本机的机器ID为: {:?}", id);

                // 确保生成的机器ID不是0
                assert_ne!(id, 0, "机器ID不应为0。");

                // 再次生成机器ID，确保与第一次生成的ID一致（因为是在同一台机器上）
                let machine_id_2 = generate_machine_id().unwrap();
                assert_eq!(id, machine_id_2, "机器ID在同一台机器上应该是一致的。");
            }
            | Err(e) => panic!("生成机器ID失败: {}", e),
        }
    }

    /// 测试获取MAC地址的函数。
    ///
    /// 该测试会确保能够成功获取到一个有效的MAC地址。
    #[test]
    fn test_get_mac_address()
    {
        // 确保能够成功获取到MAC地址
        let mac_address = get_mac_address();
        match mac_address {
            | Ok(mac) => {
                info!("本机的MAC地址为: {:?}", mac);
                assert!(!mac.is_empty(), "MAC地址不应为空。");
            }
            | Err(e) => panic!("获取MAC地址失败: {}", e),
        }
    }
}
