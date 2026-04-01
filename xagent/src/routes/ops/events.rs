use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use rocket::form::FromForm;
use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::{json, Value};
use rocket::State;

use crate::auth::RequireViewer;
use crate::dto::events::{EventActionCountDto, EventDto, EventsListDto, EventsStatsDto};
use crate::resp::{err_detail, ok};
use crate::state::AppState;

#[derive(Debug, Clone, FromForm)]
pub struct EventsQuery {
    pub kind: Option<String>,
    pub action: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub from_ts: Option<String>,
}

#[derive(Debug, Clone, FromForm)]
pub struct EventsStreamQuery {
    pub kind: Option<String>,
    pub action: Option<String>,
    pub from_id: Option<String>,
    pub from_ts: Option<String>,
    pub follow: Option<bool>,
    pub heartbeat_ms: Option<u64>,
    pub replay_limit: Option<usize>,
}

#[derive(Debug, Clone, FromForm)]
pub struct EventsStatsQuery {
    pub kind: Option<String>,
    pub since_minutes: Option<i64>,
    pub from_ts: Option<String>,
}

#[rocket::get("/events?<q..>")]
pub fn global_events(state: &State<AppState>, _role: RequireViewer, q: Option<EventsQuery>) -> Value {
    let query = q.unwrap_or(EventsQuery {
        kind: None,
        action: None,
        limit: None,
        cursor: None,
        from_ts: None,
    });

    let from_ts = match parse_optional_rfc3339_utc(query.from_ts.as_deref()) {
        Ok(v) => v,
        Err(raw) => {
            return err_detail(
                "invalid_from_ts",
                "Invalid from_ts. Expected RFC3339 timestamp.",
                "bad_request",
                json!({ "from_ts": raw }),
            );
        }
    };

    let kind_filter = query.kind.as_deref().map(|v| v.to_lowercase());
    let action_filter = query.action.as_deref().map(|v| v.to_lowercase());
    let limit = query.limit.unwrap_or(100).clamp(1, 500);

    let inner = state.read();
    let filtered_indices: Vec<usize> = inner
        .events
        .iter()
        .enumerate()
        .filter(|(_, event)| from_ts.map(|ts| event.ts_utc >= ts).unwrap_or(true))
        .filter(|(_, event)| event.matches_filters(kind_filter.as_deref(), action_filter.as_deref()))
        .map(|(idx, _)| idx)
        .collect();

    let end_exclusive = match query.cursor.as_deref() {
        Some(cursor) => {
            match filtered_indices
                .iter()
                .position(|idx| inner.events[*idx].id == cursor)
            {
                Some(pos) => pos,
                None => {
                    return err_detail(
                        "invalid_cursor",
                        "Cursor not found for current filter window.",
                        "bad_request",
                        json!({ "cursor": cursor }),
                    );
                }
            }
        }
        None => filtered_indices.len(),
    };

    let start = end_exclusive.saturating_sub(limit);
    let page_indices = &filtered_indices[start..end_exclusive];
    let events: Vec<EventDto> = page_indices
        .iter()
        .map(|idx| EventDto::from(&inner.events[*idx]))
        .collect();

    let returned = events.len();
    let next_cursor = if start > 0 {
        Some(inner.events[filtered_indices[start]].id.clone())
    } else {
        None
    };
    ok(
        "events",
        json!(EventsListDto {
            events,
            returned,
            next_cursor,
        }),
    )
}

#[rocket::get("/events/stats?<q..>")]
pub fn events_stats(state: &State<AppState>, _role: RequireViewer, q: Option<EventsStatsQuery>) -> Value {
    let query = q.unwrap_or(EventsStatsQuery {
        kind: None,
        since_minutes: None,
        from_ts: None,
    });
    let kind_filter = query.kind.as_deref().map(|v| v.to_lowercase());
    let since_minutes = query.since_minutes.map(|v| v.max(1));

    let from_ts = match parse_optional_rfc3339_utc(query.from_ts.as_deref()) {
        Ok(v) => v,
        Err(raw) => {
            return err_detail(
                "invalid_from_ts",
                "Invalid from_ts. Expected RFC3339 timestamp.",
                "bad_request",
                json!({ "from_ts": raw }),
            );
        }
    };

    let now = Utc::now();
    let mut kind_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut action_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;

    {
        let inner = state.read();
        for event in &inner.events {
            if let Some(minutes) = since_minutes {
                let threshold = now - chrono::Duration::minutes(minutes);
                if event.ts_utc < threshold {
                    continue;
                }
            }
            if let Some(ts) = from_ts {
                if event.ts_utc < ts {
                    continue;
                }
            }
            if !event.matches_filters(kind_filter.as_deref(), None) {
                continue;
            }
            total += 1;
            *kind_counts.entry(event.kind.clone()).or_insert(0) += 1;
            if let Some(action) = &event.action {
                *action_counts.entry(action.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut actions_ranked: Vec<(String, usize)> = action_counts.into_iter().collect();
    actions_ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let actions_top: Vec<EventActionCountDto> = actions_ranked
        .into_iter()
        .take(10)
        .map(|(action, count)| EventActionCountDto { action, count })
        .collect();

    ok(
        "events_stats",
        json!(EventsStatsDto {
            total,
            kind_filter,
            since_minutes,
            from_ts: from_ts.map(|v| v.to_rfc3339()),
            kinds: kind_counts,
            actions_top,
        }),
    )
}

#[rocket::get("/events/stream?<q..>")]
pub fn global_events_stream(
    state: &State<AppState>,
    _role: RequireViewer,
    q: Option<EventsStreamQuery>,
) -> EventStream![] {
    let state = state.inner().clone();
    let query = q.unwrap_or(EventsStreamQuery {
        kind: None,
        action: None,
        from_id: None,
        from_ts: None,
        follow: None,
        heartbeat_ms: None,
        replay_limit: None,
    });

    let parsed_from_ts = parse_optional_rfc3339_utc(query.from_ts.as_deref()).ok().flatten();
    let kind_filter = query.kind.map(|v| v.to_lowercase());
    let action_filter = query.action.map(|v| v.to_lowercase());
    let from_id = query.from_id;
    let follow = query.follow.unwrap_or(true);
    let heartbeat_ms = query.heartbeat_ms.unwrap_or(500).clamp(100, 5_000);
    let replay_limit = query.replay_limit.unwrap_or(500).clamp(1, 2_000);

    EventStream! {
        let mut last_index: usize = 0;
        loop {
            let batch = {
                let inner = state.read();
                let mut start_index = 0usize;
                if let Some(ref fid) = from_id {
                    if let Some(pos) = inner.events.iter().position(|event| event.id == *fid) {
                        start_index = pos.saturating_add(1);
                    }
                }
                start_index = start_index.max(last_index);

                let mut out: Vec<EventDto> = inner
                    .events
                    .iter()
                    .skip(start_index)
                    .filter(|event| parsed_from_ts.map(|ts| event.ts_utc >= ts).unwrap_or(true))
                    .filter(|event| event.matches_filters(kind_filter.as_deref(), action_filter.as_deref()))
                    .map(EventDto::from)
                    .collect();

                if out.len() > replay_limit {
                    let drop_count = out.len() - replay_limit;
                    out.drain(0..drop_count);
                }

                (inner.events.len(), out)
            };

            last_index = batch.0;
            let mut emitted_any = false;
            for item in batch.1 {
                emitted_any = true;
                yield Event::json(&item);
            }

            if !follow {
                if !emitted_any {
                    yield Event::comment("heartbeat");
                }
                break;
            }

            yield Event::comment("heartbeat");
            tokio::time::sleep(std::time::Duration::from_millis(heartbeat_ms)).await;
        }
    }
}

fn parse_optional_rfc3339_utc(raw: Option<&str>) -> Result<Option<DateTime<Utc>>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let parsed = DateTime::parse_from_rfc3339(raw).map_err(|_| raw.to_string())?;
    Ok(Some(parsed.with_timezone(&Utc)))
}
