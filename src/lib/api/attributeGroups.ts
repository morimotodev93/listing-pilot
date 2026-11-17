import { request } from "./request";
import type { AttributeGroup, CreateAttributeGroupInput, UpdateAttributeGroupInput, SetAttributeGroupArchivedInput } from "../../features/attributes/types";

export const listAttributeGroups = () => request<AttributeGroup[]>("list_attribute_groups");
export const createAttributeGroup = (input: CreateAttributeGroupInput) =>
  request<AttributeGroup>("create_attribute_group", { input });
export const updateAttributeGroup = (input: UpdateAttributeGroupInput) =>
  request<AttributeGroup>("update_attribute_group", { input });
export const setAttributeGroupArchived = (input: SetAttributeGroupArchivedInput) =>
  request<AttributeGroup>("set_attribute_group_archived", { input });
