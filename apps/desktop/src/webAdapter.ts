import {
  VaultInfo,
  VaultStats,
  Asset,
  Entity,
  Memory,
  Collection,
  CollectionItem,
  Relation,
  ExternalReference,
  TimelineItem,
  WorkSummary,
} from './types';

const STORAGE_KEY = 'looma_web_mock_vault_v1';

interface WebStore {
  vaultInfo: VaultInfo;
  assets: Asset[];
  entities: Entity[];
  memories: Memory[];
  collections: Collection[];
  collectionItems: CollectionItem[];
  relations: Relation[];
  externalReferences: ExternalReference[];
}

function getDefaultStore(): WebStore {
  return {
    vaultInfo: {
      name: 'Looma 在线演示资产库',
      version: '0.2.0',
      vault_path: 'D:/Personal/Vault',
      database_path: 'D:/Personal/Vault/.looma/vault.db',
      is_initialized: true,
      created_at: '2024-01-01T00:00:00Z',
    },
    assets: [
      {
        id: 'asset_bleach_poster',
        kind: 'image',
        source: 'local',
        path: 'D:/Media/Anime/BLEACH/poster_tybw.jpg',
        size: 2840512,
        hash: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
        mime_type: 'image/jpeg',
        metadata: { width: 1920, height: 1080 },
        status: 'active',
        created_at: '2024-09-10T12:00:00Z',
        modified_at: '2024-09-10T12:00:00Z',
        indexed_at: '2024-10-01T10:00:00Z',
      },
      {
        id: 'asset_wuwa_screenshot',
        kind: 'image',
        source: 'local',
        path: 'D:/Games/WutheringWaves/Screenshots/jinshi_ultimate.png',
        size: 5120800,
        hash: 'a1b2c3d4e5f60718293a4b5c6d7e8f90123456789abcdef0123456789abcdef0',
        mime_type: 'image/png',
        metadata: { width: 3840, height: 2160 },
        status: 'active',
        created_at: '2024-07-02T15:30:00Z',
        modified_at: '2024-07-02T15:30:00Z',
        indexed_at: '2024-10-01T10:00:00Z',
      },
      {
        id: 'asset_frieren_wall',
        kind: 'image',
        source: 'local',
        path: 'D:/Wallpapers/Frieren_sunset_scenery.jpg',
        size: 3410200,
        hash: 'fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210',
        mime_type: 'image/jpeg',
        metadata: { width: 2560, height: 1440 },
        status: 'active',
        created_at: '2024-03-24T18:00:00Z',
        modified_at: '2024-03-24T18:00:00Z',
        indexed_at: '2024-10-01T10:00:00Z',
      },
      {
        id: 'asset_ost_track',
        kind: 'audio',
        source: 'local',
        path: 'D:/Music/ShiroSagisu/NumberOne_2022.flac',
        size: 38400000,
        hash: '1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef',
        mime_type: 'audio/flac',
        metadata: { duration_seconds: 310, bitrate: 960 },
        status: 'active',
        created_at: '2024-08-15T09:00:00Z',
        modified_at: '2024-08-15T09:00:00Z',
        indexed_at: '2024-10-01T10:00:00Z',
      },
      {
        id: 'asset_nier_dir',
        kind: 'directory',
        source: 'local',
        path: 'E:/Games/NieR_Automata',
        size: 51539607552,
        hash: null,
        mime_type: 'inode/directory',
        metadata: { drive: 'E:', files_count: 1420 },
        status: 'active',
        created_at: '2024-03-10T12:00:00Z',
        modified_at: '2024-03-10T12:00:00Z',
        indexed_at: '2024-10-01T10:00:00Z',
      },
      {
        id: 'asset_cp2077_dir1',
        kind: 'directory',
        source: 'local',
        path: 'E:/SteamLibrary/steamapps/common/Cyberpunk 2077',
        size: 78539607552,
        hash: null,
        mime_type: 'inode/directory',
        metadata: { drive: 'E:', source: 'steam' },
        status: 'active',
        created_at: '2024-01-15T12:00:00Z',
        modified_at: '2024-01-15T12:00:00Z',
        indexed_at: '2024-10-01T10:00:00Z',
      },
      {
        id: 'asset_cp2077_dir2',
        kind: 'directory',
        source: 'local',
        path: 'F:/Backup/Cyberpunk2077',
        size: 78539607552,
        hash: null,
        mime_type: 'inode/directory',
        metadata: { drive: 'F:', source: 'cold_backup' },
        status: 'active',
        created_at: '2024-02-01T12:00:00Z',
        modified_at: '2024-02-01T12:00:00Z',
        indexed_at: '2024-10-01T10:00:00Z',
      },
    ],
    entities: [
      {
        id: 'work_bleach',
        entity_type: 'anime',
        title: 'BLEACH 千年血战篇',
        description: '黑崎一护重返尸魂界的壮烈战役，星十字骑士团的入侵与死神全员卍解的重制交响。',
        properties: {
          domain: 'work',
          status: 'in_progress',
          work_type: 'anime',
          work: {
            work_type: 'anime',
            status: 'in_progress',
            original_title: 'BLEACH 千年血戦篇',
            release_year: 2022,
            rating: 9.6,
            progress: {
              position: 13,
              position_type: 'episode',
              total_positions: 24,
              unit: '集',
              updated_at: '2024-10-02T14:00:00Z',
            },
          },
        },
        created_at: '2024-09-01T10:00:00Z',
        updated_at: '2024-10-02T14:00:00Z',
      },
      {
        id: 'work_wuwa',
        entity_type: 'game',
        title: '鸣潮',
        description: '开放世界高机动动作战斗 RPG。漂泊者在重奏与悲鸣的荒野中苏醒，寻找失落的记忆。',
        properties: {
          domain: 'work',
          status: 'in_progress',
          work_type: 'game',
          work: {
            work_type: 'game',
            status: 'in_progress',
            original_title: 'Wuthering Waves',
            release_year: 2024,
            rating: 9.0,
            progress: {
              position: 1.1,
              position_type: 'custom',
              total_positions: null,
              unit: '版本',
              updated_at: '2024-10-01T20:00:00Z',
            },
          },
        },
        created_at: '2024-05-23T10:00:00Z',
        updated_at: '2024-10-01T20:00:00Z',
      },
      {
        id: 'work_frieren',
        entity_type: 'anime',
        title: '葬送的芙莉莲',
        description: '讨伐魔王归来后的千年精灵芙莉莲，在故友纷纷老去离世后，重新踏上探寻人类心之轨迹的温柔旅途。',
        properties: {
          domain: 'work',
          status: 'completed',
          work_type: 'anime',
          work: {
            work_type: 'anime',
            status: 'completed',
            original_title: '葬送のフリーレン',
            release_year: 2023,
            rating: 9.8,
            progress: {
              position: 28,
              position_type: 'episode',
              total_positions: 28,
              unit: '集',
              updated_at: '2024-03-24T22:00:00Z',
            },
          },
        },
        created_at: '2023-10-01T12:00:00Z',
        updated_at: '2024-03-24T22:00:00Z',
      },
      {
        id: 'work_chainsaw',
        entity_type: 'manga',
        title: '电锯人',
        description: '藤本树创作的超现实黑暗奇幻漫画。电次与恶魔波奇塔的契约，在东京特异课开启的荒诞猎魔生涯。',
        properties: {
          domain: 'work',
          status: 'in_progress',
          work_type: 'manga',
          work: {
            work_type: 'manga',
            status: 'in_progress',
            original_title: 'チェンソーマン',
            release_year: 2018,
            rating: 9.2,
            progress: {
              position: 160,
              position_type: 'chapter',
              total_positions: null,
              unit: '话',
              updated_at: '2024-09-28T16:00:00Z',
            },
          },
        },
        created_at: '2024-01-10T10:00:00Z',
        updated_at: '2024-09-28T16:00:00Z',
      },
      {
        id: 'ent_kubo',
        entity_type: 'person',
        title: '久保带人',
        description: '日本著名漫画家，《BLEACH》原作者，以极具张力的构图与诗意的卷首诗闻名。',
        properties: { domain: 'person', subtype: 'author' },
        created_at: '2024-09-01T10:00:00Z',
        updated_at: '2024-09-01T10:00:00Z',
      },
      {
        id: 'ent_kuro',
        entity_type: 'project',
        title: '库洛游戏 (Kuro Games)',
        description: '知名二次元动作游戏开发团队，代表作《鸣潮》、《战双帕弥什》。',
        properties: { domain: 'project' },
        created_at: '2024-05-23T10:00:00Z',
        updated_at: '2024-05-23T10:00:00Z',
      },
      {
        id: 'work_nier',
        entity_type: 'game',
        title: '尼尔：机械纪元',
        description: '探讨存在主义与人造人悲剧命运的动作哲学巨作。本地存储位置: E:\\Games\\NieR_Automata',
        properties: {
          domain: 'work',
          status: 'in_progress',
          work_type: 'game',
          work: {
            work_type: 'game',
            status: 'in_progress',
            original_title: 'NieR:Automata',
            release_year: 2017,
            rating: 9.8,
            progress: {
              position: 3,
              position_type: 'custom',
              total_positions: 5,
              unit: '周目/结局',
              updated_at: '2024-10-03T18:00:00Z',
            },
          },
        },
        created_at: '2024-03-10T12:00:00Z',
        updated_at: '2024-10-03T18:00:00Z',
      },
      {
        id: 'work_cp2077',
        entity_type: 'game',
        title: '赛博朋克 2077',
        description: '夜之城传奇雇佣兵 V 与强尼·银手的交织旅程。已在两处硬盘备份: E:\\SteamLibrary\\Cyberpunk 2077 与 F:\\Backup\\Cyberpunk2077',
        properties: {
          domain: 'work',
          status: 'completed',
          work_type: 'game',
          work: {
            work_type: 'game',
            status: 'completed',
            original_title: 'Cyberpunk 2077',
            release_year: 2020,
            rating: 9.3,
            progress: {
              position: 100,
              position_type: 'percentage',
              total_positions: 100,
              unit: '%',
              updated_at: '2024-08-20T21:00:00Z',
            },
          },
        },
        created_at: '2024-01-15T12:00:00Z',
        updated_at: '2024-08-20T21:00:00Z',
      },
      {
        id: 'work_p5r',
        entity_type: 'game',
        title: '女神异闻录 5 皇家版',
        description: '心之怪盗团偷走腐朽大人的欲望。本地目录: D:\\Games\\Persona5Royal',
        properties: {
          domain: 'work',
          status: 'planned',
          work_type: 'game',
          work: {
            work_type: 'game',
            status: 'planned',
            original_title: 'Persona 5 Royal',
            release_year: 2019,
            rating: 9.7,
            progress: {
              position: 0,
              position_type: 'chapter',
              total_positions: 8,
              unit: '殿堂',
              updated_at: '2024-10-01T10:00:00Z',
            },
          },
        },
        created_at: '2024-09-15T10:00:00Z',
        updated_at: '2024-10-01T10:00:00Z',
      },
    ],
    memories: [
      {
        id: 'mem_bleach_review',
        title: '千年血战篇开播观后感：重置特效直接拉满',
        content: `### 视听盛宴全面升级
开篇作画和鹭巢诗郎配乐重置太震撼了，卍解特效完全拉满！童年回忆直接升华。

- **节奏把控**：去除了早年 TV 版的冗余拖沓，剧情推进疾如雷电；
- **配乐与色彩**：浓郁的赛博冷色调与重金属交响，完全是剧场版级别的规格。`,
        category: 'anime_review',
        metadata: { rating: 9.6, tags: ['BLEACH', 'Anime', 'Review'] },
        recorded_at: '2024-10-02T15:00:00Z',
        created_at: '2024-10-02T15:00:00Z',
        updated_at: '2024-10-02T15:00:00Z',
      },
      {
        id: 'mem_wuwa_log',
        title: '1.1版本乘霄山探索与今汐动作手感体验',
        content: `今汐的动作模组非常惊艳，剧情演出相比 1.0 有了质的飞跃。

乘霄山的纵深箱庭设计非常巧妙，声骸捕捉系统也优化得更加丝滑。期待未来罗盘与探索机制进一步完善。`,
        category: 'gaming',
        metadata: { rating: 9.0, tags: ['鸣潮', 'Gaming', 'Log'] },
        recorded_at: '2024-07-05T20:30:00Z',
        created_at: '2024-07-05T20:30:00Z',
        updated_at: '2024-07-05T20:30:00Z',
      },
    ],
    collections: [
      {
        id: 'col_2024_top',
        title: '2024 年度精选追更清单',
        description: '今年最值得反复回味的高质量文化作品',
        query: null,
        metadata: { icon: 'play-circle', color: '#6366f1' },
        created_at: '2024-01-01T10:00:00Z',
        updated_at: '2024-10-01T10:00:00Z',
      },
      {
        id: 'col_single_games',
        title: '我的单机游戏',
        description: '分散在各本地硬盘与 Steam 平台上的经典单机游戏档案',
        query: null,
        metadata: { icon: 'gamepad-2', color: '#10b981' },
        created_at: '2024-01-01T10:00:00Z',
        updated_at: '2024-10-04T12:00:00Z',
      },
    ],
    collectionItems: [
      {
        collection_id: 'col_2024_top',
        item_id: 'work_bleach',
        item_type: 'entity',
        position: 0,
        added_at: '2024-09-01T10:00:00Z',
      },
      {
        collection_id: 'col_2024_top',
        item_id: 'work_wuwa',
        item_type: 'entity',
        position: 1,
        added_at: '2024-05-23T10:00:00Z',
      },
      {
        collection_id: 'col_2024_top',
        item_id: 'work_frieren',
        item_type: 'entity',
        position: 2,
        added_at: '2024-03-24T10:00:00Z',
      },
      {
        collection_id: 'col_single_games',
        item_id: 'work_nier',
        item_type: 'entity',
        position: 0,
        added_at: '2024-03-10T12:00:00Z',
      },
      {
        collection_id: 'col_single_games',
        item_id: 'work_cp2077',
        item_type: 'entity',
        position: 1,
        added_at: '2024-01-15T12:00:00Z',
      },
      {
        collection_id: 'col_single_games',
        item_id: 'work_p5r',
        item_type: 'entity',
        position: 2,
        added_at: '2024-09-15T10:00:00Z',
      },
    ],
    relations: [
      {
        id: 'rel_bleach_kubo',
        source_id: 'work_bleach',
        source_type: 'entity',
        relation_type: 'created_by',
        target_id: 'ent_kubo',
        target_type: 'entity',
        metadata: {},
        created_at: '2024-09-01T10:00:00Z',
      },
      {
        id: 'rel_bleach_poster',
        source_id: 'work_bleach',
        source_type: 'entity',
        relation_type: 'attaches',
        target_id: 'asset_bleach_poster',
        target_type: 'asset',
        metadata: {},
        created_at: '2024-09-01T10:00:00Z',
      },
      {
        id: 'rel_wuwa_kuro',
        source_id: 'work_wuwa',
        source_type: 'entity',
        relation_type: 'developed_by',
        target_id: 'ent_kuro',
        target_type: 'entity',
        metadata: {},
        created_at: '2024-05-23T10:00:00Z',
      },
      {
        id: 'rel_wuwa_screenshot',
        source_id: 'work_wuwa',
        source_type: 'entity',
        relation_type: 'attaches',
        target_id: 'asset_wuwa_screenshot',
        target_type: 'asset',
        metadata: {},
        created_at: '2024-07-02T15:30:00Z',
      },
      {
        id: 'rel_frieren_wall',
        source_id: 'work_frieren',
        source_type: 'entity',
        relation_type: 'attaches',
        target_id: 'asset_frieren_wall',
        target_type: 'asset',
        metadata: {},
        created_at: '2024-03-24T18:00:00Z',
      },
      {
        id: 'rel_bleach_mem',
        source_id: 'work_bleach',
        source_type: 'entity',
        relation_type: 'referenced_by',
        target_id: 'mem_bleach_review',
        target_type: 'memory',
        metadata: {},
        created_at: '2024-10-02T15:00:00Z',
      },
      {
        id: 'rel_nier_dir',
        source_id: 'work_nier',
        source_type: 'entity',
        relation_type: 'attaches',
        target_id: 'asset_nier_dir',
        target_type: 'asset',
        metadata: {},
        created_at: '2024-03-10T12:00:00Z',
      },
      {
        id: 'rel_cp2077_dir1',
        source_id: 'work_cp2077',
        source_type: 'entity',
        relation_type: 'attaches',
        target_id: 'asset_cp2077_dir1',
        target_type: 'asset',
        metadata: {},
        created_at: '2024-01-15T12:00:00Z',
      },
      {
        id: 'rel_cp2077_dir2',
        source_id: 'work_cp2077',
        source_type: 'entity',
        relation_type: 'attaches',
        target_id: 'asset_cp2077_dir2',
        target_type: 'asset',
        metadata: {},
        created_at: '2024-02-01T12:00:00Z',
      },
    ],
    externalReferences: [
      {
        id: 'ref_bleach_bilibili',
        entity_id: 'work_bleach',
        provider: 'bilibili',
        title: 'Bilibili 追番主页 (千年血战篇)',
        url: 'https://www.bilibili.com/bangumi/play/ss42831',
        description: '正版追番及弹幕互动平台',
        metadata: {},
        created_at: '2024-09-01T10:00:00Z',
        updated_at: '2024-09-01T10:00:00Z',
      },
      {
        id: 'ref_bleach_bgm',
        entity_id: 'work_bleach',
        provider: 'bangumi',
        title: 'Bangumi 番组计划条目',
        url: 'https://bgm.tv/subject/302286',
        description: '评分与演职员阵容数据',
        metadata: {},
        created_at: '2024-09-01T10:00:00Z',
        updated_at: '2024-09-01T10:00:00Z',
      },
      {
        id: 'ref_nier_steam',
        entity_id: 'work_nier',
        provider: 'steam',
        title: 'Steam 商店页 (NieR:Automata)',
        url: 'https://store.steampowered.com/app/524220/NieRAutomata/',
        description: '白金工作室动作战斗',
        metadata: {},
        created_at: '2024-03-10T12:00:00Z',
        updated_at: '2024-03-10T12:00:00Z',
      },
      {
        id: 'ref_cp2077_steam',
        entity_id: 'work_cp2077',
        provider: 'steam',
        title: 'Steam 商店页 (Cyberpunk 2077)',
        url: 'https://store.steampowered.com/app/1091500/Cyberpunk_2077/',
        description: 'CD PROJEKT RED 开放世界第一人称RPG',
        metadata: {},
        created_at: '2024-01-15T12:00:00Z',
        updated_at: '2024-01-15T12:00:00Z',
      },
      {
        id: 'ref_wuwa_official',
        entity_id: 'work_wuwa',
        provider: 'website',
        title: '《鸣潮》官方网站',
        url: 'https://mc.kurogames.com',
        description: '版本公告与官方壁纸下载',
        metadata: {},
        created_at: '2024-05-23T10:00:00Z',
        updated_at: '2024-05-23T10:00:00Z',
      },
      {
        id: 'ref_frieren_bgm',
        entity_id: 'work_frieren',
        provider: 'bangumi',
        title: 'Bangumi 条目 (葬送のフリーレン)',
        url: 'https://bgm.tv/subject/399920',
        description: '9.0 分超高口碑榜首神作',
        metadata: {},
        created_at: '2023-10-01T12:00:00Z',
        updated_at: '2023-10-01T12:00:00Z',
      },
    ],
  };
}

class WebVaultService {
  private store: WebStore;

  constructor() {
    this.store = this.loadStore();
  }

  private loadStore(): WebStore {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        return JSON.parse(saved);
      }
    } catch {
      // fallback
    }
    const def = getDefaultStore();
    this.saveStore(def);
    return def;
  }

  private saveStore(store?: WebStore) {
    if (store) this.store = store;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.store));
    } catch {
      // ignore
    }
  }

  public getVaultInfo(): VaultInfo {
    return this.store.vaultInfo;
  }

  public getVaultStats(): VaultStats {
    let dbBytes = 0;
    try {
      dbBytes = new Blob([JSON.stringify(this.store)]).size;
    } catch {
      dbBytes = 1048576;
    }
    return {
      total_assets: this.store.assets.length,
      total_entities: this.store.entities.length,
      total_relations: this.store.relations.length,
      total_memories: this.store.memories.length,
      total_collections: this.store.collections.length,
      database_size_bytes: dbBytes,
    };
  }

  public listAssets(): Asset[] {
    return [...this.store.assets];
  }

  public getAsset(id: string): Asset | null {
    return this.store.assets.find((a) => a.id === id) || null;
  }

  public listEntities(): Entity[] {
    return [...this.store.entities];
  }

  public getEntity(id: string): Entity | null {
    return this.store.entities.find((e) => e.id === id) || null;
  }

  public createEntity(entity: Entity): Entity {
    this.store.entities.unshift(entity);
    this.saveStore();
    return entity;
  }

  public updateEntity(entity: Entity): Entity {
    const idx = this.store.entities.findIndex((e) => e.id === entity.id);
    if (idx !== -1) {
      this.store.entities[idx] = entity;
      this.saveStore();
    }
    return entity;
  }

  public deleteEntity(id: string): boolean {
    const prev = this.store.entities.length;
    this.store.entities = this.store.entities.filter((e) => e.id !== id);
    this.store.relations = this.store.relations.filter((r) => r.source_id !== id && r.target_id !== id);
    this.saveStore();
    return this.store.entities.length < prev;
  }

  public listWorks(workType?: string | null, status?: string | null): Entity[] {
    return this.store.entities.filter((e) => {
      const meta = (e.properties as any)?.work;
      if (!meta && e.entity_type !== 'anime' && e.entity_type !== 'game' && e.entity_type !== 'manga' && e.entity_type !== 'movie' && e.entity_type !== 'book') {
        return false;
      }
      if (workType && workType !== 'all') {
        const wt = meta?.work_type || e.entity_type;
        if (wt !== workType) return false;
      }
      if (status && status !== 'all') {
        const st = meta?.status || (e.properties as any)?.status;
        if (st !== status) return false;
      }
      return true;
    });
  }

  public createWork(
    title: string,
    kind: string,
    status: string,
    originalTitle?: string | null,
    description?: string | null
  ): Entity {
    const newWork: Entity = {
      id: 'work_' + Math.random().toString(36).substring(2, 9),
      entity_type: kind,
      title,
      description: description || null,
      properties: {
        domain: 'work',
        status,
        work_type: kind,
        work: {
          work_type: kind,
          status,
          original_title: originalTitle || null,
          release_year: new Date().getFullYear(),
          rating: null,
          progress: {
            position: status === 'completed' ? 1 : 0,
            position_type: kind === 'manga' ? 'chapter' : 'episode',
            total_positions: null,
            unit: kind === 'manga' ? '话' : '集',
            updated_at: new Date().toISOString(),
          },
        },
      },
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };
    return this.createEntity(newWork);
  }

  public updateWorkStatus(id: string, status: string): Entity | null {
    const ent = this.getEntity(id);
    if (!ent) return null;
    const props = { ...ent.properties };
    props.status = status;
    if (props.work && typeof props.work === 'object') {
      (props.work as any).status = status;
    }
    const updated = { ...ent, properties: props, updated_at: new Date().toISOString() };
    return this.updateEntity(updated);
  }

  public updateWorkProgress(
    id: string,
    position: number,
    positionType?: string | null,
    totalPositions?: number | null,
    unit?: string | null
  ): Entity | null {
    const ent = this.getEntity(id);
    if (!ent) return null;
    const props = { ...ent.properties } as any;
    if (!props.work) {
      props.work = { work_type: ent.entity_type, status: props.status || 'in_progress' };
    }
    props.work.progress = {
      position,
      position_type: positionType || props.work.progress?.position_type || (ent.entity_type === 'manga' ? 'chapter' : 'episode'),
      total_positions: totalPositions !== undefined ? totalPositions : props.work.progress?.total_positions,
      unit: unit || props.work.progress?.unit || (ent.entity_type === 'manga' ? '话' : '集'),
      updated_at: new Date().toISOString(),
    };
    const updated = { ...ent, properties: props, updated_at: new Date().toISOString() };
    return this.updateEntity(updated);
  }

  public updateWorkCoverAsset(id: string, coverAssetId: string | null): Entity | null {
    const ent = this.getEntity(id);
    if (!ent) return null;
    const props = { ...ent.properties } as any;
    if (props.work && typeof props.work === 'object') {
      props.work.cover_asset_id = coverAssetId;
    }
    props.cover_asset_id = coverAssetId;
    const updated = { ...ent, properties: props, updated_at: new Date().toISOString() };
    return this.updateEntity(updated);
  }

  public getWorkSummary(id: string): WorkSummary | null {
    const ent = this.getEntity(id);
    if (!ent) return null;
    const meta = (ent.properties as any)?.work || null;
    const rels = this.store.relations.filter((r) => r.source_id === id || r.target_id === id);

    const assetIds = new Set(
      rels.filter((r) => r.source_type === 'asset' || r.target_type === 'asset')
        .map((r) => (r.source_id === id ? r.target_id : r.source_id))
    );
    const linked_assets = this.store.assets.filter((a) => assetIds.has(a.id));

    const memIds = new Set(
      rels.filter((r) => r.source_type === 'memory' || r.target_type === 'memory')
        .map((r) => (r.source_id === id ? r.target_id : r.source_id))
    );
    const linked_memories = this.store.memories.filter((m) => memIds.has(m.id));

    const colIds = new Set(
      this.store.collectionItems.filter((ci) => ci.item_id === id).map((ci) => ci.collection_id)
    );
    const linked_collections = this.store.collections.filter((c) => colIds.has(c.id));

    const external_references = this.store.externalReferences.filter((r) => r.entity_id === id);

    return {
      entity: ent,
      work_metadata: meta,
      relations: rels,
      linked_assets,
      linked_memories,
      linked_collections,
      external_references,
    };
  }

  public listMemories(): Memory[] {
    return [...this.store.memories];
  }

  public createMemory(memory: Memory): Memory {
    this.store.memories.unshift(memory);
    this.saveStore();
    return memory;
  }

  public updateMemory(memory: Memory): Memory {
    const idx = this.store.memories.findIndex((m) => m.id === memory.id);
    if (idx !== -1) {
      this.store.memories[idx] = memory;
      this.saveStore();
    }
    return memory;
  }

  public deleteMemory(id: string): boolean {
    const prev = this.store.memories.length;
    this.store.memories = this.store.memories.filter((m) => m.id !== id);
    this.store.relations = this.store.relations.filter((r) => r.source_id !== id && r.target_id !== id);
    this.saveStore();
    return this.store.memories.length < prev;
  }

  public listCollections(): Collection[] {
    return [...this.store.collections];
  }

  public listCollectionItems(collectionId: string): CollectionItem[] {
    return this.store.collectionItems.filter((ci) => ci.collection_id === collectionId);
  }

  public listRelationsForItem(itemId: string): Relation[] {
    return this.store.relations.filter((r) => r.source_id === itemId || r.target_id === itemId);
  }

  public createRelation(relation: Relation): Relation {
    this.store.relations.push(relation);
    this.saveStore();
    return relation;
  }

  public deleteRelationBetween(sourceId: string, targetId: string): boolean {
    const prev = this.store.relations.length;
    this.store.relations = this.store.relations.filter(
      (r) =>
        !(
          (r.source_id === sourceId && r.target_id === targetId) ||
          (r.source_id === targetId && r.target_id === sourceId)
        )
    );
    this.saveStore();
    return this.store.relations.length < prev;
  }

  public listExternalReferences(entityId?: string | null): ExternalReference[] {
    if (entityId) {
      return this.store.externalReferences.filter((r) => r.entity_id === entityId);
    }
    return [...this.store.externalReferences];
  }

  public createExternalReference(reference: ExternalReference): ExternalReference {
    this.store.externalReferences.push(reference);
    this.saveStore();
    return reference;
  }

  public deleteExternalReference(id: string): boolean {
    const prev = this.store.externalReferences.length;
    this.store.externalReferences = this.store.externalReferences.filter((r) => r.id !== id);
    this.saveStore();
    return this.store.externalReferences.length < prev;
  }

  public queryTimeline(): TimelineItem[] {
    const items: TimelineItem[] = [];
    this.store.entities.forEach((e) => {
      items.push({
        id: 'time_e_' + e.id,
        item_type: 'entity',
        timestamp: e.updated_at,
        title: e.title,
        description: (e.properties as any)?.work?.original_title || e.description,
        badge: e.entity_type,
        metadata: e.properties,
      });
    });
    this.store.memories.forEach((m) => {
      items.push({
        id: 'time_m_' + m.id,
        item_type: 'memory',
        timestamp: m.recorded_at,
        title: m.title,
        description: m.content.substring(0, 80) + '...',
        badge: m.category,
        metadata: m.metadata,
      });
    });
    this.store.assets.forEach((a) => {
      items.push({
        id: 'time_a_' + a.id,
        item_type: 'asset',
        timestamp: a.indexed_at,
        title: a.path?.split(/[\\/]/).pop() || a.id,
        description: a.path,
        badge: a.kind,
        metadata: a.metadata,
      });
    });
    return items.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());
  }

  public resetToDefault() {
    this.store = getDefaultStore();
    this.saveStore();
  }
}

export const webVault = new WebVaultService();

export function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && Boolean((window as any).__TAURI_INTERNALS__?.plugins);
}

// Setup mock invoke in Web environment
export function initWebMockAdapter() {
  if (typeof window === 'undefined') return;

  // If running inside Tauri native webview, don't overwrite
  if (isTauriEnvironment()) return;

  (window as any).__LOOMA_WEB_MODE__ = true;

  const mockInvoke = async (cmd: string, args: any = {}) => {
    switch (cmd) {
      case 'get_vault_info':
        return webVault.getVaultInfo();
      case 'get_vault_stats':
        return webVault.getVaultStats();
      case 'list_assets':
        return webVault.listAssets();
      case 'get_asset':
        return webVault.getAsset(args.id);
      case 'list_entities':
        return webVault.listEntities();
      case 'get_entity':
        return webVault.getEntity(args.id);
      case 'create_entity':
        return webVault.createEntity(args.entity);
      case 'update_entity':
        return webVault.updateEntity(args.entity);
      case 'delete_entity':
        return webVault.deleteEntity(args.id);
      case 'list_works':
        return webVault.listWorks(args.workType, args.status);
      case 'create_work':
        return webVault.createWork(args.title, args.kind, args.status, args.originalTitle, args.description);
      case 'update_work_status':
        return webVault.updateWorkStatus(args.id, args.status);
      case 'update_work_progress':
        return webVault.updateWorkProgress(args.id, args.position, args.positionType, args.totalPositions, args.unit);
      case 'update_work_cover_asset':
        return webVault.updateWorkCoverAsset(args.id, args.coverAssetId);
      case 'detect_game_candidates':
        return [
          {
            id: 'cand_nier',
            path: 'E:/Games/NieR_Automata',
            deduced_title: 'NieR Automata',
            drive: 'E:',
            has_executable: true,
            file_count: 24,
            matched_work_id: 'work_nier',
            matched_work_title: '尼尔：机械纪元',
          },
          {
            id: 'cand_p5r',
            path: 'D:/Games/Persona5Royal',
            deduced_title: 'Persona 5 Royal',
            drive: 'D:',
            has_executable: true,
            file_count: 36,
            matched_work_id: 'work_p5r',
            matched_work_title: '女神异闻录 5 皇家版',
          },
          {
            id: 'cand_witcher3',
            path: 'F:/Archive/Witcher3_WildHunt',
            deduced_title: 'The Witcher 3 Wild Hunt',
            drive: 'F:',
            has_executable: true,
            file_count: 18,
            matched_work_id: null,
            matched_work_title: null,
          },
        ];
      case 'get_work_summary':
        return webVault.getWorkSummary(args.id);
      case 'list_memories':
        return webVault.listMemories();
      case 'create_memory':
        return webVault.createMemory(args.memory);
      case 'update_memory':
        return webVault.updateMemory(args.memory);
      case 'delete_memory':
        return webVault.deleteMemory(args.id);
      case 'list_collections':
        return webVault.listCollections();
      case 'list_collection_items':
        return webVault.listCollectionItems(args.collectionId);
      case 'list_relations_for_item':
        return webVault.listRelationsForItem(args.itemId);
      case 'create_relation':
        return webVault.createRelation(args.relation);
      case 'delete_relation_between':
        return webVault.deleteRelationBetween(args.sourceId, args.targetId);
      case 'list_external_references':
        return webVault.listExternalReferences(args.entityId);
      case 'create_external_reference':
        return webVault.createExternalReference(args.reference);
      case 'delete_external_reference':
        return webVault.deleteExternalReference(args.id);
      case 'query_timeline':
        return webVault.queryTimeline();
      case 'open_external_url':
        window.open(args.url, '_blank');
        return;
      case 'open_in_file_manager':
        alert(`[网页演示模式] 原生文件路径：${args.path}\n在 Windows 原生客户端中将直接打开资源管理器定位文件。`);
        return;
      case 'scan_directory':
        return {
          root_path: args.path || 'D:/Personal/Wallpapers',
          duration_ms: 120,
          total_scanned: 4,
          new_assets: 0,
          modified_assets: 0,
          unchanged_assets: 4,
          missing_assets: 0,
          errors: [],
        };
      case 'doctor_inspect':
        return {
          total_assets: 4,
          active_assets: 4,
          missing_assets: [],
          database_size_bytes: 1048576,
          integrity_ok: true,
          integrity_message: 'ok (Web LocalStorage Virtual DB)',
        };
      case 'get_smart_insights':
        return {
          total_suggestions: 2,
          relation_suggestions: [
            {
              id: 'sug_1',
              suggestion_type: 'relation',
              title: '推荐关联：久保带人 ➔ BLEACH 千年血战篇',
              description: '识别到创作者实体与作品标题高度吻合',
              confidence: 0.98,
              source_id: 'ent_kubo',
              source_name: '久保带人',
              target_id: 'work_bleach',
              target_name: 'BLEACH 千年血战篇',
              relation_type: 'created_by',
              tags: ['Anime', 'Author'],
            },
          ],
          tag_suggestions: [],
          cluster_suggestions: [],
        };
      default:
        console.warn(`[Web Mock IPC] Unhandled cmd: ${cmd}`, args);
        return null;
    }
  };

  (window as any).__TAURI_INTERNALS__ = {
    invoke: mockInvoke,
    transformCallback: () => 0,
  };
}
