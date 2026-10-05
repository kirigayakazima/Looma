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

export type AssetKind = 'image' | 'video' | 'audio' | 'document' | 'archive' | 'model3d' | 'code' | 'directory' | 'other';
export type AssetSource = 'local' | 'managed' | 'external';
export type AssetStatus = 'active' | 'missing' | 'archived';

export interface GameCandidate {
  id: string;
  path: string;
  deduced_title: string;
  drive: string;
  has_executable: boolean;
  file_count: number;
  matched_work_id?: string | null;
  matched_work_title?: string | null;
}

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

export interface AssetFilter {
  kind?: AssetKind;
  status?: AssetStatus;
  search_query?: string;
  limit?: number;
  offset?: number;
}

export interface ScanProgress {
  total_discovered: number;
  new_assets: number;
  modified_assets: number;
  unchanged_assets: number;
  current_file: string | null;
}

export interface ScanSummary {
  root_path: string;
  duration_ms: number;
  total_scanned: number;
  new_assets: number;
  modified_assets: number;
  unchanged_assets: number;
  missing_assets: number;
  errors: string[];
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

export interface Relation {
  id: string;
  source_id: string;
  source_type: string;
  relation_type: string;
  target_id: string;
  target_type: string;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface CollectionItem {
  collection_id: string;
  item_id: string;
  item_type: string;
  position: number;
  added_at: string;
}

export interface TimelineItem {
  id: string;
  item_type: 'asset' | 'entity' | 'memory';
  timestamp: string;
  title: string;
  description: string | null;
  badge: string | null;
  metadata: Record<string, unknown>;
}

export interface TimelineFilter {
  item_types?: string[];
  limit?: number;
  offset?: number;
}

export interface VaultManifest {
  manifest_version: string;
  created_at: string;
  vault_info: VaultInfo;
  stats: VaultStats;
  assets_count: number;
  entities_count: number;
  memories_count: number;
  collections_count: number;
  relations_count: number;
}

export interface BackupResult {
  backup_path: string;
  manifest: VaultManifest;
  duration_ms: number;
}

export interface RestoreResult {
  restored_assets: number;
  restored_entities: number;
  restored_memories: number;
  restored_collections: number;
  restored_relations: number;
  duration_ms: number;
}

export interface VaultDoctorReport {
  total_assets: number;
  active_assets: number;
  missing_assets: string[];
  database_size_bytes: number;
  integrity_ok: boolean;
  integrity_message: string;
}

export type SuggestionType = 'relation' | 'tag' | 'cluster';

export interface Suggestion {
  id: string;
  suggestion_type: SuggestionType;
  title: string;
  description: string;
  confidence: number;
  source_id: string;
  source_name: string;
  target_id?: string;
  target_name?: string;
  relation_type?: string;
  tags: string[];
  action_payload?: Record<string, unknown>;
}

export interface SmartInsightsReport {
  total_suggestions: number;
  relation_suggestions: Suggestion[];
  tag_suggestions: Suggestion[];
  cluster_suggestions: Suggestion[];
}

export interface ExternalReference {
  id: string;
  entity_id: string | null;
  provider: string;
  title: string;
  url: string;
  description: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export type WorkType =
  | 'anime'
  | 'manga'
  | 'game'
  | 'movie'
  | 'tv_series'
  | 'music'
  | 'album'
  | 'book'
  | 'novel'
  | 'documentary'
  | 'other';

export type RecordStatus =
  | 'planned'
  | 'in_progress'
  | 'completed'
  | 'paused'
  | 'dropped'
  | 'revisit'
  | 'archived'
  | 'unknown';

export type ProgressPositionType =
  | 'episode'
  | 'chapter'
  | 'page'
  | 'minute'
  | 'percentage'
  | string;

export interface WorkProgress {
  position: number;
  position_type: ProgressPositionType;
  total_positions: number | null;
  unit: string | null;
  updated_at: string;
}

export interface WorkMetadata {
  work_type: WorkType;
  status: RecordStatus;
  original_title: string | null;
  release_year: number | null;
  start_date: string | null;
  end_date: string | null;
  cover_asset_id: string | null;
  rating: number | null;
  progress?: WorkProgress | null;
  aliases?: string[];
  type_metadata?: Record<string, unknown> | null;
}

export interface WorkSummary {
  entity: Entity;
  work_metadata: WorkMetadata | null;
  relations: Relation[];
  linked_assets: Asset[];
  linked_memories: Memory[];
  linked_collections: Collection[];
  external_references: ExternalReference[];
}

