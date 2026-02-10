export type BookSourceType = 'Local' | 'CloudflareR2';
export type DatabaseType = 'PostgreSQL';

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

export interface PostgreSQLDatabase {
  type: 'PostgreSQL';
  details: {
    host: string;
    port: number;
    user: string;
    password?: string;
    database: string;
    ssl: boolean;
  };
}

export type DatabaseConnection = PostgreSQLDatabase;

export interface SystemConfig {
  language: string;
  theme: 'system' | 'light' | 'dark';
}

export interface AppConfig {
  system: SystemConfig;
  book_source: BookSource | null;
  database: DatabaseConnection | null;
}
