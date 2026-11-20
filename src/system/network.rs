use crate::engine::from_wide_null;
use crate::models::NetworkAdapterInfo;
use std::ffi::CStr;
use std::net::{Ipv4Addr, Ipv6Addr};
use windows::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6};

const IF_TYPE_ETHERNET: u32 = 6;
const IF_TYPE_LOOPBACK: u32 = 24;
const IF_TYPE_WIFI: u32 = 71;
const IF_TYPE_TUNNEL: u32 = 131;

pub fn detect_adapters(diagnostics: &mut Vec<String>) -> Vec<NetworkAdapterInfo> {
    let mut out = Vec::new();
    unsafe {
        let mut size = 32_768u32;
        let mut buf = vec![0u8; size as usize];
        let flags = GAA_FLAG_INCLUDE_GATEWAYS;
        let mut status = GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            flags,
            None,
            Some(buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
            &mut size,
        );
        if status == ERROR_BUFFER_OVERFLOW.0 {
            buf.resize(size as usize, 0);
            status = GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                flags,
                None,
                Some(buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut size,
            );
        }
        if status != 0 {
            diagnostics.push(format!("GetAdaptersAddresses failed with error {status}"));
            return out;
        }

        let mut cur = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
        while !cur.is_null() {
            let a = &*cur;
            cur = a.Next;

            if a.OperStatus != IfOperStatusUp {
                continue;
            }
            if a.IfType == IF_TYPE_LOOPBACK || a.IfType == IF_TYPE_TUNNEL {
                continue;
            }

            let friendly_name = if !a.FriendlyName.0.is_null() {
                pwstr_to_string(a.FriendlyName.0)
            } else {
                "Network Adapter".to_string()
            };
            let description = if !a.Description.0.is_null() {
                pwstr_to_string(a.Description.0)
            } else {
                String::new()
            };
            let guid = if !a.AdapterName.0.is_null() {
                CStr::from_ptr(a.AdapterName.0 as *const i8)
                    .to_string_lossy()
                    .trim()
                    .to_string()
            } else {
                String::new()
            };

            let mac_len = (a.PhysicalAddressLength as usize).min(8);
            let mac_address = a.PhysicalAddress[..mac_len]
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(":");

            let mut ipv4_addrs = Vec::new();
            let mut ipv6_addrs = Vec::new();
            let mut uc = a.FirstUnicastAddress;
            while !uc.is_null() {
                let u = &*uc;
                uc = u.Next;
                if let Some((ip, is_v6)) = sockaddr_to_string(u.Address.lpSockaddr) {
                    if is_v6 {
                        ipv6_addrs.push(ip);
                    } else {
                        ipv4_addrs.push(ip);
                    }
                }
            }

            let mut gateways = Vec::new();
            let mut gw = a.FirstGatewayAddress;
            while !gw.is_null() {
                let g = &*gw;
                gw = g.Next;
                if let Some((ip, _)) = sockaddr_to_string(g.Address.lpSockaddr) {
                    gateways.push(ip);
                }
            }

            let mut dns_servers = Vec::new();
            let mut dns = a.FirstDnsServerAddress;
            while !dns.is_null() {
                let d = &*dns;
                dns = d.Next;
                if let Some((ip, _)) = sockaddr_to_string(d.Address.lpSockaddr) {
                    if !dns_servers.contains(&ip) {
                        dns_servers.push(ip);
                    }
                }
            }

            let is_virtual = is_virtual_adapter(&friendly_name, &description);
            let adapter_kind = if is_virtual {
                "Virtual".to_string()
            } else {
                match a.IfType {
                    IF_TYPE_ETHERNET => "Ethernet".to_string(),
                    IF_TYPE_WIFI => "Wi-Fi".to_string(),
                    _ => "Network".to_string(),
                }
            };

            let link_speed_mbps = if a.TransmitLinkSpeed == u64::MAX {
                0
            } else {
                a.TransmitLinkSpeed / 1_000_000
            };

            let is_physical_primary =
                !is_virtual && !gateways.is_empty() && (!ipv4_addrs.is_empty() || !ipv6_addrs.is_empty());

            out.push(NetworkAdapterInfo {
                friendly_name,
                description,
                guid,
                adapter_kind,
                is_physical_primary,
                link_speed_mbps,
                ipv4_addrs,
                ipv6_addrs,
                gateways,
                dns_servers,
                mac_address,
            });
        }
    }

    // Put primary physical adapter with gateway first, then physical Ethernet before Wi-Fi before Virtual
    out.sort_by_key(|n| {
        let primary_rank = if n.is_physical_primary { 0 } else { 1 };
        let type_rank = match n.adapter_kind.as_str() {
            "Ethernet" => 0,
            "Wi-Fi" => 1,
            "Network" => 2,
            _ => 3,
        };
        (primary_rank, type_rank)
    });

    // Ensure at least the top physical one stays flagged primary if there were multiple physical gateways
    let mut marked = false;
    for nic in &mut out {
        if nic.is_physical_primary {
            if !marked {
                marked = true;
            } else {
                nic.is_physical_primary = false;
            }
        }
    }

    let _ = from_wide_null(&[]);
    out
}

fn is_virtual_adapter(friendly: &str, desc: &str) -> bool {
    let combined = format!("{} {}", friendly, desc).to_lowercase();
    const VIRTUAL_MARKERS: &[&str] = &[
        "vethernet",
        "wsl",
        "hyper-v",
        "vmware",
        "virtualbox",
        "loopback",
        "tap-",
        "wintun",
        "wireguard",
        "zerotier",
        "hamachi",
        "radmin",
        "nordlynx",
        "tailscale",
    ];
    VIRTUAL_MARKERS.iter().any(|m| combined.contains(m))
}

unsafe fn pwstr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while *ptr.add(len) != 0 && len < 512 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}

unsafe fn sockaddr_to_string(
    sa: *const windows::Win32::Networking::WinSock::SOCKADDR,
) -> Option<(String, bool)> {
    if sa.is_null() {
        return None;
    }
    let family = (*sa).sa_family;
    if family == AF_INET {
        let sin = &*(sa as *const SOCKADDR_IN);
        let octets = sin.sin_addr.S_un.S_addr.to_ne_bytes();
        let ip = Ipv4Addr::from(octets);
        Some((ip.to_string(), false))
    } else if family == AF_INET6 {
        let sin6 = &*(sa as *const SOCKADDR_IN6);
        let ip = Ipv6Addr::from(sin6.sin6_addr.u.Byte);
        // Filter out fec0:: deprecated Windows DNS site-local probes
        if ip.segments()[0] == 0xfec0 {
            return None;
        }
        Some((ip.to_string(), true))
    } else {
        None
    }
}
