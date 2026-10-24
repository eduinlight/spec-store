use std::net::{IpAddr, SocketAddr};

fn parse_address(entry: &str) -> Option<IpAddr> {
  let entry = entry.trim();
  entry
    .parse::<IpAddr>()
    .or_else(|_| entry.parse::<SocketAddr>().map(|socket| socket.ip()))
    .ok()
    .map(|ip| ip.to_canonical())
}
pub fn client_origin(
  peer: Option<IpAddr>,
  forwarded_for: Option<&str>,
  trusted_hops: usize,
) -> Option<String> {
  let forwarded = if trusted_hops == 0 {
    None
  } else {
    forwarded_for.and_then(|header| {
      let entries: Vec<&str> = header.split(',').collect();
      let index = entries.len().saturating_sub(trusted_hops);
      parse_address(entries[index])
    })
  };
  forwarded
    .or_else(|| peer.map(|ip| ip.to_canonical()))
    .map(|ip| ip.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn peer() -> Option<IpAddr> {
    Some("10.0.0.9".parse().unwrap())
  }

  #[test]
  fn without_trusted_hops_the_socket_peer_wins_over_the_header() {
    assert_eq!(
      client_origin(peer(), Some("203.0.113.7"), 0).as_deref(),
      Some("10.0.0.9")
    );
  }

  #[test]
  fn the_entry_that_many_hops_from_the_right_is_used() {
    let header = "198.51.100.1, 203.0.113.7, 10.1.1.1";
    assert_eq!(
      client_origin(peer(), Some(header), 1).as_deref(),
      Some("10.1.1.1")
    );
    assert_eq!(
      client_origin(peer(), Some(header), 2).as_deref(),
      Some("203.0.113.7")
    );
    assert_eq!(
      client_origin(peer(), Some(header), 3).as_deref(),
      Some("198.51.100.1")
    );
  }

  #[test]
  fn more_hops_than_entries_clamp_to_the_first_entry() {
    assert_eq!(
      client_origin(peer(), Some("203.0.113.7, 10.1.1.1"), 5).as_deref(),
      Some("203.0.113.7")
    );
  }

  #[test]
  fn a_missing_or_unparsable_header_falls_back_to_the_peer() {
    assert_eq!(client_origin(peer(), None, 2).as_deref(), Some("10.0.0.9"));
    assert_eq!(
      client_origin(peer(), Some("not-an-address"), 1).as_deref(),
      Some("10.0.0.9")
    );
  }

  #[test]
  fn ports_and_mapped_addresses_are_normalized() {
    assert_eq!(
      client_origin(None, Some("203.0.113.7:5000"), 1).as_deref(),
      Some("203.0.113.7")
    );
    let mapped: IpAddr = "::ffff:192.0.2.1".parse().unwrap();
    assert_eq!(
      client_origin(Some(mapped), None, 0).as_deref(),
      Some("192.0.2.1")
    );
  }

  #[test]
  fn no_peer_and_no_header_is_unknown() {
    assert_eq!(client_origin(None, None, 0), None);
  }
}
