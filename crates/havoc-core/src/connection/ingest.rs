//! The classifier: one parsed `irc_proto::Message` in, at most one
//! [`Ingest`](crate::storage::Ingest) out. Lives beside the actor (which owns
//! the single parse); the row shape lives in storage, because rows are
//! storage's business.
//!
//! Kinds ingested: Privmsg, Notice, Join, Part, Topic, Mode — everything
//! addressed to a named target. QUIT/NICK fan out to shared channels, which
//! needs membership state nothing has built yet (PLAN stage 2 note); numerics
//! stay un-ingested.

use std::collections::BTreeMap;

use havoc_ipc::{MessageKind, ServerTime};
use irc_proto::{Command, Prefix};

use crate::storage::Ingest;

pub(crate) fn classify(
    message: &irc_proto::Message,
    our_nick: &str,
    received_at_millis: i64,
) -> Option<Ingest> {
    let nick = match &message.prefix {
        Some(Prefix::Nickname(nick, _, _)) => Some(nick.clone()),
        _ => None,
    };

    let (kind, target, text) = match &message.command {
        Command::PRIVMSG(target, text) => (MessageKind::Privmsg, target, Some(text.clone())),
        Command::NOTICE(target, text) => (MessageKind::Notice, target, Some(text.clone())),
        Command::JOIN(channel, _, _) => (MessageKind::Join, channel, None),
        Command::PART(channel, reason) => (MessageKind::Part, channel, reason.clone()),
        Command::TOPIC(channel, topic) => (MessageKind::Topic, channel, topic.clone()),
        Command::ChannelMODE(channel, modes) => (
            MessageKind::Mode,
            channel,
            Some(
                modes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
        ),
        _ => return None,
    };

    // A PRIVMSG/NOTICE addressed to *us* belongs in the query buffer named
    // after the peer, not a buffer named after ourselves.
    let target = if target == our_nick {
        nick.clone()?
    } else {
        target.clone()
    };

    let mut msgid = None;
    let mut account = None;
    let mut server_time = None;
    let mut tags = BTreeMap::new();
    for tag in message.tags.iter().flatten() {
        let value = tag.1.clone().unwrap_or_default();
        match tag.0.as_str() {
            "msgid" => msgid = Some(value),
            "account" => account = Some(value),
            "time" => server_time = parse_server_time(&value),
            _ => {
                tags.insert(tag.0.clone(), value);
            }
        }
    }

    Some(Ingest {
        target,
        kind,
        nick,
        account,
        text,
        server_time: server_time.unwrap_or(ServerTime::from_unix_millis(received_at_millis)),
        msgid,
        tags,
    })
}

/// The one grammar IRCv3 `server-time` pins: `YYYY-MM-DDThh:mm:ss.sssZ`
/// (fractional part optional). Anything else → None, and the caller falls
/// back to local receipt time. A time dependency for one fixed format fails
/// the same allowlist bar the backoff jitter's rand did.
pub(crate) fn parse_server_time(value: &str) -> Option<ServerTime> {
    let value = value.strip_suffix('Z')?;
    let (date, time) = value.split_once('T')?;

    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: u32 = date_parts.next()?.parse().ok()?;
    let day: u32 = date_parts.next()?.parse().ok()?;
    if date_parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let (hms, millis) = match time.split_once('.') {
        Some((hms, frac)) => {
            let frac: String = frac.chars().take(3).collect();
            let scale = 10i64.pow(3 - u32::try_from(frac.len()).ok()?);
            (hms, frac.parse::<i64>().ok()? * scale)
        }
        None => (time, 0),
    };
    let mut time_parts = hms.split(':');
    // Unsigned parses reject leading '-'; bounds reject the rest.
    let hour = i64::from(time_parts.next()?.parse::<u32>().ok()?);
    let minute = i64::from(time_parts.next()?.parse::<u32>().ok()?);
    let second = i64::from(time_parts.next()?.parse::<u32>().ok()?);
    if time_parts.next().is_some() || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day > days_in_month[(month - 1) as usize] {
        return None;
    }

    let days = days_from_civil(year, month, day);
    Some(ServerTime::from_unix_millis(
        ((days * 24 + hour) * 60 + minute) * 60_000 + second * 1_000 + millis,
    ))
}

/// Howard Hinnant's `days_from_civil`: days since 1970-01-01, proleptic
/// Gregorian.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = i64::from((month + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_time_vectors() {
        // Epoch and a known ergo-shaped stamp.
        assert_eq!(
            parse_server_time("1970-01-01T00:00:00.000Z"),
            Some(ServerTime::from_unix_millis(0))
        );
        assert_eq!(
            parse_server_time("2026-08-10T08:35:51.107Z"),
            Some(ServerTime::from_unix_millis(1_786_350_951_107))
        );
        // No fractional part is legal.
        assert_eq!(
            parse_server_time("2000-02-29T12:00:00Z"),
            Some(ServerTime::from_unix_millis(951_825_600_000))
        );
        // Garbage falls back (caller substitutes receipt time).
        assert_eq!(parse_server_time("not-a-time"), None);
        assert_eq!(parse_server_time("2026-13-01T00:00:00Z"), None);
        assert_eq!(
            parse_server_time("2026-02-31T00:00:00Z"),
            None,
            "day/month aware"
        );
        assert_eq!(
            parse_server_time("2026-08-10T-1:00:00Z"),
            None,
            "negatives rejected"
        );
        assert_eq!(parse_server_time("2026-08-10 08:35:51Z"), None);
    }
}
