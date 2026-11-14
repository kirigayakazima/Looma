use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{LoomaError, LoomaResult};

pub mod experience_events {
    pub const STARTED: &str = "started";
    pub const PROGRESS_MILESTONE: &str = "progress_milestone";
    pub const COMPLETED: &str = "completed";
    pub const REPLAYED: &str = "replayed";
    pub const PUBLISHED: &str = "published";
    pub const CREATED: &str = "created";
    pub const REFLECTION: &str = "reflection";
}

/// Represents an individual structured footprint or life event associated with a subject (Entity/Work).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperienceEntry {
    pub id: String,                     // exp_<uuid> or UUID
    pub subject_type: String,           // MVP: "entity"
    pub subject_id: String,             // Entity ID (e.g. Work ID)
    pub event_type: String,             // completed/started/progress_milestone/replayed/...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,          // User-custom title or generated display title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,           // Optional brief reflection / note
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurred_value: Option<String>, // "2026", "2026-11", "2026-11-14", or RFC3339
    pub occurred_precision: String,     // unknown/year/month/day/instant
    #[serde(default = "default_context_json")]
    pub context_json: Value,            // platform, environment, playthrough, etc.
    #[serde(default = "default_visibility")]
    pub visibility: String,             // "normal" | "archived"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_context_json() -> Value {
    Value::Object(Default::default())
}

fn default_visibility() -> String {
    "normal".to_string()
}

pub fn validate_occurred_time(precision: &str, value: Option<&str>) -> LoomaResult<()> {
    match precision {
        "unknown" => {
            if let Some(v) = value {
                let trimmed = v.trim();
                if !trimmed.is_empty() {
                    return Err(LoomaError::Validation(
                        "occurred_value must be null when occurred_precision is 'unknown'".to_string(),
                    ));
                }
            }
            Ok(())
        }
        "year" => {
            let v = value
                .ok_or_else(|| {
                    LoomaError::Validation(
                        "occurred_value is required when occurred_precision is 'year'".to_string(),
                    )
                })?
                .trim();
            if v.len() != 4 || !v.chars().all(|c| c.is_ascii_digit()) {
                return Err(LoomaError::Validation(format!(
                    "Invalid year format '{v}', expected 4 digits (YYYY)"
                )));
            }
            let year: i32 = v
                .parse()
                .map_err(|_| LoomaError::Validation(format!("Invalid year number: {v}")))?;
            if !(1000..=9999).contains(&year) {
                return Err(LoomaError::Validation(format!("Year out of range: {year}")));
            }
            Ok(())
        }
        "month" => {
            let v = value
                .ok_or_else(|| {
                    LoomaError::Validation(
                        "occurred_value is required when occurred_precision is 'month'".to_string(),
                    )
                })?
                .trim();
            let parts: Vec<&str> = v.split('-').collect();
            if parts.len() != 2 || parts[0].len() != 4 || parts[1].len() != 2 {
                return Err(LoomaError::Validation(format!(
                    "Invalid month format '{v}', expected YYYY-MM"
                )));
            }
            let _year: i32 = parts[0]
                .parse()
                .map_err(|_| LoomaError::Validation(format!("Invalid year in '{v}'")))?;
            let month: u32 = parts[1]
                .parse()
                .map_err(|_| LoomaError::Validation(format!("Invalid month number in '{v}'")))?;
            if !(1..=12).contains(&month) {
                return Err(LoomaError::Validation(format!(
                    "Month out of range (01-12) in '{v}'"
                )));
            }
            Ok(())
        }
        "day" => {
            let v = value
                .ok_or_else(|| {
                    LoomaError::Validation(
                        "occurred_value is required when occurred_precision is 'day'".to_string(),
                    )
                })?
                .trim();
            chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").map_err(|e| {
                LoomaError::Validation(format!(
                    "Invalid day format or non-existent calendar date '{v}' (expected YYYY-MM-DD): {e}"
                ))
            })?;
            Ok(())
        }
        "instant" => {
            let v = value
                .ok_or_else(|| {
                    LoomaError::Validation(
                        "occurred_value is required when occurred_precision is 'instant'".to_string(),
                    )
                })?
                .trim();
            chrono::DateTime::parse_from_rfc3339(v).map_err(|e| {
                LoomaError::Validation(format!(
                    "Invalid instant format '{v}', expected RFC3339 with timezone: {e}"
                ))
            })?;
            Ok(())
        }
        other => Err(LoomaError::Validation(format!(
            "Invalid occurred_precision '{other}'. Must be one of: 'unknown', 'year', 'month', 'day', 'instant'"
        ))),
    }
}

impl ExperienceEntry {
    pub fn new(subject_id: impl Into<String>, event_type: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: format!("exp_{}", Uuid::new_v4().simple()),
            subject_type: "entity".to_string(),
            subject_id: subject_id.into(),
            event_type: event_type.into(),
            title: None,
            note: None,
            occurred_value: None,
            occurred_precision: "unknown".to_string(),
            context_json: Value::Object(Default::default()),
            visibility: "normal".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn with_occurred_unknown(mut self) -> Self {
        self.occurred_precision = "unknown".to_string();
        self.occurred_value = None;
        self
    }

    pub fn with_occurred_year(mut self, year: u32) -> Self {
        self.occurred_precision = "year".to_string();
        self.occurred_value = Some(format!("{:04}", year));
        self
    }

    pub fn with_occurred_month(mut self, year: u32, month: u32) -> Self {
        self.occurred_precision = "month".to_string();
        self.occurred_value = Some(format!("{:04}-{:02}", year, month));
        self
    }

    pub fn with_occurred_day(mut self, year: u32, month: u32, day: u32) -> Self {
        self.occurred_precision = "day".to_string();
        self.occurred_value = Some(format!("{:04}-{:02}-{:02}", year, month, day));
        self
    }

    pub fn with_occurred_instant(mut self, dt: DateTime<Utc>) -> Self {
        self.occurred_precision = "instant".to_string();
        self.occurred_value = Some(dt.to_rfc3339());
        self
    }

    pub fn with_context(mut self, context: Value) -> Self {
        self.context_json = context;
        self
    }

    pub fn validate(&self) -> LoomaResult<()> {
        if self.id.trim().is_empty() {
            return Err(LoomaError::Validation("Experience ID cannot be empty".to_string()));
        }
        if self.subject_type != "entity" {
            return Err(LoomaError::Validation(format!(
                "Invalid subject_type '{}', MVP only supports 'entity'",
                self.subject_type
            )));
        }
        if self.subject_id.trim().is_empty() {
            return Err(LoomaError::Validation(
                "Experience subject_id cannot be empty".to_string(),
            ));
        }
        if self.event_type.trim().is_empty() {
            return Err(LoomaError::Validation(
                "Experience event_type cannot be empty".to_string(),
            ));
        }
        if self.visibility != "normal" && self.visibility != "archived" {
            return Err(LoomaError::Validation(format!(
                "Invalid visibility '{}', expected 'normal' or 'archived'",
                self.visibility
            )));
        }
        validate_occurred_time(&self.occurred_precision, self.occurred_value.as_deref())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceLink {
    pub experience_id: String,
    pub linked_type: String,   // "asset" | "memory" | "external_reference"
    pub linked_id: String,
    #[serde(default = "default_relation_type")]
    pub relation_type: String, // "evidence" | "note" | "reference"
    #[serde(default)]
    pub position: i32,
}

fn default_relation_type() -> String {
    "evidence".to_string()
}

impl ExperienceLink {
    pub fn new(
        experience_id: impl Into<String>,
        linked_type: impl Into<String>,
        linked_id: impl Into<String>,
    ) -> Self {
        Self {
            experience_id: experience_id.into(),
            linked_type: linked_type.into(),
            linked_id: linked_id.into(),
            relation_type: "evidence".to_string(),
            position: 0,
        }
    }

    pub fn validate(&self) -> LoomaResult<()> {
        if self.experience_id.trim().is_empty() {
            return Err(LoomaError::Validation(
                "Link experience_id cannot be empty".to_string(),
            ));
        }
        if !["asset", "memory", "external_reference"].contains(&self.linked_type.as_str()) {
            return Err(LoomaError::Validation(format!(
                "Invalid linked_type '{}', expected 'asset', 'memory', or 'external_reference'",
                self.linked_type
            )));
        }
        if self.linked_id.trim().is_empty() {
            return Err(LoomaError::Validation(
                "Link linked_id cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExperienceFilter {
    pub subject_id: Option<String>,
    pub subject_type: Option<String>,
    pub event_type: Option<String>,
    pub visibility: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
