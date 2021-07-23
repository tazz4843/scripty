use crate::METRICS;
use std::ops::Add;
use std::time::Duration;
use systemstat::{
    BlockDeviceStats, ByteSize, CPULoad, LoadAverage, NetworkStats, Platform, SocketStats,
};
use tokio::time;

const ONE_SECOND: Duration = Duration::from_secs(1);

pub fn spawn_updater_task() {
    tokio::spawn(async move {
        loop {
            updater_task().await;
            time::sleep(Duration::from_secs(5)).await;
        }
    });
}

async fn updater_task() {
    let metrics = unsafe { METRICS.get().unwrap_unchecked() };

    let sys = systemstat::System::new();

    if let Ok(temp) = sys.cpu_temp() {
        metrics.cpu_temp.set(temp as f64)
    }

    if let Ok(block_devices) = sys.block_device_statistics() {
        if block_devices.len() == 0 {
            tracing::warn!("no block devices found? should be at least one");
        }

        let mut general_stats = BlockDeviceStats {
            name: "all".to_string(),
            read_ios: 0,
            read_merges: 0,
            read_sectors: 0,
            read_ticks: 0,
            write_ios: 0,
            write_merges: 0,
            write_sectors: 0,
            write_ticks: 0,
            in_flight: 0,
            io_ticks: 0,
            time_in_queue: 0,
        };

        for (_, stats) in block_devices {
            general_stats.read_ios += stats.read_ios;
            general_stats.read_merges += stats.read_merges;
            general_stats.read_sectors += stats.read_sectors;
            general_stats.read_ticks += stats.read_ticks;
            general_stats.write_ios += stats.write_ios;
            general_stats.write_merges += stats.write_merges;
            general_stats.write_sectors += stats.write_sectors;
            general_stats.write_ticks += stats.write_ticks;
            general_stats.in_flight += stats.in_flight;
            general_stats.io_ticks += stats.io_ticks;
            general_stats.time_in_queue += stats.time_in_queue;
        }

        let BlockDeviceStats {
            read_ios,
            read_merges,
            read_sectors,
            read_ticks,
            write_ios,
            write_merges,
            write_sectors,
            write_ticks,
            in_flight,
            io_ticks,
            time_in_queue,
            ..
        } = general_stats;

        metrics.block_stats.read_ios.set(read_ios as i64);
        metrics.block_stats.read_merges.set(read_merges as i64);
        metrics.block_stats.read_sectors.set(read_sectors as i64);
        metrics.block_stats.read_ticks.set(read_ticks as i64);
        metrics.block_stats.write_ios.set(write_ios as i64);
        metrics.block_stats.write_merges.set(write_merges as i64);
        metrics.block_stats.write_sectors.set(write_sectors as i64);
        metrics.block_stats.write_ticks.set(write_ticks as i64);
        metrics.block_stats.in_flight.set(in_flight as i64);
        metrics.block_stats.io_ticks.set(io_ticks as i64);
        metrics.block_stats.time_in_queue.set(time_in_queue as i64);
    }

    if let Ok(delayed_cpu_load) = sys.cpu_load_aggregate() {
        tokio::time::sleep(ONE_SECOND).await;
        if let Ok(CPULoad {
            user,
            nice,
            system,
            interrupt,
            idle,
            platform,
        }) = delayed_cpu_load.done()
        {
            metrics.cpu_usage.user.set(user as f64);
            metrics.cpu_usage.idle.set(idle as f64);
            metrics.cpu_usage.interrupt.set(interrupt as f64);
            metrics.cpu_usage.nice.set(nice as f64);
            metrics.cpu_usage.system.set(system as f64);
            metrics.cpu_usage.iowait.set(platform.iowait as f64);
        }
    }

    if let Ok(LoadAverage { one, five, fifteen }) = sys.load_average() {
        metrics.load_avg_stats.one.set(one as f64);
        metrics.load_avg_stats.five.set(five as f64);
        metrics.load_avg_stats.fifteen.set(fifteen as f64);
    }

    if let Ok(memory) = sys.memory() {
        metrics.mem_usage.total.set(memory.total.0 as i64);
        metrics.mem_usage.free.set(memory.free.0 as i64);
        metrics.mem_usage.active.set(
            memory
                .platform_memory
                .meminfo
                .get("Active")
                .unwrap_or(&ByteSize(0))
                .0 as i64,
        );
        metrics.mem_usage.active_anon.set(
            memory
                .platform_memory
                .meminfo
                .get("Active(anon)")
                .unwrap_or(&ByteSize(0))
                .0 as i64,
        );
        metrics.mem_usage.active_file.set(
            memory
                .platform_memory
                .meminfo
                .get("Active(file)")
                .unwrap_or(&ByteSize(0))
                .0 as i64,
        );
        metrics.mem_usage.buffer.set(
            memory
                .platform_memory
                .meminfo
                .get("Buffers")
                .unwrap_or(&ByteSize(0))
                .0 as i64,
        );
        metrics.mem_usage.cache.set(
            memory
                .platform_memory
                .meminfo
                .get("Cached")
                .unwrap_or(&ByteSize(0))
                .0 as i64,
        );
        metrics.mem_usage.inactive.set(
            memory
                .platform_memory
                .meminfo
                .get("Inactive")
                .unwrap_or(&ByteSize(0))
                .0 as i64,
        );
        metrics.mem_usage.inactive_anon.set(
            memory
                .platform_memory
                .meminfo
                .get("Inactive(anon)")
                .unwrap_or(&ByteSize(0))
                .0 as i64,
        );
        metrics.mem_usage.inactive_file.set(
            memory
                .platform_memory
                .meminfo
                .get("Inactive(file)")
                .unwrap_or(&ByteSize(0))
                .0 as i64,
        );
        metrics.mem_usage.available.set(
            memory
                .platform_memory
                .meminfo
                .get("MemAvailable")
                .unwrap_or(&ByteSize(0))
                .0 as i64,
        );
    }

    if let Ok(ifaces) = sys.networks() {
        let mut general_stats = NetworkStats {
            rx_bytes: ByteSize(0),
            tx_bytes: ByteSize(0),
            rx_packets: 0,
            tx_packets: 0,
            rx_errors: 0,
            tx_errors: 0,
        };

        for (_, iface) in ifaces {
            if let Ok(iface_stats) = sys.network_stats(iface.name.as_str()) {
                general_stats.rx_bytes = general_stats.rx_bytes.add(iface_stats.rx_bytes);
                general_stats.tx_bytes = general_stats.tx_bytes.add(iface_stats.tx_bytes);
                general_stats.rx_packets += iface_stats.rx_packets;
                general_stats.tx_packets += iface_stats.tx_packets;
                general_stats.rx_errors += iface_stats.rx_errors;
                general_stats.tx_errors += iface_stats.tx_errors;
            }
        }

        let NetworkStats {
            rx_bytes,
            tx_bytes,
            rx_packets,
            tx_packets,
            rx_errors,
            tx_errors,
        } = general_stats;

        metrics.network_stats.rx_bytes.set(rx_bytes.0 as i64);
        metrics.network_stats.tx_bytes.set(tx_bytes.0 as i64);
        metrics.network_stats.rx_packets.set(rx_packets as i64);
        metrics.network_stats.tx_packets.set(tx_packets as i64);
        metrics.network_stats.rx_errors.set(rx_errors as i64);
        metrics.network_stats.tx_errors.set(tx_errors as i64);
    }

    if let Ok(SocketStats {
        tcp_sockets_in_use,
        tcp_sockets_orphaned,
        udp_sockets_in_use,
        tcp6_sockets_in_use,
        udp6_sockets_in_use,
    }) = sys.socket_stats()
    {
        metrics
            .socket_stats
            .tcp_sockets_in_use
            .set(tcp_sockets_in_use as i64);
        metrics
            .socket_stats
            .tcp6_sockets_in_use
            .set(tcp6_sockets_in_use as i64);
        metrics
            .socket_stats
            .tcp_sockets_orphaned
            .set(tcp_sockets_orphaned as i64);

        metrics
            .socket_stats
            .udp_sockets_in_use
            .set(udp_sockets_in_use as i64);
        metrics
            .socket_stats
            .udp6_sockets_in_use
            .set(udp6_sockets_in_use as i64);
    }
}
