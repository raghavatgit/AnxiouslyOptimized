use crate::engine::run_command_timeout;
use crate::models::{DnsProviderResult, PingState, TransactionJournal, TransactionStep};
use crate::system::network::detect_adapters;
use rayon::prelude::*;
use std::net::Ipv4Addr;
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows::Win32::NetworkManagement::IpHelper::{
    IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, ICMP_ECHO_REPLY,
};

pub fn default_dns_providers() -> Vec<DnsProviderResult> {
    vec![
        DnsProviderResult {
            id: "cloudflare".into(),
            name: "Cloudflare Gaming (1.1.1.1)".into(),
            primary_dns: "1.1.1.1".into(),
            secondary_dns: "1.0.0.1".into(),
            description: "Lowest edge latency, privacy-first DNS without telemetry.".into(),
            state: PingState::NotTested,
            is_fastest: false,
        },
        DnsProviderResult {
            id: "cloudflare_sec".into(),
            name: "Cloudflare Security (1.1.1.2)".into(),
            primary_dns: "1.1.1.2".into(),
            secondary_dns: "1.0.0.2".into(),
            description: "Low latency + automatic malware and phishing domain blocking.".into(),
            state: PingState::NotTested,
            is_fastest: false,
        },
        DnsProviderResult {
            id: "google".into(),
            name: "Google Public DNS (8.8.8.8)".into(),
            primary_dns: "8.8.8.8".into(),
            secondary_dns: "8.8.4.4".into(),
            description: "Global anycast routing with high resilience.".into(),
            state: PingState::NotTested,
            is_fastest: false,
        },
        DnsProviderResult {
            id: "quad9".into(),
            name: "Quad9 Secure (9.9.9.9)".into(),
            primary_dns: "9.9.9.9".into(),
            secondary_dns: "149.112.112.112".into(),
            description: "Threat-blocking anycast DNS with DNSSEC validation.".into(),
            state: PingState::NotTested,
            is_fastest: false,
        },
        DnsProviderResult {
            id: "opendns".into(),
            name: "OpenDNS Home (208.67.222.222)".into(),
            primary_dns: "208.67.222.222".into(),
            secondary_dns: "208.67.220.220".into(),
            description: "Cisco enterprise routing backbone.".into(),
            state: PingState::NotTested,
            is_fastest: false,
        },
        DnsProviderResult {
            id: "adguard".into(),
            name: "AdGuard DNS (94.140.14.14)".into(),
            primary_dns: "94.140.14.14".into(),
            secondary_dns: "94.140.15.15".into(),
            description: "Blocks ads, trackers, and analytics at DNS resolution layer.".into(),
            state: PingState::NotTested,
            is_fastest: false,
        },
    ]
}

/// Native Win32 IcmpSendEcho (no PowerShell/ping.exe spawning, handles 0 ms cleanly).
pub fn icmp_ping_ipv4(ip_str: &str, timeout_ms: u32) -> Option<u32> {
    let ip: Ipv4Addr = ip_str.parse().ok()?;
    let dest_addr = u32::from_ne_bytes(ip.octets());

    unsafe {
        let Ok(handle) = IcmpCreateFile() else {
            return None;
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_invalid() {
            return None;
        }

        let payload = [0x41u8; 32];
        let mut reply_buf = vec![0u8; std::mem::size_of::<ICMP_ECHO_REPLY>() + 64];
        let count = IcmpSendEcho(
            handle,
            dest_addr,
            payload.as_ptr() as *const _,
            payload.len() as u16,
            None,
            reply_buf.as_mut_ptr() as *mut _,
            reply_buf.len() as u32,
            timeout_ms,
        );
        let _ = IcmpCloseHandle(handle);

        if count > 0 {
            let reply = &*(reply_buf.as_ptr() as *const ICMP_ECHO_REPLY);
            if reply.Status == 0 {
                return Some(reply.RoundTripTime);
            }
        }
        None
    }
}

pub fn benchmark_all_providers() -> Vec<DnsProviderResult> {
    let mut providers = default_dns_providers();
    providers.par_iter_mut().for_each(|p| {
        const ATTEMPTS: u32 = 4;
        let mut ok_count = 0u32;
        let mut sum_rtt = 0u32;
        for _ in 0..ATTEMPTS {
            if let Some(ms) = icmp_ping_ipv4(&p.primary_dns, 1200) {
                ok_count += 1;
                sum_rtt = sum_rtt.saturating_add(ms);
            }
        }
        if ok_count > 0 {
            let avg = sum_rtt / ok_count;
            let loss = (((ATTEMPTS - ok_count) * 100) / ATTEMPTS) as u8;
            p.state = PingState::Measured {
                rtt_ms: avg,
                loss_pct: loss,
            };
        } else {
            p.state = PingState::Failed;
        }
        p.is_fastest = false;
    });

    mark_fastest_provider(&mut providers);
    providers
}

pub fn mark_fastest_provider(providers: &mut [DnsProviderResult]) {
    for p in providers.iter_mut() {
        p.is_fastest = false;
    }

    // Prefer 0% packet loss lowest RTT (0 ms IS valid!), fallback to lowest RTT with <100% loss
    let best_idx = providers
        .iter()
        .enumerate()
        .filter_map(|(i, p)| match p.state {
            PingState::Measured { rtt_ms, loss_pct } if loss_pct == 0 => Some((i, rtt_ms)),
            _ => None,
        })
        .min_by_key(|(_, ms)| *ms)
        .or_else(|| {
            providers
                .iter()
                .enumerate()
                .filter_map(|(i, p)| match p.state {
                    PingState::Measured { rtt_ms, .. } => Some((i, rtt_ms)),
                    _ => None,
                })
                .min_by_key(|(_, ms)| *ms)
        })
        .map(|(i, _)| i);

    if let Some(i) = best_idx {
        providers[i].is_fastest = true;
    }
}

pub fn apply_dns_to_adapter(
    journal: &mut TransactionJournal,
    adapter_name: &str,
    primary: &str,
    secondary: &str,
) -> Result<(), String> {
    let mut diag = Vec::new();
    let adapters = detect_adapters(&mut diag);
    let current_servers = adapters
        .iter()
        .find(|a| a.friendly_name.eq_ignore_ascii_case(adapter_name))
        .map(|a| a.dns_servers.clone())
        .unwrap_or_default();

    let name_arg = format!("name={adapter_name}");
    let out1 = run_command_timeout(
        "netsh.exe",
        &["interface", "ip", "set", "dns", &name_arg, "static", primary],
        6000,
    )?;
    if out1.exit_code != 0 {
        return Err(format!(
            "Failed setting primary DNS on '{adapter_name}' (exit {}): {} {}",
            out1.exit_code,
            out1.stdout.trim(),
            out1.stderr.trim()
        ));
    }

    let out2 = run_command_timeout(
        "netsh.exe",
        &[
            "interface",
            "ip",
            "add",
            "dns",
            &name_arg,
            secondary,
            "index=2",
        ],
        6000,
    )?;
    if out2.exit_code != 0 {
        return Err(format!(
            "Failed adding secondary DNS on '{adapter_name}' (exit {}): {} {}",
            out2.exit_code,
            out2.stdout.trim(),
            out2.stderr.trim()
        ));
    }

    let _ = flush_dns_cache();
    journal.steps.push(TransactionStep::DnsConfig {
        adapter_name: adapter_name.to_string(),
        previous_servers: current_servers,
        was_dhcp: false,
    });
    Ok(())
}

pub fn reset_dns_dhcp(adapter_name: &str) -> Result<(), String> {
    let name_arg = format!("name={adapter_name}");
    let out = run_command_timeout(
        "netsh.exe",
        &["interface", "ip", "set", "dns", &name_arg, "dhcp"],
        6000,
    )?;
    if out.exit_code != 0 {
        return Err(format!(
            "Failed resetting IPv4 DNS to DHCP on '{adapter_name}': {} {}",
            out.stdout.trim(),
            out.stderr.trim()
        ));
    }
    let _ = run_command_timeout(
        "netsh.exe",
        &["interface", "ipv6", "set", "dns", &name_arg, "dhcp"],
        4000,
    );
    flush_dns_cache()?;
    Ok(())
}

pub fn flush_dns_cache() -> Result<(), String> {
    let out = run_command_timeout("ipconfig.exe", &["/flushdns"], 5000)?;
    if out.exit_code != 0 {
        return Err(format!(
            "ipconfig /flushdns failed (exit {}): {}",
            out.exit_code,
            out.stderr.trim()
        ));
    }
    Ok(())
}

pub fn reset_winsock_stack() -> Result<String, String> {
    let out1 = run_command_timeout("netsh.exe", &["winsock", "reset"], 10_000)?;
    let out2 = run_command_timeout("netsh.exe", &["int", "ip", "reset"], 10_000)?;
    if out1.exit_code == 0 && out2.exit_code == 0 {
        Ok("Winsock catalog & TCP/IP stack reset successfully. Reboot recommended.".to_string())
    } else {
        Err(format!(
            "Winsock/IP reset errors: {} {}",
            out1.stderr.trim(),
            out2.stderr.trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dns_benchmark_zero_ms_and_loss_tests() {
        let mut list = default_dns_providers();
        // Provider 0: 0 ms (<1 ms fiber) -> MUST be valid and chosen fastest!
        list[0].state = PingState::Measured {
            rtt_ms: 0,
            loss_pct: 0,
        };
        list[1].state = PingState::Measured {
            rtt_ms: 14,
            loss_pct: 0,
        };
        list[2].state = PingState::Failed;

        mark_fastest_provider(&mut list);
        assert!(list[0].is_fastest);
        assert!(!list[1].is_fastest);
        assert_eq!(list[2].state, PingState::Failed);
    }
}
