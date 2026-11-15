use std::sync::Arc;
use looma_core::models::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use serde_json::json;

#[test]
fn test_entity_update_preserves_work_metadata() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // 1. Create a Work via core.create_work
    let work = core
        .create_work(
            "test",
            "艾尔登法环 (Elden Ring)",
            WorkType::Game,
            RecordStatus::Planned,
            Some("ELDEN RING".to_string()),
            Some("FromSoftware 动作角色扮演游戏".to_string()),
        )
        .expect("failed to create work");

    // 2. Set aliases, progress
    core.update_work_aliases("test", &work.id, vec!["老头环".to_string(), "法环".to_string()])
        .expect("update aliases");
    core.update_work_progress(
        "test",
        &work.id,
        85.0,
        Some(ProgressPositionType::Percentage),
        None,
        None,
    )
    .expect("update progress");

    // Verify initial work metadata is present
    let initial = core
        .get_entity(&work.id)
        .expect("get")
        .expect("found");
    let initial_meta = initial.as_work_metadata().expect("valid initial meta");
    assert_eq!(initial_meta.aliases, vec!["老头环".to_string(), "法环".to_string()]);
    assert_eq!(initial_meta.progress.as_ref().map(|p| p.position), Some(85.0));

    // 3. Perform a generic entity update that only updates title, description, and custom properties
    let mut updated_entity = initial.clone();
    updated_entity.title = "艾尔登法环：黄金树幽影".to_string();
    updated_entity.description = Some("含 DLC 内容".to_string());
    // Simulate generic editor which only sets custom properties and omits "work"
    updated_entity.properties = json!({
        "platform": "PC",
        "rating": 9.8,
        "goty": true
    });

    core.update_entity("test", &updated_entity).expect("update entity success");

    // 4. Verify WorkMetadata is safely preserved
    let fetched = core
        .get_entity(&work.id)
        .expect("get")
        .expect("found");
    assert_eq!(fetched.title, "艾尔登法环：黄金树幽影");
    assert_eq!(fetched.description.as_deref(), Some("含 DLC 内容"));

    let preserved_meta = fetched
        .as_work_metadata()
        .expect("WorkMetadata must not be corrupted or stringified into invalid format");
    assert_eq!(preserved_meta.work_type, WorkType::Game);
    assert_eq!(preserved_meta.aliases, vec!["老头环".to_string(), "法环".to_string()]);
    assert_eq!(preserved_meta.progress.as_ref().map(|p| p.position), Some(85.0));
    assert_eq!(preserved_meta.original_title.as_deref(), Some("ELDEN RING"));

    // And verify custom properties are also preserved
    assert_eq!(fetched.properties.get("platform").and_then(|v| v.as_str()), Some("PC"));
    assert_eq!(fetched.properties.get("rating").and_then(|v| v.as_f64()), Some(9.8));
    assert_eq!(fetched.properties.get("goty").and_then(|v| v.as_bool()), Some(true));

    // 5. Test safety guard: if properties.work is set to a string (e.g. from naive frontend JSON.stringify)
    let mut malicious_entity = fetched.clone();
    malicious_entity.properties["work"] = json!("this is an invalid raw string, not a json object");
    let err = core.update_entity("test", &malicious_entity);
    assert!(
        err.is_err(),
        "Updating entity with non-object properties.work must return validation error"
    );
}
