use crate::{CoreError, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;
use std::net::Ipv4Addr;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ipv4Subnet {
    pub network: Ipv4Addr,
    pub prefix: u8,
}

impl Ipv4Subnet {
    pub fn contains(self, address: Ipv4Addr) -> bool {
        let mask = mask(self.prefix);
        (u32::from(address) & mask) == (u32::from(self.network) & mask)
    }

    pub fn intersects(self, other: Ipv4Subnet) -> bool {
        let shared_prefix = self.prefix.min(other.prefix);
        let shared_mask = mask(shared_prefix);
        (u32::from(self.network) & shared_mask) == (u32::from(other.network) & shared_mask)
    }
}

impl fmt::Display for Ipv4Subnet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.network, self.prefix)
    }
}

impl FromStr for Ipv4Subnet {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self> {
        let (network, prefix) = value
            .split_once('/')
            .ok_or_else(|| CoreError::InvalidCidr(value.to_owned()))?;
        let prefix = prefix
            .parse::<u8>()
            .map_err(|_| CoreError::InvalidCidr(value.to_owned()))?;
        if prefix > 32 {
            return Err(CoreError::InvalidCidr(value.to_owned()));
        }
        let network = network
            .parse::<Ipv4Addr>()
            .map_err(|_| CoreError::InvalidIpv4(network.to_owned()))?;
        let normalized = Ipv4Addr::from(u32::from(network) & mask(prefix));
        Ok(Self {
            network: normalized,
            prefix,
        })
    }
}

impl Serialize for Ipv4Subnet {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Ipv4Subnet {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

pub fn mask(prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    }
}

pub fn subnet_for_room(room_id: &str, local_networks: &[Ipv4Subnet]) -> Result<Ipv4Subnet> {
    let pools = ["10.77.0.0/16", "10.88.0.0/16", "172.22.0.0/16"];
    let hash = Sha256::digest(room_id.as_bytes());
    for pool in pools {
        let pool = Ipv4Subnet::from_str(pool)?;
        let subnet_count = 1u32 << (24 - pool.prefix);
        let start = u32::from(hash[0]) % subnet_count;
        for index in 0..subnet_count {
            let offset = (start + index) % subnet_count;
            let candidate = Ipv4Subnet {
                network: Ipv4Addr::from(u32::from(pool.network) + (offset << 8)),
                prefix: 24,
            };
            if !local_networks
                .iter()
                .any(|local| candidate.intersects(*local))
            {
                return Ok(candidate);
            }
        }
    }
    "10.77.0.0/24".parse()
}

pub fn host_address(subnet: Ipv4Subnet) -> Ipv4Addr {
    Ipv4Addr::from(u32::from(subnet.network) + 1)
}

pub fn peer_address(subnet: Ipv4Subnet, ordinal: u32) -> Ipv4Addr {
    Ipv4Addr::from(u32::from(subnet.network) + ordinal + 2)
}

pub fn broadcast_address(subnet: Ipv4Subnet) -> Ipv4Addr {
    Ipv4Addr::from(u32::from(subnet.network) | !mask(subnet.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_subnet_uses_expected_pool() {
        let subnet = subnet_for_room("room-123", &[]).unwrap();
        assert_eq!(subnet.prefix, 24);
        assert!(subnet.to_string().starts_with("10.77."));
    }

    #[test]
    fn room_subnet_avoids_conflict() {
        let original = subnet_for_room("room-123", &[]).unwrap();
        let replacement = subnet_for_room("room-123", &[original]).unwrap();
        assert_ne!(original, replacement);
    }
}
