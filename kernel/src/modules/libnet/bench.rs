#[derive(Debug, Clone, Copy, Default)]
pub struct LibNetMicroBenchReport {
    pub iterations: usize,
    pub bridge_frames_ingested: usize,
    pub l34_polls: usize,
    pub transport_roundtrips: usize,
}

pub fn bench_bridge_surface(iterations: usize, l2_budget: usize) -> LibNetMicroBenchReport {
    let loops = iterations.max(1);
    let budget = l2_budget.max(1);
    let mut report = LibNetMicroBenchReport {
        iterations: loops,
        ..LibNetMicroBenchReport::default()
    };
    for _ in 0..loops {
        let cycle = crate::modules::libnet::pump_once_with_budget_report(Some(budget));
        report.bridge_frames_ingested = report
            .bridge_frames_ingested
            .saturating_add(cycle.l2_frames_ingested);
        if cycle.l34_polled {
            report.l34_polls = report.l34_polls.saturating_add(1);
        }
    }
    report
}

pub fn bench_fast_path_surface(iterations: usize, l2_budget: usize) -> LibNetMicroBenchReport {
    let loops = iterations.max(1);
    let budget = l2_budget.max(1);
    let mut report = LibNetMicroBenchReport {
        iterations: loops,
        ..LibNetMicroBenchReport::default()
    };
    for _ in 0..loops {
        let cycle =
            crate::modules::libnet::run_fast_path_once(crate::modules::libnet::FastPathConfig {
                poll_strategy: crate::modules::libnet::PollStrategy::Fixed(
                    crate::modules::libnet::PollProfile::Balanced,
                ),
                run_pump: true,
                collect_transport_snapshot: false,
                l2_pump_budget: budget,
            });
        report.bridge_frames_ingested = report
            .bridge_frames_ingested
            .saturating_add(cycle.pump.l2_frames_ingested);
        if cycle.pump.l34_polled {
            report.l34_polls = report.l34_polls.saturating_add(1);
        }
    }
    report
}

#[cfg(feature = "network_transport")]
pub fn bench_transport_surface(iterations: usize, payload_len: usize) -> LibNetMicroBenchReport {
    use crate::modules::libnet::DatagramSocket;

    let loops = iterations.max(1);
    let size = payload_len.max(1);
    let source = match crate::modules::libnet::udp_bind(36400) {
        Ok(s) => s,
        Err(_) => return LibNetMicroBenchReport::default(),
    };
    let sink = match crate::modules::libnet::udp_bind(36401) {
        Ok(s) => s,
        Err(_) => return LibNetMicroBenchReport::default(),
    };
    let payload = alloc::vec![0xABu8; size];
    let mut report = LibNetMicroBenchReport {
        iterations: loops,
        ..LibNetMicroBenchReport::default()
    };

    for _ in 0..loops {
        if source.send_to(sink.local_port(), &payload).is_ok() && sink.recv().is_some() {
            report.transport_roundtrips = report.transport_roundtrips.saturating_add(1);
        }
        let cycle = crate::modules::libnet::pump_once_with_report();
        report.bridge_frames_ingested = report
            .bridge_frames_ingested
            .saturating_add(cycle.l2_frames_ingested);
        if cycle.l34_polled {
            report.l34_polls = report.l34_polls.saturating_add(1);
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn libnet_bench_bridge_surface_executes() {
        let report = bench_bridge_surface(16, 8);
        assert_eq!(report.iterations, 16);
    }

    #[test_case]
    fn libnet_bench_fast_path_surface_executes() {
        let report = bench_fast_path_surface(16, 8);
        assert_eq!(report.iterations, 16);
    }

    #[cfg(feature = "network_transport")]
    #[test_case]
    fn libnet_bench_transport_surface_executes() {
        let report = bench_transport_surface(8, 64);
        assert_eq!(report.iterations, 8);
        assert!(report.transport_roundtrips <= report.iterations);
    }
}
