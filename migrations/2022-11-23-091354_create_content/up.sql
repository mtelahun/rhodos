CREATE TABLE content (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    publisher_id BIGINT NOT NULL,
    cw VARCHAR,
    body VARCHAR,
    published BOOLEAN,
    published_at TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_publisher
        FOREIGN KEY(publisher_id)
            REFERENCES account
);

SELECT diesel_manage_updated_at('content');
