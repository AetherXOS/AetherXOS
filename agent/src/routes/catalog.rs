use rocket::serde::json::Value;
use rocket::State;
use crate::state::AppState;
use crate::resp::ok;

/// GET /catalog
#[rocket::get("/catalog")]
pub fn catalog(state: &State<AppState>) -> Value {
    let inner = state.read();
    let actions: Vec<Value> = inner.actions.iter().map(|a| {
        rocket::serde::json::json!({
            "id": a.id,
            "title": a.title,
            "desc": a.desc,
            "risk": a.risk,
            "category": a.category,
            "impact": a.impact,
        })
    }).collect();

    ok("catalog", rocket::serde::json::json!({ "actions": actions }))
}

/// GET /catalog/<id>
#[rocket::get("/catalog/<id>")]
pub fn catalog_action(state: &State<AppState>, id: String) -> Value {
    let inner = state.read();
    match inner.actions.iter().find(|a| a.id == id) {
        Some(a) => {
            let run_count = inner.jobs.values().filter(|j| j.action == a.id).count();
            ok(
                "catalog_action",
                rocket::serde::json::json!({
                    "action": {
                        "id": a.id,
                        "title": a.title,
                        "desc": a.desc,
                        "risk": a.risk,
                        "category": a.category,
                        "impact": a.impact,
                        "cmd": a.cmd,
                        "args": a.args,
                        "run_count": run_count,
                    }
                }),
            )
        }
        None => crate::resp::err("not_found", "Action not found.", "not_found"),
    }
}
