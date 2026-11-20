CREATE TABLE image_url_settings (
    image_id TEXT NOT NULL REFERENCES product_images(id) ON DELETE CASCADE,
    platform TEXT NOT NULL CHECK (length(platform) BETWEEN 1 AND 32),
    store_key TEXT NOT NULL CHECK (length(store_key) BETWEEN 15 AND 77),
    url TEXT NOT NULL CHECK (
        length(CAST(url AS BLOB)) BETWEEN 1 AND 4096
        AND substr(url, 1, 24) = 'https://cdn.shopify.com/'
        AND instr(url, char(0)) = 0
    ),
    image_sha256 TEXT NOT NULL CHECK (
        length(image_sha256) = 64 AND image_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    alt_text TEXT NOT NULL DEFAULT '' CHECK (
        length(alt_text) <= 512 AND instr(alt_text, char(0)) = 0
        AND instr(alt_text, char(10)) = 0 AND instr(alt_text, char(13)) = 0
    ),
    updated_at TEXT NOT NULL CHECK (length(updated_at) > 0),
    PRIMARY KEY (image_id, platform, store_key)
);

CREATE INDEX idx_image_url_settings_store
ON image_url_settings(platform, store_key, image_id);
