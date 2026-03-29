use super::super::*;

pub(crate) fn sys_get_network_stats(_ptr: usize, _len: usize) -> usize {
    SYSCALL_NETWORK_STATS_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) = require_control_plane_access(crate::modules::security::RESOURCE_NETWORK_STATS)
    {
        return err;
    }

    #[cfg(feature = "networking")]
    {
        let net = crate::modules::network::bridge::stats();
        write_user_words(
            _ptr,
            _len,
            [
                net.smoltcp_bridge_inits as usize,
                net.smoltcp_polls as usize,
                net.smoltcp_rx_frames as usize,
                net.smoltcp_tx_frames as usize,
                net.smoltcp_runtime_ready as usize,
                net.smoltcp_runtime_poll_enabled as usize,
                net.smoltcp_init_errors as usize,
                net.smoltcp_poll_errors as usize,
                net.smoltcp_poll_skips as usize,
                net.smoltcp_runtime_control_updates as usize,
            ],
        )
    }

    #[cfg(not(feature = "networking"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_set_network_polling(_enable: usize) -> usize {
    SYSCALL_NETWORK_POLL_CONTROL_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) =
        require_control_plane_access(crate::modules::security::RESOURCE_NETWORK_CONTROL)
    {
        return err;
    }

    #[cfg(feature = "networking")]
    {
        let Some(mode) = BinarySwitch::from_usize(_enable) else {
            return invalid_arg();
        };
        crate::modules::network::bridge::set_runtime_polling_enabled(mode.is_enabled());
        0
    }

    #[cfg(not(feature = "networking"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_network_reset_stats() -> usize {
    SYSCALL_NETWORK_RESET_STATS_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) =
        require_control_plane_access(crate::modules::security::RESOURCE_NETWORK_CONTROL)
    {
        return err;
    }

    #[cfg(feature = "networking")]
    {
        crate::modules::network::reset_runtime_stats();
        0
    }

    #[cfg(not(feature = "networking"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_network_force_poll() -> usize {
    SYSCALL_NETWORK_FORCE_POLL_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) =
        require_control_plane_access(crate::modules::security::RESOURCE_NETWORK_CONTROL)
    {
        return err;
    }

    #[cfg(feature = "networking")]
    {
        crate::modules::network::force_poll_once() as usize
    }

    #[cfg(not(feature = "networking"))]
    {
        invalid_arg()
    }
}

pub(crate) fn sys_network_reinitialize() -> usize {
    SYSCALL_NETWORK_REINIT_CALLS.fetch_add(1, Ordering::Relaxed);
    if let Err(err) =
        require_control_plane_access(crate::modules::security::RESOURCE_NETWORK_CONTROL)
    {
        return err;
    }

    #[cfg(feature = "networking")]
    {
        struct LoopbackNic;
        impl crate::modules::network::NetworkInterface for LoopbackNic {
            fn send(
                &mut self,
                _packet: crate::modules::network::Packet,
            ) -> Result<(), &'static str> {
                Ok(())
            }
            fn receive(&mut self) -> Result<Option<crate::modules::network::Packet>, &'static str> {
                Ok(None)
            }
            fn mac(&self) -> crate::modules::network::MacAddress {
                crate::modules::network::MacAddress::Ethernet([0x02, 0x00, 0x00, 0x00, 0x00, 0x01])
            }
        }
        let nic = LoopbackNic;
        match crate::modules::network::reinitialize_smoltcp_runtime(&nic) {
            Ok(()) => 0,
            Err(_) => invalid_arg(),
        }
    }

    #[cfg(not(feature = "networking"))]
    {
        invalid_arg()
    }
}
