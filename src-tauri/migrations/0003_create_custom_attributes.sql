CREATE TABLE attribute_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL COLLATE BINARY UNIQUE
        CHECK (length(trim(name, char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288))) BETWEEN 1 AND 80),
    data_type TEXT NOT NULL CHECK (data_type IN ('text', 'number')),
    unit TEXT CHECK (unit IS NULL OR length(unit) BETWEEN 1 AND 20),
    archived_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (data_type = 'number' OR unit IS NULL)
);

-- Values are reserved for Phase 4.2; no value-writing IPC is exposed in 4.1.
CREATE TABLE product_attribute_values (
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    attribute_id TEXT NOT NULL REFERENCES attribute_definitions(id) ON DELETE RESTRICT,
    value TEXT NOT NULL
        CHECK (length(trim(value, char(9,10,11,12,13,32,133,160,5760,8192,8193,8194,8195,8196,8197,8198,8199,8200,8201,8202,8232,8233,8239,8287,12288))) > 0),
    PRIMARY KEY (product_id, attribute_id)
);
