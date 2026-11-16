export type AttributeDataType = "text" | "number";

export const DEFAULT_GROUP_ID = "7a8f0c3e-4d5b-4c6a-9e20-1234567890ab";

export type AttributeDefinition = {
  id: string;
  groupId: string;
  name: string;
  dataType: AttributeDataType;
  unit: string | null;
  archivedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CreateAttributeInput = {
  groupId?: string;
  name: string;
  dataType: AttributeDataType;
  unit: string | null;
};

export type UpdateAttributeInput = {
  id: string;
  name: string;
  expectedUpdatedAt: string;
};

export type SetAttributeArchivedInput = {
  id: string;
  archived: boolean;
  expectedUpdatedAt: string;
};
