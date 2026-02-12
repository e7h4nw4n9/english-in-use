export type BookSourceType = 'Local' | 'CloudflareR2';
export type DatabaseType = 'SQLite' | 'CloudflareD1';

export interface LocalBookSource {
  type: 'Local';
  details: {
    path: string;
  };
}

export interface CloudflareR2BookSource {
  type: 'CloudflareR2';
  details: {
    account_id: string;
    bucket_name: string;
    access_key_id: string;
    secret_access_key: string;
    public_url?: string;
  };
}

export type BookSource = LocalBookSource | CloudflareR2BookSource;

export interface SQLiteDatabase {
  type: 'SQLite';
  details: {
    path: string;
  };
}

export interface CloudflareD1Database {
  type: 'CloudflareD1';
  details: {
    account_id: string;
    database_id: string;
    api_token: string;
  };
}

export type DatabaseConnection = SQLiteDatabase | CloudflareD1Database;

export interface SystemConfig {
  language: string;
  theme: 'system' | 'light' | 'dark';
  log_level: 'trace' | 'debug' | 'info' | 'warn' | 'error';
  enable_auto_check: boolean;
  check_interval_mins: number;
}

export interface AppConfig {
  system: SystemConfig;
  book_source: BookSource | null;
  database: DatabaseConnection | null;
}

export type ServiceStatusType = 
  | { status: 'Connected' }
  | { status: 'Disconnected', message: string }
  | { status: 'NotConfigured' }
  | { status: 'Testing' };

export interface ConnectionStatus {
  r2: ServiceStatusType;
  d1: ServiceStatusType;
}

export interface AppInitProgress {
  message: string;
  progress: number;
}
