use super::execute;
use crate::{
    domain::attribute_group::*, error::AppError, infrastructure::db::Database,
    services::attribute_group_service as service,
};
use tauri::State;

#[tauri::command]
pub async fn list_attribute_groups(
    state: State<'_, Database>,
) -> Result<Vec<AttributeGroup>, AppError> {
    execute(state, service::list).await
}

#[tauri::command]
pub async fn create_attribute_group(
    state: State<'_, Database>,
    input: CreateAttributeGroupInput,
) -> Result<AttributeGroup, AppError> {
    execute(state, move |connection| service::create(connection, input)).await
}

#[tauri::command]
pub async fn update_attribute_group(
    state: State<'_, Database>,
    input: UpdateAttributeGroupInput,
) -> Result<AttributeGroup, AppError> {
    execute(state, move |connection| service::update(connection, input)).await
}

#[tauri::command]
pub async fn set_attribute_group_archived(
    state: State<'_, Database>,
    input: SetAttributeGroupArchivedInput,
) -> Result<AttributeGroup, AppError> {
    execute(state, move |connection| {
        service::set_archived(connection, input)
    })
    .await
}
