//! The search query grammar, parsed core-side over the frozen wire shape
//! (`RequestBody::Search { query: String }` — prompt-2 carry-forward): a
//! quote-aware scanner lifts `from:` / `in:` / `after:` / `before:` filters
//! out of the plain string; everything else, quotes included, passes to FTS5
//! MATCH verbatim — which is where phrase search comes from for free.

/// Core-private, storage-executable search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSpec {
    pub match_query: String,
    pub nick: Option<String>,
    /// `in:` — a buffer **name**, and therefore deliberately **unscoped across
    /// networks**: two networks with a `#rust` each are one `in:#rust` result
    /// set. Left that way at prompt 10a rather than fixed there: the accretion
    /// *cause* died with the `debug-<host>` naming (stable config names mean one
    /// network row per network, not one per host string), so the union is now
    /// real but rare, and scoping is a grammar question (`in:net/#chan` versus a
    /// `network:` filter) owned by stage 2's `/search` — the only consumer that
    /// can render which network a hit came from. Pinned by
    /// `in_filter_unions_one_buffer_name_across_networks` in
    /// storage/tests.rs, so stage 2 changes a documented behaviour with a
    /// failing test rather than discovering an assumption.
    pub buffer: Option<String>,
    /// `server_time >=`, unix millis — a filter on display time, never an
    /// ordering (§6.1).
    pub after: Option<i64>,
    /// `server_time <`, unix millis.
    pub before: Option<i64>,
}

/// Errors are immediate `Response::Error`s — a filters-only query is a
/// backlog scan wearing search's clothes, and scans are prompt 9's.
pub fn parse(query: &str) -> Result<SearchSpec, String> {
    let mut spec = SearchSpec {
        match_query: String::new(),
        nick: None,
        buffer: None,
        after: None,
        before: None,
    };
    let mut terms: Vec<String> = Vec::new();

    for token in tokenize(query) {
        let filter = match &token {
            // A filter-shaped token inside quotes is text, not a filter.
            Token::Quoted(text) => {
                terms.push(format!("\"{text}\""));
                continue;
            }
            Token::Bare(word) => word.split_once(':'),
        };
        match filter {
            Some(("from", value)) if !value.is_empty() => {
                set_once(&mut spec.nick, "from", value.to_owned())?;
            }
            Some(("in", value)) if !value.is_empty() => {
                set_once(&mut spec.buffer, "in", value.to_owned())?;
            }
            Some(("after", value)) if !value.is_empty() => {
                let millis = date_bucket_start(value)
                    .ok_or_else(|| format!("after: wants YYYY[-MM[-DD]], got {value:?}"))?;
                set_once(&mut spec.after, "after", millis)?;
            }
            Some(("before", value)) if !value.is_empty() => {
                let millis = date_bucket_start(value)
                    .ok_or_else(|| format!("before: wants YYYY[-MM[-DD]], got {value:?}"))?;
                set_once(&mut spec.before, "before", millis)?;
            }
            _ => {
                let Token::Bare(word) = token else {
                    unreachable!()
                };
                terms.push(word);
            }
        }
    }

    if terms.is_empty() {
        return Err(
            "search needs text terms; a filters-only query is a backlog scan (prompt 9)".to_owned(),
        );
    }
    spec.match_query = terms.join(" ");
    Ok(spec)
}

fn set_once<T>(slot: &mut Option<T>, name: &str, value: T) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("duplicate {name}: filter"));
    }
    *slot = Some(value);
    Ok(())
}

enum Token {
    Bare(String),
    Quoted(String),
}

/// Whitespace-split, but `"..."` groups stay together (and keep their quotes
/// out of the filter scanner's reach). An unterminated quote passes through
/// as-is — FTS5 will reject it and that error comes back on the Response.
fn tokenize(query: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = query.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        if c == '"' {
            let mut text = String::new();
            let mut closed = false;
            for c in chars.by_ref() {
                if c == '"' {
                    closed = true;
                    break;
                }
                text.push(c);
            }
            if closed {
                tokens.push(Token::Quoted(text));
            } else {
                // Unterminated: hand it to FTS5 verbatim for the loud error.
                tokens.push(Token::Bare(format!("\"{text}")));
            }
        } else {
            let mut word = String::new();
            word.push(c);
            while let Some(&next) = chars.peek() {
                if next.is_whitespace() {
                    break;
                }
                word.push(next);
                chars.next();
            }
            tokens.push(Token::Bare(word));
        }
    }
    tokens
}

/// `YYYY`, `YYYY-MM`, or `YYYY-MM-DD` (UTC, §7.1's `after:2024-03` shape) →
/// the bucket's starting instant in unix millis. Reuses ingest's civil-date
/// arithmetic rather than growing a second calendar.
fn date_bucket_start(value: &str) -> Option<i64> {
    let mut parts = value.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = match parts.next() {
        None => 1,
        Some(m) => {
            let m = m.parse().ok()?;
            if !(1..=12).contains(&m) {
                return None;
            }
            m
        }
    };
    let day: u32 = match parts.next() {
        None => 1,
        Some(d) => {
            let d = d.parse().ok()?;
            if !(1..=31).contains(&d) {
                return None;
            }
            d
        }
    };
    if parts.next().is_some() {
        return None;
    }
    Some(crate::connection::ingest::days_from_civil(year, month, day) * 86_400_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_lift_out_and_terms_join() {
        let spec = parse(r#"from:bob in:#supernaut deployment failed"#).expect("parses");
        assert_eq!(spec.nick.as_deref(), Some("bob"));
        assert_eq!(spec.buffer.as_deref(), Some("#supernaut"));
        assert_eq!(spec.match_query, "deployment failed");
    }

    #[test]
    fn quoted_filter_shapes_are_text() {
        let spec = parse(r#""from: x" deployment"#).expect("parses");
        assert!(spec.nick.is_none());
        assert_eq!(spec.match_query, r#""from: x" deployment"#);
    }

    #[test]
    fn duplicate_filter_errors() {
        assert!(parse("from:a from:b hello").is_err());
    }

    #[test]
    fn all_three_date_forms() {
        let year = parse("after:2024 x").expect("year").after.expect("set");
        let month = parse("after:2024-03 x").expect("month").after.expect("set");
        let day = parse("after:2024-03-15 x")
            .expect("day")
            .after
            .expect("set");
        assert!(year < month && month < day);
        assert_eq!(year, 1_704_067_200_000, "2024-01-01T00:00:00Z");
        assert!(parse("after:notadate x").is_err());
    }

    #[test]
    fn filters_only_is_rejected() {
        assert!(parse("from:bob in:#x").is_err());
        assert!(parse("").is_err());
    }
}
