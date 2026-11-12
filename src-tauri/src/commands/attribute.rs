use super::execute;
use crate::{
    domain::attribute::{
        AttributeDefinition, CreateAttributeInput, SetAttributeArchivedInput, UpdateAttributeInput,
    },
    error::AppError,
    infrastructure::db::Database,
    services::attribute_service,
};
use tauri::State;

#[tauri::command]
pub async fn list_attribute_definitions(
    state: State<'_, Database>,
) -> Result<Vec<AttributeDefinition>, AppError> {
    execute(state, attribute_service::list).await
}

#[tauri::command]
pub async fn create_attribute_definition(
    state: State<'_, Database>,
    input: CreateAttributeInput,
) -> Result<AttributeDefinition, AppError> {
    execute(state, move |connection| {
        attribute_service::create(connection, input)
    })
    .await
}

#[tauri::command]
pub async fn update_attribute_definition(
    state: State<'_, Database>,
    input: UpdateAttributeInput,
) -> Result<AttributeDefinition, AppError> {
    execute(state, move |connection| {
        attribute_service::update(connection, input)
    })
    .await
}

#[tauri::command]
pub async fn set_attribute_archived(
    state: State<'_, Database>,
    input: SetAttributeArchivedInput,
) -> Result<AttributeDefinition, AppError> {
    execute(state, move |connection| {
        attribute_service::set_archived(connection, input)
    })
    .await
}
