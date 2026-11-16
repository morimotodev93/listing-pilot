CREATE TABLE attribute_groups (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL COLLATE BINARY UNIQUE
        CHECK (length(trim(name, char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288))) BETWEEN 1 AND 80),
    archived_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Must match domain::attribute_group::DEFAULT_GROUP_ID. Identity is independent of name.
INSERT INTO attribute_groups VALUES (
    '7a8f0c3e-4d5b-4c6a-9e20-1234567890ab', '既定', NULL,
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
);

-- SQLite cannot ADD a non-null REFERENCES column with a non-null default with FKs ON.
-- Backfill first, then enforce required membership at both write boundaries.
ALTER TABLE products ADD COLUMN group_id TEXT REFERENCES attribute_groups(id) ON DELETE RESTRICT;
UPDATE products SET group_id = '7a8f0c3e-4d5b-4c6a-9e20-1234567890ab';
CREATE TRIGGER products_require_group_insert BEFORE INSERT ON products
WHEN NEW.group_id IS NULL
BEGIN SELECT RAISE(ABORT, 'product group required'); END;
CREATE TRIGGER products_require_group_update BEFORE UPDATE OF group_id ON products
WHEN NEW.group_id IS NULL
BEGIN SELECT RAISE(ABORT, 'product group required'); END;
CREATE INDEX products_group_id ON products(group_id);

CREATE TABLE attribute_definitions_v5 (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL COLLATE BINARY
        CHECK (length(trim(name, char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288))) BETWEEN 1 AND 80),
    data_type TEXT NOT NULL CHECK (data_type IN ('text', 'number')),
    unit TEXT CHECK (unit IS NULL OR length(unit) BETWEEN 1 AND 20),
    archived_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    group_id TEXT NOT NULL REFERENCES attribute_groups(id) ON DELETE RESTRICT,
    UNIQUE (group_id, name),
    CHECK (data_type = 'number' OR unit IS NULL)
);
INSERT INTO attribute_definitions_v5
    SELECT *, '7a8f0c3e-4d5b-4c6a-9e20-1234567890ab' FROM attribute_definitions;

-- Copy children before dropping either old table; ON DELETE RESTRICT stays enabled.
CREATE TABLE product_attribute_values_v5 (
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    attribute_id TEXT NOT NULL REFERENCES attribute_definitions_v5(id) ON DELETE RESTRICT,
    value TEXT NOT NULL
        CHECK (length(trim(value, char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288))) > 0),
    PRIMARY KEY (product_id, attribute_id)
);
INSERT INTO product_attribute_values_v5 SELECT * FROM product_attribute_values;
DROP TABLE product_attribute_values;
DROP TABLE attribute_definitions;
-- SQLite updates the new child's FK target when renaming its parent.
ALTER TABLE attribute_definitions_v5 RENAME TO attribute_definitions;
ALTER TABLE product_attribute_values_v5 RENAME TO product_attribute_values;

CREATE TRIGGER attribute_group_immutable BEFORE UPDATE OF group_id ON attribute_definitions
WHEN NEW.group_id IS NOT OLD.group_id
BEGIN SELECT RAISE(ABORT, 'attribute group immutable'); END;
