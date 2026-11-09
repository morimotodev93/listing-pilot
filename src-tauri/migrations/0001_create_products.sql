CREATE TABLE products (
    id TEXT PRIMARY KEY NOT NULL,
    sku TEXT NOT NULL COLLATE BINARY UNIQUE CHECK(length(trim(sku)) > 0 AND sku = trim(sku)),
    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
    brand TEXT,
    price_yen INTEGER CHECK(price_yen IS NULL OR (typeof(price_yen) = 'integer' AND price_yen BETWEEN 0 AND 9007199254740991)),
    description_note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
