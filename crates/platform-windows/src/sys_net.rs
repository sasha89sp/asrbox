use std::collections::VecDeque;
use std::ffi::c_void;
use std::mem;
use std::time::Instant;

use asrbox_platform_api::NetTelemetry;
use windows::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetAdaptersAddresses, GetIfTable2, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
    GAA_FLAG_INCLUDE_GATEWAYS, ICMP_ECHO_REPLY, IF_TYPE_SOFTWARE_LOOPBACK, IP_ADAPTER_ADDRESSES_LH,
    MIB_IF_TABLE2,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};

const PING_HISTORY_LEN: usize = 10;
const PING_INTERVAL_SEC: u64 = 4;

pub(crate) struct NetSampler {
    prev_rx: u64,
    prev_tx: u64,
    has_prev_bytes: bool,
    last_bytes_ts: Instant,
    last_ping_ts: Option<Instant>,
    gw_ping_ms: Option<u16>,
    inet_history: VecDeque<Option<u16>>,
}

impl NetSampler {
    pub(crate) fn new() -> Self {
        Self {
            prev_rx: 0,
            prev_tx: 0,
            has_prev_bytes: false,
            last_bytes_ts: Instant::now(),
            last_ping_ts: None,
            gw_ping_ms: None,
            inet_history: VecDeque::with_capacity(PING_HISTORY_LEN),
        }
    }

    pub(crate) fn sample(&mut self) -> NetTelemetry {
        let (ip_v4, gw_ipv4, vpn_active) = query_adapter_info();
        let (rx_kbps, tx_kbps) = self.sample_throughput();

        let should_ping = self
            .last_ping_ts
            .is_none_or(|t| t.elapsed().as_secs() >= PING_INTERVAL_SEC);
        if should_ping {
            self.last_ping_ts = Some(Instant::now());
            self.gw_ping_ms = gw_ipv4.and_then(|ip| icmp_ping_ipv4(ip, 200));
            let cf_ip = u32::from_ne_bytes([1, 1, 1, 1]);
            let inet = icmp_ping_ipv4(cf_ip, 300);
            if self.inet_history.len() >= PING_HISTORY_LEN {
                self.inet_history.pop_front();
            }
            self.inet_history.push_back(inet);
        }

        let (inet_ping_ms, inet_loss_pct, inet_jitter_ms) = summarize_ping(&self.inet_history);
        NetTelemetry {
            ip_v4,
            vpn_active,
            rx_kbps,
            tx_kbps,
            gw_ping_ms: self.gw_ping_ms,
            inet_ping_ms,
            inet_loss_pct,
            inet_jitter_ms,
            timer_res_us: 0,
            sample_us: 0,
        }
    }

    fn sample_throughput(&mut self) -> (u32, u32) {
        let (rx, tx) = query_total_net_bytes();
        let dt = self.last_bytes_ts.elapsed().as_secs_f64().max(0.1);
        self.last_bytes_ts = Instant::now();
        if !self.has_prev_bytes {
            self.prev_rx = rx;
            self.prev_tx = tx;
            self.has_prev_bytes = true;
            return (0, 0);
        }
        let d_rx = rx.saturating_sub(self.prev_rx) as f64;
        let d_tx = tx.saturating_sub(self.prev_tx) as f64;
        self.prev_rx = rx;
        self.prev_tx = tx;
        let rx_kbps = ((d_rx * 8.0) / 1000.0 / dt).round() as u32;
        let tx_kbps = ((d_tx * 8.0) / 1000.0 / dt).round() as u32;
        (rx_kbps, tx_kbps)
    }
}

fn summarize_ping(hist: &VecDeque<Option<u16>>) -> (Option<u16>, u8, u16) {
    if hist.is_empty() {
        return (None, 0, 0);
    }
    let last = hist.back().copied().flatten();
    let total = hist.len();
    let lost = hist.iter().filter(|x| x.is_none()).count();
    let loss_pct = ((lost * 100) / total) as u8;
    let ok_vals: Vec<u16> = hist.iter().filter_map(|&v| v).collect();
    if ok_vals.len() < 2 {
        return (last, loss_pct, 0);
    }
    let mut diff_sum = 0u32;
    for pair in ok_vals.windows(2) {
        diff_sum += u32::from(pair[0].abs_diff(pair[1]));
    }
    let jitter = (diff_sum / (ok_vals.len() as u32 - 1)) as u16;
    (last, loss_pct, jitter)
}

fn query_total_net_bytes() -> (u64, u64) {
    let mut table_ptr: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    // SAFETY: GetIfTable2 allocates MIB_IF_TABLE2 which is freed via FreeMibTable before returning.
    unsafe {
        if GetIfTable2(&mut table_ptr).0 != 0 || table_ptr.is_null() {
            return (0, 0);
        }
        let num = (*table_ptr).NumEntries as usize;
        let rows = std::slice::from_raw_parts((*table_ptr).Table.as_ptr(), num);
        let (mut rx, mut tx) = (0u64, 0u64);
        for row in rows {
            if row.OperStatus != IfOperStatusUp || row.Type == IF_TYPE_SOFTWARE_LOOPBACK {
                continue;
            }
            if row.InterfaceAndOperStatusFlags._bitfield & 0x01 == 0 {
                // Non-hardware filter interfaces can double-count; keep physical/tunnel interfaces with traffic.
                if row.Type != 53 && row.Type != 6 && row.Type != 71 {
                    continue;
                }
            }
            rx = rx.saturating_add(row.InOctets);
            tx = tx.saturating_add(row.OutOctets);
        }
        FreeMibTable(table_ptr as *const c_void);
        (rx, tx)
    }
}

fn query_adapter_info() -> (String, Option<u32>, bool) {
    let mut size = 16_384u32;
    let mut buf = vec![0u8; size as usize];
    // SAFETY: GetAdaptersAddresses writes linked IP_ADAPTER_ADDRESSES_LH structs inside `buf`.
    unsafe {
        let mut res = GetAdaptersAddresses(
            u32::from(AF_INET.0),
            GAA_FLAG_INCLUDE_GATEWAYS,
            None,
            Some(buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
            &mut size,
        );
        if res == 111 && size > 0 {
            buf.resize(size as usize, 0);
            res = GetAdaptersAddresses(
                u32::from(AF_INET.0),
                GAA_FLAG_INCLUDE_GATEWAYS,
                None,
                Some(buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut size,
            );
        }
        if res != 0 {
            return ("—".to_owned(), None, false);
        }
        let mut primary_ip: Option<String> = None;
        let mut primary_gw: Option<u32> = None;
        let mut vpn_active = false;
        let mut cur = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
        while !cur.is_null() {
            let ad = &*cur;
            if ad.OperStatus == IfOperStatusUp && ad.IfType != IF_TYPE_SOFTWARE_LOOPBACK {
                let desc = pwstr_to_lower(ad.Description.0);
                let friendly = pwstr_to_lower(ad.FriendlyName.0);
                let is_vpn = ad.IfType == 53
                    || desc.contains("vpn")
                    || desc.contains("wireguard")
                    || desc.contains("wintun")
                    || desc.contains("tap-")
                    || desc.contains("tailscale")
                    || desc.contains("zerotier")
                    || friendly.contains("vpn");
                if is_vpn {
                    vpn_active = true;
                }
                let gw = first_ipv4_from_list(ad.FirstGatewayAddress);
                let ip = first_ipv4_from_unicast(ad.FirstUnicastAddress);
                if primary_ip.is_none() || (primary_gw.is_none() && gw.is_some()) {
                    if let Some(addr) = ip {
                        let oct = addr.to_ne_bytes();
                        if oct[0] != 169 {
                            primary_ip =
                                Some(format!("{}.{}.{}.{}", oct[0], oct[1], oct[2], oct[3]));
                            if gw.is_some() {
                                primary_gw = gw;
                            }
                        }
                    }
                }
            }
            cur = ad.Next;
        }
        (
            primary_ip.unwrap_or_else(|| "—".to_owned()),
            primary_gw,
            vpn_active,
        )
    }
}

unsafe fn first_ipv4_from_unicast(
    mut ptr: *const windows::Win32::NetworkManagement::IpHelper::IP_ADAPTER_UNICAST_ADDRESS_LH,
) -> Option<u32> {
    while !ptr.is_null() {
        let sa = (*ptr).Address.lpSockaddr;
        if !sa.is_null() && (*sa).sa_family == AF_INET {
            let sin = &*(sa as *const SOCKADDR_IN);
            return Some(sin.sin_addr.S_un.S_addr);
        }
        ptr = (*ptr).Next;
    }
    None
}

unsafe fn first_ipv4_from_list(
    mut ptr: *const windows::Win32::NetworkManagement::IpHelper::IP_ADAPTER_GATEWAY_ADDRESS_LH,
) -> Option<u32> {
    while !ptr.is_null() {
        let sa = (*ptr).Address.lpSockaddr;
        if !sa.is_null() && (*sa).sa_family == AF_INET {
            let sin = &*(sa as *const SOCKADDR_IN);
            if sin.sin_addr.S_un.S_addr != 0 {
                return Some(sin.sin_addr.S_un.S_addr);
            }
        }
        ptr = (*ptr).Next;
    }
    None
}

fn icmp_ping_ipv4(dest_ip: u32, timeout_ms: u32) -> Option<u16> {
    // SAFETY: Opening ICMP handle, sending 8-byte echo into sized reply buffer, closing handle.
    unsafe {
        let h = IcmpCreateFile().ok()?;
        let payload = [0x61u8; 8];
        let reply_size = mem::size_of::<ICMP_ECHO_REPLY>() + payload.len() + 16;
        let mut reply_buf = vec![0u8; reply_size];
        let count = IcmpSendEcho(
            h,
            dest_ip,
            payload.as_ptr() as *const c_void,
            payload.len() as u16,
            None,
            reply_buf.as_mut_ptr() as *mut c_void,
            reply_buf.len() as u32,
            timeout_ms,
        );
        let _ = IcmpCloseHandle(h);
        if count == 0 {
            return None;
        }
        let reply = &*(reply_buf.as_ptr() as *const ICMP_ECHO_REPLY);
        if reply.Status == 0 {
            u16::try_from(reply.RoundTripTime).ok()
        } else {
            None
        }
    }
}

unsafe fn pwstr_to_lower(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while len < 256 && *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len)).to_ascii_lowercase()
}
