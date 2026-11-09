CREATE TABLE product_images (
    id TEXT PRIMARY KEY NOT NULL,

    product_id TEXT NOT NULL
        REFERENCES products(id) ON DELETE RESTRICT,

    original_filename TEXT NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,

    mime_type TEXT NOT NULL
        CHECK (mime_type IN (
            'image/jpeg',
            'image/png',
            'image/webp'
        )),

    file_size_bytes INTEGER NOT NULL
        CHECK (file_size_bytes > 0),

    width INTEGER NOT NULL CHECK (width > 0),
    height INTEGER NOT NULL CHECK (height > 0),

    sha256 TEXT NOT NULL CHECK (length(sha256) = 64),
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    created_at TEXT NOT NULL,

    UNIQUE(product_id, sha256)
);

CREATE INDEX idx_product_images_product_order
ON product_images (
    product_id,
    sort_order,
    created_at,
    id
);
