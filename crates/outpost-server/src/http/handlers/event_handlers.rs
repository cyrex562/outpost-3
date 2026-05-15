//! Event handlers — event log page, ticker partial, and choice resolution.

use actix_web::{web, HttpResponse, Result};
use outpost_core::content::{EventCategory, EventSeverity};
use serde::Deserialize;
use tera::Context;

use crate::services::SimulationService;

// ─────────────────────────────────────────────────────────────────────────────
// Query / form types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct EventLogQuery {
    pub category: Option<String>,
    pub severity: Option<String>,
    pub page: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveChoiceForm {
    pub choice_id: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /events
// ─────────────────────────────────────────────────────────────────────────────

/// Full event log page with filtering and pagination.
pub async fn event_log_page(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
    query: web::Query<EventLogQuery>,
) -> Result<HttpResponse> {
    let game_state = simulation.get_state().await;
    let log = &game_state.event_log;

    // Determine active filters
    let cat_filter: Option<EventCategory> = query.category.as_deref().and_then(parse_category);
    let sev_filter: Option<EventSeverity> = query.severity.as_deref().and_then(parse_severity);

    // Collect and filter entries (newest first for display)
    let mut entries: Vec<serde_json::Value> = log
        .all()
        .iter()
        .rev()
        .filter(|e| {
            cat_filter.as_ref().map_or(true, |c| &e.category == c)
                && sev_filter.as_ref().map_or(true, |s| &e.severity == s)
        })
        .map(|e| {
            serde_json::json!({
                "index":      e.index,
                "tick":       e.tick,
                "site_id":    e.site_id.to_string(),
                "event_id":   e.event_id,
                "title":      e.title,
                "category":   category_label(&e.category),
                "category_key": category_key(&e.category),
                "severity":   severity_label(&e.severity),
                "severity_key": severity_key(&e.severity),
                "required_choice": e.required_choice,
                "resolved":   e.resolved_choice_id.is_some(),
                "choice_id":  e.resolved_choice_id.clone().unwrap_or_default(),
            })
        })
        .collect();

    // Pagination (50 per page)
    const PAGE_SIZE: usize = 50;
    let page = query.page.unwrap_or(1).max(1);
    let total = entries.len();
    let total_pages = (total + PAGE_SIZE - 1) / PAGE_SIZE;
    let start = (page - 1) * PAGE_SIZE;
    let page_entries: Vec<_> = entries.drain(start..).take(PAGE_SIZE).collect();

    // Badge counts for top bar
    let critical_count = log.count_by_severity(&EventSeverity::Critical);
    let warning_count = log.count_by_severity(&EventSeverity::Warning);
    let unresolved = log.unresolved_choices();

    let mut ctx = Context::new();
    ctx.insert("entries", &page_entries);
    ctx.insert("total", &total);
    ctx.insert("page", &page);
    ctx.insert("total_pages", &total_pages);
    ctx.insert("page_size", &PAGE_SIZE);
    ctx.insert("active_category", &query.category);
    ctx.insert("active_severity", &query.severity);
    ctx.insert("critical_count", &critical_count);
    ctx.insert("warning_count", &warning_count);
    ctx.insert("unresolved_choices", &unresolved);

    let html = tmpl
        .render("events.html", &ctx)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/events/ticker
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the ticker partial — polled by HTMX every few seconds.
pub async fn event_ticker(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let game_state = simulation.get_state().await;
    let log = &game_state.event_log;

    let recent: Vec<serde_json::Value> = log
        .recent(8)
        .iter()
        .map(|e| {
            serde_json::json!({
                "tick":       e.tick,
                "title":      e.title,
                "category":   category_label(&e.category),
                "category_key": category_key(&e.category),
                "severity":   severity_label(&e.severity),
                "severity_key": severity_key(&e.severity),
                "icon":       severity_icon(&e.severity),
                "required_choice": e.required_choice,
                "resolved":   e.resolved_choice_id.is_some(),
            })
        })
        .collect();

    let critical_count = log.count_by_severity(&EventSeverity::Critical);
    let warning_count = log.count_by_severity(&EventSeverity::Warning);
    let unresolved = log.unresolved_choices();

    let mut ctx = Context::new();
    ctx.insert("recent_events", &recent);
    ctx.insert("critical_count", &critical_count);
    ctx.insert("warning_count", &warning_count);
    ctx.insert("unresolved_choices", &unresolved);

    let html = tmpl
        .render("components/_event_ticker.html", &ctx)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/events/badges
// ─────────────────────────────────────────────────────────────────────────────

/// Returns alert badge counts for the top bar — polled by HTMX.
pub async fn event_badges(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
) -> Result<HttpResponse> {
    let game_state = simulation.get_state().await;
    let log = &game_state.event_log;

    let critical_count = log.count_by_severity(&EventSeverity::Critical);
    let warning_count = log.count_by_severity(&EventSeverity::Warning);
    let unresolved = log.unresolved_choices();

    let mut ctx = Context::new();
    ctx.insert("critical_count", &critical_count);
    ctx.insert("warning_count", &warning_count);
    ctx.insert("unresolved_choices", &unresolved);

    let html = tmpl
        .render("components/_event_badges.html", &ctx)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /site/{id}/events/{event_id}/choice
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the event choice modal for a pending event.
pub async fn event_choice_modal(
    tmpl: web::Data<tera::Tera>,
    simulation: web::Data<SimulationService>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (site_id_str, event_id) = path.into_inner();
    let game_state = simulation.get_state().await;

    // Find pending choice
    let pending = game_state.event_engine.pending_choices.iter()
        .find(|p| p.site_id == site_id_str && p.event_id == event_id);

    let Some(pending) = pending else {
        return Ok(HttpResponse::NotFound().body("Event choice not found"));
    };

    // Get event definition for choices
    let choices: Vec<serde_json::Value> = game_state
        .get_event_def(&event_id)
        .map(|def| {
            def.choices.iter().map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "text": c.text,
                    "description": c.description,
                    "outcomes": c.outcomes.len(),
                })
            }).collect()
        })
        .unwrap_or_default();

    let mut ctx = Context::new();
    ctx.insert("site_id", &site_id_str);
    ctx.insert("event_id", &event_id);
    ctx.insert("title", &pending.title);
    ctx.insert("description", &pending.description);
    ctx.insert("fired_at_tick", &pending.fired_at_tick);
    ctx.insert("choices", &choices);

    let html = tmpl
        .render("components/_event_choice_modal.html", &ctx)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /site/{id}/events/{event_id}/resolve
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve a pending event choice.
pub async fn resolve_event_choice(
    simulation: web::Data<SimulationService>,
    path: web::Path<(String, String)>,
    form: web::Form<ResolveChoiceForm>,
) -> Result<HttpResponse> {
    let (site_id_str, event_id) = path.into_inner();

    use outpost_core::domain::SiteId;
    use outpost_core::simulation::event_engine::resolve_choice;
    use uuid::Uuid;

    let site_uuid = Uuid::parse_str(&site_id_str)
        .map_err(|_| actix_web::error::ErrorBadRequest("Invalid site ID"))?;
    let site_id = SiteId(site_uuid);

    let result = simulation
        .with_state_mut(|state| {
            resolve_choice(state, site_id, &event_id, &form.choice_id)
        })
        .await;

    match result {
        Some(_) => Ok(HttpResponse::Ok()
            .append_header(("HX-Trigger", "eventResolved"))
            .body("")),
        None => Ok(HttpResponse::NotFound().body("Choice not found")),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn parse_category(s: &str) -> Option<EventCategory> {
    match s {
        "disaster"  => Some(EventCategory::Disaster),
        "discovery" => Some(EventCategory::Discovery),
        "social"    => Some(EventCategory::Social),
        "technical" => Some(EventCategory::Technical),
        "economic"  => Some(EventCategory::Economic),
        "general"   => Some(EventCategory::General),
        _ => None,
    }
}

fn parse_severity(s: &str) -> Option<EventSeverity> {
    match s {
        "info"     => Some(EventSeverity::Info),
        "warning"  => Some(EventSeverity::Warning),
        "critical" => Some(EventSeverity::Critical),
        _ => None,
    }
}

fn category_label(c: &EventCategory) -> &'static str {
    match c {
        EventCategory::Disaster  => "Disaster",
        EventCategory::Discovery => "Discovery",
        EventCategory::Social    => "Social",
        EventCategory::Technical => "Technical",
        EventCategory::Economic  => "Economic",
        EventCategory::General   => "General",
    }
}

fn category_key(c: &EventCategory) -> &'static str {
    match c {
        EventCategory::Disaster  => "disaster",
        EventCategory::Discovery => "discovery",
        EventCategory::Social    => "social",
        EventCategory::Technical => "technical",
        EventCategory::Economic  => "economic",
        EventCategory::General   => "general",
    }
}

fn severity_label(s: &EventSeverity) -> &'static str {
    match s {
        EventSeverity::Info     => "Info",
        EventSeverity::Warning  => "Warning",
        EventSeverity::Critical => "Critical",
    }
}

fn severity_key(s: &EventSeverity) -> &'static str {
    match s {
        EventSeverity::Info     => "info",
        EventSeverity::Warning  => "warning",
        EventSeverity::Critical => "critical",
    }
}

fn severity_icon(s: &EventSeverity) -> &'static str {
    match s {
        EventSeverity::Info     => "ℹ️",
        EventSeverity::Warning  => "⚠️",
        EventSeverity::Critical => "🚨",
    }
}
