use super::*;

#[cfg(feature = "network_transport")]
pub fn poll(fds: &mut [PosixPollFd], retries: usize) -> Result<usize, &'static str> {
    ensure_posix_available()?;

    for _ in 0..=retries {
        let mut ready = 0usize;
        for pollfd in fds.iter_mut() {
            pollfd.revents = PosixPollEvents::empty();
            let mut revents = 0;

            let res = with_socket(pollfd.fd, |s| {
                let policy = s.poll_events();
                revents = policy.bits() as u16;
                Ok(())
            });

            if res.is_err() {
                pollfd.revents = PosixPollEvents::from_bits_truncate(
                    crate::modules::posix_consts::net::POLLNVAL as u16,
                );
                continue;
            }

            pollfd.revents = PosixPollEvents::from_bits_truncate(revents);
            if !pollfd.revents.is_empty() {
                ready += 1;
            }
        }

        if ready > 0 {
            return Ok(ready);
        }
        poll_transport_hint();
    }

    Ok(0)
}

#[cfg(feature = "network_transport")]
pub fn select(
    read_fds: &[u32],
    write_fds: &[u32],
    except_fds: &[u32],
    retries: usize,
) -> Result<PosixSelectResult, &'static str> {
    ensure_posix_available()?;

    let mut read_pollfds: Vec<PosixPollFd> = read_fds
        .iter()
        .copied()
        .map(|fd| PosixPollFd::new(fd, PosixPollEvents::IN))
        .collect();

    let mut write_pollfds: Vec<PosixPollFd> = write_fds
        .iter()
        .copied()
        .map(|fd| PosixPollFd::new(fd, PosixPollEvents::OUT))
        .collect();

    let mut except_pollfds: Vec<PosixPollFd> = except_fds
        .iter()
        .copied()
        .map(|fd| PosixPollFd::new(fd, PosixPollEvents::ERR | PosixPollEvents::HUP))
        .collect();

    let _ = poll(&mut read_pollfds, retries)?;
    let _ = poll(&mut write_pollfds, 0)?;
    let _ = poll(&mut except_pollfds, 0)?;

    Ok(PosixSelectResult {
        readable: read_pollfds
            .into_iter()
            .filter(|fd| fd.revents.contains(PosixPollEvents::IN))
            .map(|fd| fd.fd)
            .collect(),
        writable: write_pollfds
            .into_iter()
            .filter(|fd| fd.revents.contains(PosixPollEvents::OUT))
            .map(|fd| fd.fd)
            .collect(),
        exceptional: except_pollfds
            .into_iter()
            .filter(|fd| {
                fd.revents
                    .intersects(PosixPollEvents::ERR | PosixPollEvents::HUP)
            })
            .map(|fd| fd.fd)
            .collect(),
    })
}