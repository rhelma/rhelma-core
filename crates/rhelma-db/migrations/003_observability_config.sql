-- observability configuration tables
CREATE TABLE IF NOT EXISTS observability_defaults (
    id SERIAL PRIMARY KEY,
    json_logs BOOLEAN,
    console_logs BOOLEAN,
    log_level TEXT,
    sampling_rate DOUBLE PRECISION,
    otel_enabled BOOLEAN,
    otel_endpoint TEXT,
    metrics_enabled BOOLEAN,
    prometheus_port INTEGER,
    performance_profile TEXT
);

CREATE TABLE IF NOT EXISTS observability_regions (
    region TEXT PRIMARY KEY,
    json_logs BOOLEAN,
    console_logs BOOLEAN,
    log_level TEXT,
    sampling_rate DOUBLE PRECISION,
    otel_enabled BOOLEAN,
    otel_endpoint TEXT,
    metrics_enabled BOOLEAN,
    prometheus_port INTEGER,
    performance_profile TEXT
);

CREATE TABLE IF NOT EXISTS observability_services (
    region TEXT NOT NULL,
    service_name TEXT NOT NULL,
    json_logs BOOLEAN,
    console_logs BOOLEAN,
    log_level TEXT,
    sampling_rate DOUBLE PRECISION,
    otel_enabled BOOLEAN,
    otel_endpoint TEXT,
    metrics_enabled BOOLEAN,
    prometheus_port INTEGER,
    performance_profile TEXT,
    PRIMARY KEY (region, service_name)
);
