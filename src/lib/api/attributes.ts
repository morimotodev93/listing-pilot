import { request } from "./request";
import type {
  AttributeDefinition, CreateAttributeInput, UpdateAttributeInput, SetAttributeArchivedInput,
} from "../../features/attributes/types";

export const listAttributeDefinitions = () => request<AttributeDefinition[]>("list_attribute_definitions");
export const createAttributeDefinition = (input: CreateAttributeInput) =>
  request<AttributeDefinition>("create_attribute_definition", { input });
export const updateAttributeDefinition = (input: UpdateAttributeInput) =>
  request<AttributeDefinition>("update_attribute_definition", { input });
export const setAttributeArchived = (input: SetAttributeArchivedInput) =>
  request<AttributeDefinition>("set_attribute_archived", { input });
