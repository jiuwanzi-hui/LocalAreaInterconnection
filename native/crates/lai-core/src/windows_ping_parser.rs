use crate::network_observation::TunnelObservation;

pub fn parse_windows_ping_observation(text: &str, expected_peer_count: u16) -> TunnelObservation {
    let received = parse_stat_value(text, "received").unwrap_or(0);
    let lost = parse_stat_value(text, "lost").unwrap_or_else(|| {
        let sent = parse_stat_value(text, "sent").unwrap_or(0);
        sent.saturating_sub(received)
    });
    let total = received + lost;
    let loss_percent = if total == 0 {
        100.0
    } else {
        (lost as f32 * 100.0) / total as f32
    };

    TunnelObservation {
        state: if received > 0 {
            "connected"
        } else {
            "disconnected"
        }
        .to_owned(),
        connected_peer_count: if received > 0 {
            expected_peer_count.max(1)
        } else {
            0
        },
        latency_ms: parse_average_latency_ms(text),
        packet_loss_percent: Some(loss_percent),
        path: Some("ping".to_owned()),
    }
}

fn parse_stat_value(text: &str, key: &str) -> Option<u16> {
    let key = key.to_ascii_lowercase();
    text.split(',').find_map(|part| {
        let part = part.trim();
        if !part.to_ascii_lowercase().starts_with(&key) {
            return None;
        }
        part.split('=')
            .nth(1)
            .and_then(|value| first_number(value).and_then(|number| number.parse::<u16>().ok()))
    })
}

fn parse_average_latency_ms(text: &str) -> Option<u32> {
    text.lines()
        .find(|line| line.to_ascii_lowercase().contains("average"))
        .and_then(|line| line.rsplit('=').next())
        .and_then(|value| first_number(value).and_then(|number| number.parse::<u32>().ok()))
}

fn first_number(value: &str) -> Option<String> {
    let mut number = String::new();
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            number.push(ch);
        } else if !number.is_empty() {
            break;
        }
    }
    if number.is_empty() {
        None
    } else {
        Some(number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_successful_windows_ping_output() {
        let output = r#"
Ping statistics for 127.0.0.1:
    Packets: Sent = 4, Received = 4, Lost = 0 (0% loss),
Approximate round trip times in milli-seconds:
    Minimum = 0ms, Maximum = 1ms, Average = 0ms
"#;

        let observation = parse_windows_ping_observation(output, 1);

        assert_eq!(observation.state, "connected");
        assert_eq!(observation.connected_peer_count, 1);
        assert_eq!(observation.packet_loss_percent, Some(0.0));
        assert_eq!(observation.latency_ms, Some(0));
    }

    #[test]
    fn parses_failed_windows_ping_output() {
        let output = r#"
Ping statistics for 10.77.12.3:
    Packets: Sent = 4, Received = 0, Lost = 4 (100% loss),
"#;

        let observation = parse_windows_ping_observation(output, 1);

        assert_eq!(observation.state, "disconnected");
        assert_eq!(observation.connected_peer_count, 0);
        assert_eq!(observation.packet_loss_percent, Some(100.0));
    }
}
