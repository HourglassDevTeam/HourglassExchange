use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use pnet::datalink; // Import the correct module from the pnet crate

#[allow(dead_code)]
pub fn generate_machine_id() -> u64 {
    let mac_address = get_mac_address().unwrap(); // Retrieve MAC address
    let mut hasher = DefaultHasher::new();
    mac_address.hash(&mut hasher); // Hash the MAC address
    hasher.finish() // Returns a unique 64-bit identifier
}

#[allow(dead_code)]
fn get_mac_address() -> Option<String> {
    let interfaces = datalink::interfaces(); // Correctly refer to pnet::datalink::interfaces
    for iface in interfaces {
        if let Some(mac) = iface.mac {
            return Some(mac.to_string());
        }
    }
    None
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_machine_id() {
        // Generate a machine ID
        let machine_id = generate_machine_id();
        // Ensure the machine ID is non-zero
        assert_ne!(machine_id, 0, "Machine ID should not be zero.");

        // Generate another machine ID and ensure it's the same as the first one (since it's the same machine)
        let machine_id_2 = generate_machine_id();
        assert_eq!(machine_id, machine_id_2, "Machine ID should be consistent on the same machine.");
    }

    #[test]
    fn test_get_mac_address() {
        // Ensure that a MAC address is retrieved successfully
        let mac_address = get_mac_address();
        assert!(mac_address.is_some(), "MAC address should be retrieved.");
    }
}