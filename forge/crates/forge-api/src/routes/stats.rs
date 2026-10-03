use axum::extract::Query;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Query params / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    /// Directory containing .traj files (default: "trajectories")
    #[serde(default = "default_dir")]
    pub dir: String,
}

fn default_dir() -> String {
    "trajectories".to_string()
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub directory: String,
    pub total: usize,
    pub submitted: usize,
    pub forfeited: usize,
    pub errors: usize,
    pub step_limit_reached: usize,
    pub other: usize,
}

// Minimal subset of TrajFile.info needed to read exit_status without
// importing all of forge-types. Using serde_json::Value for flexibility.
#[derive(Debug, serde::Deserialize)]
struct TrajFilePartial {
    info: TrajInfoPartial,
}

#[derive(Debug, serde::Deserialize)]
struct TrajInfoPartial {
    #[serde(alias = "exitStatus")]
    exit_status: Option<String>,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// GET /api/stats?dir=trajectories
///
/// Scans the given directory for .traj files and returns aggregate counts.
pub async fn handler(
    Query(params): Query<StatsQuery>,
) -> Result<Json<StatsResponse>, (StatusCode, String)> {
    let dir = std::path::PathBuf::from(&params.dir);

    if !dir.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Directory {:?} does not exist", dir),
        ));
    }

    let mut total = 0usize;
    let mut submitted = 0usize;
    let mut forfeited = 0usize;
    let mut errors = 0usize;
    let mut step_limit = 0usize;

    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("traj") {
            continue;
        }
        total += 1;

        if let Ok(content) = tokio::fs::read_to_string(&path).await {
            if let Ok(traj) = serde_json::from_str::<TrajFilePartial>(&content) {
                match traj.info.exit_status.as_deref() {
                    Some("submitted") => submitted += 1,
                    Some("forfeited") => forfeited += 1,
                    Some("error") => errors += 1,
                    Some("step_limit_reached") => step_limit += 1,
                    _ => {}
                }
            }
        }
    }

    let other = total.saturating_sub(submitted + forfeited + errors + step_limit);

    Ok(Json(StatsResponse {
        directory: params.dir,
        total,
        submitted,
        forfeited,
        errors,
        step_limit_reached: step_limit,
        other,
    }))
}
