export interface VaultInfo {
  name: string;
  version: string;
  vault_path: string;
  database_path: string;
  is_initialized: boolean;
  created_at: string | null;
}

export interface VaultStats {
  total_assets: number;
  total_entities: number;
  total_relations: number;
  total_memories: number;
  total_collections: number;
  database_size_bytes: number;
}

export type AssetKind = 'image' | 'video' | 'audio' | 'document' | 'archive' | 'model3d' | 'code' | 'other';
export type AssetSource = 'local' | 'managed' | 'external';
export type AssetStatus = 'active' | 'missing' | 'archived';

export interface Asset {
  id: string;
  kind: AssetKind;
  source: AssetSource;
  path: string | null;
  size: number | null;
  hash: string | null;
  mime_type: string | null;
  metadata: Record<string, unknown>;
  status: AssetStatus;
  created_at: string;
  modified_at: string;
  indexed_at: string;
}

export interface Entity {
  id: string;
  entity_type: string;
  title: string;
  description: string | null;
  properties: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface Memory {
  id: string;
  title: string;
  content: string;
  category: string | null;
  metadata: Record<string, unknown>;
  recorded_at: string;
  created_at: string;
  updated_at: string;
}

export interface Collection {
  id: string;
  title: string;
  description: string | null;
  query: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}
