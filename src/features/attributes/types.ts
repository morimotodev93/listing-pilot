export type AttributeDataType = "text" | "number";

export type AttributeDefinition = {
  id: string;
  name: string;
  dataType: AttributeDataType;
  unit: string | null;
  archivedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type CreateAttributeInput = {
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
