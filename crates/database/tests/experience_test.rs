use std::sync::Arc;
use chrono::Utc;
use looma_core::models::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use serde_json::json;

#[test]
fn test_t01_bloodborne_unknown_date_no_assets() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // User scenario: Bloodborne on PC emulator, completed, unknown date, no screenshots
    let work = core.create_work(
        "test-suite",
        "血源诅咒",
        WorkType::Game,
        RecordStatus::InProgress,
        Some("Bloodborne".to_string()),
        Some("FromSoftware developed action RPG".to_string()),
    ).expect("failed to create Bloodborne work");

    assert_eq!(work.title, "血源诅咒");

    // Complete work and record experience with unknown date
    let exp = ExperienceEntry::new(&work.id, experience_events::COMPLETED)
        .with_title("完成首次通关")
        .with_occurred_unknown()
        .with_context(json!({
            "platform": "PC",
            "environment": "emulator",
            "playthrough": 1
        }));

    let (updated_work, created_exp) = core.complete_work_and_record_experience(
        "test-suite",
        &work.id,
        exp,
        &[],
        None,
    ).expect("failed to complete work with experience");

    assert_eq!(updated_work.as_work_metadata().unwrap().status, RecordStatus::Completed);
    assert_eq!(created_exp.subject_id, work.id);
    assert_eq!(created_exp.event_type, "completed");
    assert_eq!(created_exp.occurred_precision, "unknown");
    assert!(created_exp.occurred_value.is_none());
    assert_eq!(created_exp.title.as_deref(), Some("完成首次通关"));

    // Verify WorkSummary contains the experience
    let summary = core.get_work_summary(&work.id).expect("get_work_summary failed").unwrap();
    assert_eq!(summary.experiences.len(), 1);
    assert_eq!(summary.experiences[0].id, created_exp.id);

    // Verify Timeline contains the experience
    let timeline = core.query_timeline(&TimelineFilter {
        item_types: Some(vec!["experience".to_string()]),
        ..Default::default()
    }).expect("query timeline failed");

    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].item_type, "experience");
    assert_eq!(timeline[0].title, "完成首次通关");
    assert_eq!(timeline[0].metadata["occurred_precision"], "unknown");
}

#[test]
fn test_t02_completed_work_without_fabricated_experience() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // Work already completed (e.g. from previous import) without footprints
    let work = core.create_work(
        "test-suite",
        "只狼：影逝二度",
        WorkType::Game,
        RecordStatus::Completed,
        Some("Sekiro: Shadows Die Twice".to_string()),
        None,
    ).expect("failed to create Sekiro work");

    // No experiences should be automatically fabricated
    let experiences = core.list_experiences_for_work(&work.id).expect("list experiences failed");
    assert!(experiences.is_empty(), "Experiences must not be fabricated on work creation or status");

    // User explicitly backfills footprint with year precision
    let exp = ExperienceEntry::new(&work.id, experience_events::COMPLETED)
        .with_title("通关龙之还乡结局")
        .with_occurred_year(2019)
        .with_note("打剑圣一心花了整整三天");

    let created = core.record_experience("test-suite", &exp, &[], None)
        .expect("failed to record backfill experience");

    assert_eq!(created.occurred_precision, "year");
    assert_eq!(created.occurred_value.as_deref(), Some("2019"));

    let updated_list = core.list_experiences_for_work(&work.id).expect("list experiences failed");
    assert_eq!(updated_list.len(), 1);
}

#[test]
fn test_t03_idempotent_request_id() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    let work = core.create_work(
        "test-suite",
        "艾尔登法环",
        WorkType::Game,
        RecordStatus::InProgress,
        Some("Elden Ring".to_string()),
        None,
    ).expect("failed to create Elden Ring");

    let request_id = ("client_mcp", "req_unique_998877");

    let exp1 = ExperienceEntry::new(&work.id, experience_events::COMPLETED)
        .with_title("通关群星结局")
        .with_occurred_month(2022, 3);

    let (work1, res1) = core.complete_work_and_record_experience(
        "client_mcp",
        &work.id,
        exp1.clone(),
        &[],
        Some(request_id),
    ).expect("first call failed");

    // Call second time with identical request_id (e.g. network retry)
    let (work2, res2) = core.complete_work_and_record_experience(
        "client_mcp",
        &work.id,
        exp1,
        &[],
        Some(request_id),
    ).expect("retry call failed");

    // Must return the exact same experience ID idempotently
    assert_eq!(res1.id, res2.id);
    assert_eq!(work1.id, work2.id);

    // Database must only have exactly 1 experience entry
    let all_exp = core.list_experiences_for_work(&work.id).expect("list failed");
    assert_eq!(all_exp.len(), 1);
}

#[test]
fn test_t04_replayed_different_request_id() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    let work = core.create_work(
        "test-suite",
        "黑神话：悟空",
        WorkType::Game,
        RecordStatus::InProgress,
        Some("Black Myth: Wukong".to_string()),
        None,
    ).expect("failed to create game");

    // Playthrough 1
    let exp1 = ExperienceEntry::new(&work.id, experience_events::COMPLETED)
        .with_title("一周目通关")
        .with_occurred_day(2024, 8, 25);

    let (_, res1) = core.complete_work_and_record_experience(
        "test-suite",
        &work.id,
        exp1,
        &[],
        Some(("test-suite", "run_1")),
    ).expect("run 1 failed");

    // Playthrough 2 (Replayed) with different request_id
    let exp2 = ExperienceEntry::new(&work.id, experience_events::REPLAYED)
        .with_title("二周目全成就全隐秘通关")
        .with_occurred_day(2024, 9, 10);

    let (_, res2) = core.complete_work_and_record_experience(
        "test-suite",
        &work.id,
        exp2,
        &[],
        Some(("test-suite", "run_2")),
    ).expect("run 2 failed");

    assert_ne!(res1.id, res2.id);
    let all = core.list_experiences_for_work(&work.id).expect("list failed");
    assert_eq!(all.len(), 2);
    assert!(all.iter().any(|e| e.event_type == "completed"));
    assert!(all.iter().any(|e| e.event_type == "replayed"));
}

#[test]
fn test_t05_transactional_rollback_on_failure() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    let work = core.create_work(
        "test-suite",
        "测试作品回滚",
        WorkType::Game,
        RecordStatus::InProgress,
        None,
        None,
    ).expect("failed to create work");

    // Create an experience with an invalid date that violates validation before transaction
    let invalid_exp = ExperienceEntry {
        id: "exp_test_rollback".to_string(),
        subject_type: "entity".to_string(),
        subject_id: work.id.clone(),
        event_type: "completed".to_string(),
        title: Some("通关".to_string()),
        note: None,
        occurred_value: Some("2026-02-30".to_string()), // Non-existent date!
        occurred_precision: "day".to_string(),
        context_json: json!({}),
        visibility: "normal".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let res = core.complete_work_and_record_experience(
        "test-suite",
        &work.id,
        invalid_exp,
        &[],
        None,
    );

    assert!(res.is_err(), "Must reject invalid calendar date 2026-02-30");

    // Invariant check: Work status must NOT have changed to completed!
    let current_work = core.get_entity(&work.id).expect("get entity failed").unwrap();
    let meta = current_work.as_work_metadata().unwrap();
    assert_eq!(meta.status, RecordStatus::InProgress, "Work status must remain InProgress on rollback");

    // Invariant check: No experience should exist in database
    let exps = core.list_experiences_for_work(&work.id).expect("list failed");
    assert!(exps.is_empty(), "No experience should be created on rollback");

    // Invariant check: No audit log for success should exist
    let audits = core.list_recent_audits(10).expect("list audits failed");
    assert!(!audits.iter().any(|a| a.operation == "work.complete_with_experience"), "No audit log for complete should be created");
}

#[test]
fn test_t06_occurred_date_validation() {
    // 1. Day precision with invalid date "2026-02-30" -> rejected
    assert!(validate_occurred_time("day", Some("2026-02-30")).is_err());
    // Valid day
    assert!(validate_occurred_time("day", Some("2026-02-28")).is_ok());

    // 2. Month precision with invalid month "2026-13" -> rejected
    assert!(validate_occurred_time("month", Some("2026-13")).is_err());
    // Valid month
    assert!(validate_occurred_time("month", Some("2026-11")).is_ok());

    // 3. Year precision with invalid year "999" -> rejected
    assert!(validate_occurred_time("year", Some("999")).is_err());
    // Valid year
    assert!(validate_occurred_time("year", Some("2026")).is_ok());

    // 4. Instant precision with invalid format -> rejected
    assert!(validate_occurred_time("instant", Some("2026-11-14")).is_err());
    // Valid RFC3339
    assert!(validate_occurred_time("instant", Some("2026-11-14T11:00:00Z")).is_ok());

    // 5. Unknown precision with non-null value -> rejected
    assert!(validate_occurred_time("unknown", Some("2026-11-14")).is_err());
    // Unknown precision with None -> ok
    assert!(validate_occurred_time("unknown", None).is_ok());
    assert!(validate_occurred_time("unknown", Some("")).is_ok());

    // 6. Invalid precision string -> rejected
    assert!(validate_occurred_time("century", Some("20")).is_err());
}

#[test]
fn test_experience_links_and_resilience() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    let work = core.create_work(
        "test-suite",
        "塞尔达传说：旷野之息",
        WorkType::Game,
        RecordStatus::InProgress,
        Some("The Legend of Zelda: Breath of the Wild".to_string()),
        None,
    ).expect("failed to create Zelda");

    // Create a memory note
    let memory = Memory {
        id: "mem_zelda_notes".to_string(),
        title: "海拉鲁通关手记".to_string(),
        content: "探索了全神庙和四大神兽".to_string(),
        category: Some("gaming".to_string()),
        metadata: json!({"tags": ["zelda"]}),
        recorded_at: Utc::now(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    core.create_memory("test-suite", &memory).expect("create memory failed");

    let exp = ExperienceEntry::new(&work.id, experience_events::COMPLETED)
        .with_title("全解谜通关")
        .with_occurred_day(2017, 3, 30);

    let link = ExperienceLink::new(&exp.id, "memory", &memory.id);

    let created = core.record_experience("test-suite", &exp, &[link.clone()], None)
        .expect("record experience with link failed");

    let links = core.list_experience_links(&created.id).expect("list links failed");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].linked_id, memory.id);
    assert_eq!(links[0].linked_type, "memory");

    // Remove link
    let removed = core.remove_experience_link("test-suite", &created.id, "memory", &memory.id)
        .expect("remove link failed");
    assert!(removed);

    // Invariant check: Experience itself must remain intact!
    let remaining_links = core.list_experience_links(&created.id).expect("list links failed");
    assert!(remaining_links.is_empty());
    let exp_still_exists = core.get_experience(&created.id).expect("get failed").unwrap();
    assert_eq!(exp_still_exists.title.as_deref(), Some("全解谜通关"));
}

#[test]
fn test_backup_and_restore_experiences() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    let work = core.create_work(
        "test-suite",
        "空洞骑士",
        WorkType::Game,
        RecordStatus::InProgress,
        Some("Hollow Knight".to_string()),
        None,
    ).expect("failed to create Hollow Knight");

    let exp = ExperienceEntry::new(&work.id, experience_events::COMPLETED)
        .with_title("击败辐光")
        .with_occurred_day(2018, 5, 20)
        .with_note("泪水之城音乐绝赞");

    core.complete_work_and_record_experience(
        "test-suite",
        &work.id,
        exp,
        &[],
        None,
    ).expect("complete work failed");

    // Generate manifest
    let manifest = core.generate_manifest().expect("generate manifest failed");
    assert_eq!(manifest.experiences_count, 1);

    // Export backup
    let temp_dir = std::env::temp_dir().join(format!("looma_exp_backup_{}", uuid::Uuid::new_v4()));
    let backup_res = core.export_backup("test-suite", &temp_dir).expect("export backup failed");
    assert_eq!(backup_res.manifest.experiences_count, 1);

    // Restore into a fresh new database
    let db2 = Arc::new(LoomaDb::in_memory().expect("in-memory db2 init failed"));
    let core2 = LoomaCore::new(db2.clone());

    let restore_res = core2.restore_backup("test-suite", std::path::Path::new(&backup_res.backup_path))
        .expect("restore backup failed");
    assert_eq!(restore_res.restored_experiences, 1);

    // Verify restored experience in db2
    let restored_work = core2.get_entity(&work.id).expect("get entity failed").unwrap();
    assert_eq!(restored_work.as_work_metadata().unwrap().status, RecordStatus::Completed);

    let restored_exps = core2.list_experiences_for_work(&work.id).expect("list failed");
    assert_eq!(restored_exps.len(), 1);
    assert_eq!(restored_exps[0].title.as_deref(), Some("击败辐光"));
    assert_eq!(restored_exps[0].occurred_value.as_deref(), Some("2018-05-20"));
    assert_eq!(restored_exps[0].occurred_precision, "day");
    assert_eq!(restored_exps[0].note.as_deref(), Some("泪水之城音乐绝赞"));

    // Clean up temp dir
    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_t15_mcp_readonly_permission_denied() {
    use looma_mcp::{McpPermissionLevel, McpServer};

    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    let work = core.create_work(
        "test-suite",
        "受限测试作品",
        WorkType::Game,
        RecordStatus::InProgress,
        None,
        None,
    ).expect("failed to create work");

    // By default or explicit ReadOnly
    let server_default = McpServer::new(core.clone());
    let server_readonly = McpServer::with_permission(core.clone(), McpPermissionLevel::ReadOnly);

    // Call record_experience on ReadOnly server
    let call_args = json!({
        "subject_id": work.id,
        "event_type": "completed",
        "occurred_precision": "unknown"
    });

    let res = server_readonly.call_tool("record_experience", call_args.clone());
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    assert!(err_msg.contains("Permission Denied"), "Must reject with permission denied: {err_msg}");

    // Also verify default McpServer is ReadOnly
    let res_def = server_default.call_tool("record_experience", call_args);
    assert!(res_def.is_err());
    assert!(res_def.unwrap_err().contains("Permission Denied"));

    // Side-effect verification: database has 0 experiences
    let exps = core.list_experiences_for_work(&work.id).expect("list failed");
    assert!(exps.is_empty(), "ReadOnly MCP call must produce zero side-effects");
}

#[test]
fn test_t16_mcp_content_write_record_experience() {
    use looma_mcp::{McpPermissionLevel, McpServer};

    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    let work = core.create_work(
        "test-suite",
        "星际拓荒",
        WorkType::Game,
        RecordStatus::InProgress,
        Some("Outer Wilds".to_string()),
        None,
    ).expect("failed to create Outer Wilds");

    let server_write = McpServer::with_permission(core.clone(), McpPermissionLevel::ContentWrite);

    // Call complete_work_with_experience tool
    let complete_args = json!({
        "work_id": work.id,
        "event_type": "completed",
        "occurred_precision": "day",
        "occurred_value": "2020-06-18",
        "title": "到达宇宙眼",
        "note": "宇宙终结前的壮丽旅程",
        "context": {
            "platform": "PC",
            "environment": "Steam"
        },
        "request_id": "mcp_outerwilds_001"
    });

    let res = server_write.call_tool("complete_work_with_experience", complete_args);
    assert!(res.is_ok(), "ContentWrite call should succeed: {:?}", res);

    // Check that GUI / CLI can query it
    let summary = core.get_work_summary(&work.id).expect("get summary failed").unwrap();
    assert_eq!(summary.work_metadata.unwrap().status, RecordStatus::Completed);
    assert_eq!(summary.experiences.len(), 1);
    assert_eq!(summary.experiences[0].title.as_deref(), Some("到达宇宙眼"));
    assert_eq!(summary.experiences[0].occurred_value.as_deref(), Some("2020-06-18"));

    // Check audit trail
    let audits = core.list_recent_audits(10).expect("list audits failed");
    assert!(audits.iter().any(|a| a.actor == "mcp" && a.operation == "work.complete_with_experience"));

    // Read resource looma://work/{id}/experiences via handle_method
    let resp = server_write.handle_method(
        "resources/read",
        json!({ "uri": format!("looma://work/{}/experiences", work.id) }),
        Some(json!(1)),
    ).expect("read resource failed");
    assert!(resp.to_string().contains("到达宇宙眼"));
}
