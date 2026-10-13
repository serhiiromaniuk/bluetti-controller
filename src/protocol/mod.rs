//! Bluetti BLE protocol: Modbus-RTU frames over a GATT channel, encrypted on V2 stations.

pub mod crypto;
pub mod device;
pub mod identity;
pub mod modbus;
