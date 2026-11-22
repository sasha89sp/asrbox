import {useMemo, useState} from 'react';
import {Check, Copy, FileCode2, FileText, FolderOpen, Search, ShieldCheck, Terminal} from 'lucide-react';

import gitignoreRaw from '../.gitignore?raw';
import cargoConfigRaw from '../.cargo/config.toml?raw';
import ciYmlRaw from '../.github/workflows/ci.yml?raw';
import releaseYmlRaw from '../.github/workflows/release.yml?raw';
import rootCargoRaw from '../Cargo.toml?raw';
import agentsRaw from '../AGENTS.md?raw';
import specRaw from '../SPEC.md?raw';
import atlasRaw from '../ATLAS.md?raw';
import handoffRaw from '../HANDOFF.yaml?raw';
import architectureRaw from '../ARCHITECTURE.md?raw';
import decisionsRaw from '../DECISIONS.md?raw';
import bugsRaw from '../BUGS.md?raw';
import changelogRaw from '../CHANGELOG.md?raw';
import releaseChecklistRaw from '../docs/RELEASE_CHECKLIST.md?raw';
import licensesRaw from '../docs/LICENSES.md?raw';
import bugsClosedRaw from '../docs/archive/bugs-closed.md?raw';
import specsReadmeRaw from '../specs/README.md?raw';

const crateModules = import.meta.glob<string>(
  ['../crates/**/*.toml', '../crates/**/*.rs', '../crates/**/*.md', '../xtask/**/*.toml', '../xtask/**/*.rs', '../xtask/**/*.md'],
  {query: '?raw', import: 'default', eager: true},
);

type TabKey = 'files' | 'architecture' | 'budgets' | 'ci';
type FileCategory = 'all' | 'crates' | 'docs' | 'config';

interface WorkspaceFile {
  path: string;
  content: string;
  category: Exclude<FileCategory, 'all'>;
  group: string;
}

interface AtlasCheckIssue {
  scope: string;
  message: string;
}

interface AtlasCheckSummary {
  passed: boolean;
  cratesChecked: number;
  rustFilesChecked: number;
  issues: AtlasCheckIssue[];
}

const NF_BUDGETS = [
  {
    id: 'NF-001',
    name: 'CPU в покое',
    budget: 'Меньше 1% (события вместо опроса по таймеру)',
    verification: 'Наблюдение через mod-monitor в фоне',
  },
  {
    id: 'NF-002',
    name: 'RAM в покое (без Whisper)',
    budget: 'Меньше 30 МБ (ленивая загрузка Whisper, полное освобождение после стопа)',
    verification: 'mod-monitor и Windows Task Manager',
  },
  {
    id: 'NF-003',
    name: 'Запуск до готовности трея',
    budget: 'Меньше 1 сек',
    verification: 'Замер времени старта в логе bootstrap',
  },
  {
    id: 'NF-004',
    name: 'Обработка хука клавиатуры',
    budget: 'Меньше 1 мс на событие (без тяжёлого кода внутри коллбэка)',
    verification: 'Профилировщик tracing::instrument в mod-keyboard',
  },
  {
    id: 'NF-005',
    name: 'Размер portable .exe',
    budget: 'Меньше 25 МБ для asrbox.exe (x64 release)',
    verification: 'Автопроверка размера в ci.yml и release.yml',
  },
  {
    id: 'NF-006',
    name: 'Изоляция падений модулей',
    budget: 'Падение любого модуля не закрывает программу и другие модули',
    verification: 'Отдельные процессы (--module <имя>) и супервизор в app',
  },
  {
    id: 'NF-007',
    name: 'Выключенный модуль',
    budget: '0% CPU и 0 МБ RAM (выключенный модуль не загружается)',
    verification: 'Проверка списка процессов при enabled = false',
  },
  {
    id: 'NF-008',
    name: 'Перезапуск упавшего модуля',
    budget: 'Не дольше 3 сек (бэкофф 1 / 5 / 30 сек, отключение после 3 падений подряд)',
    verification: 'Тест супервизора с уведомлением в трее и записью в лог',
  },
  {
    id: 'NF-009',
    name: 'Единая версия',
    budget: 'workspace.package.version в имени .exe, --version, трее, «О программе» и логах',
    verification: 'Проверка совпадения тега в release.yml и сверка версии дочерних процессов',
  },
  {
    id: 'NF-010',
    name: 'Логи, профилирование и монитор',
    budget: 'Вкл/выкл логов (5x2 МБ, off..trace) и tracing::instrument на лету + mod-monitor + ZIP диагностики',
    verification: 'Онлайн-монитор нагрузки и выгрузка диагностического ZIP из трея',
  },
];

const CRATE_LAYERS = [
  {
    layer: 'Точка входа и супервизор (один .exe)',
    crates: [
      {
        path: 'crates/app',
        pkg: 'asrbox',
        deps: 'core, platform-api, platform-windows, mod-*',
        unsafeAllowed: 'Запрещён',
        role: 'Супервизор (--module, --safe-mode, --selftest), трей, окно настроек (X -> трей)',
      },
    ],
  },
  {
    layer: 'Прикладные модули (строго core + platform-api, связь через Named Pipes)',
    crates: [
      {
        path: 'crates/mod-keyboard',
        pkg: 'asrbox-mod-keyboard',
        deps: 'core, platform-api',
        unsafeAllowed: 'Запрещён',
        role: 'Отдельный процесс · Переназначение клавиш и аварийное отключение',
      },
      {
        path: 'crates/mod-dock',
        pkg: 'asrbox-mod-dock',
        deps: 'core, platform-api',
        unsafeAllowed: 'Запрещён',
        role: 'Один процесс на весь док · 8 независимых док-панелей в своих потоках',
      },
      {
        path: 'crates/mod-speech',
        pkg: 'asrbox-mod-speech',
        deps: 'core, platform-api',
        unsafeAllowed: 'Запрещён',
        role: 'Отдельный процесс · Голос в текст (Whisper, ленивая загрузка + SHA-256)',
      },
      {
        path: 'crates/mod-webdav',
        pkg: 'asrbox-mod-webdav',
        deps: 'core, platform-api',
        unsafeAllowed: 'Запрещён',
        role: 'Отдельный процесс · WebDAV и синхронизация папок (секреты в Credential Manager)',
      },
      {
        path: 'crates/mod-monitor',
        pkg: 'asrbox-mod-monitor',
        deps: 'core, platform-api',
        unsafeAllowed: 'Запрещён',
        role: 'Процесс при открытом окне · Онлайн-монитор CPU/RAM/событий/функций',
      },
      {
        path: 'crates/mod-screenshot',
        pkg: 'asrbox-mod-screenshot',
        deps: 'core, platform-api',
        unsafeAllowed: 'Запрещён',
        role: 'Процесс по запросу · Скриншоты (живёт секунды и закрывается)',
      },
      {
        path: 'crates/mod-huawei',
        pkg: 'asrbox-mod-huawei',
        deps: 'core, platform-api',
        unsafeAllowed: 'Запрещён',
        role: 'Отдельный процесс · Модем Huawei (AT-команды, изоляция сбоев порта)',
      },
    ],
  },
  {
    layer: 'Ядро и платформенный слой',
    crates: [
      {
        path: 'crates/core',
        pkg: 'asrbox-core',
        deps: 'Нет внутренних зависимостей',
        unsafeAllowed: 'Запрещён',
        role: 'Настройки (settings_version/written_by), шина, трейт Module, логи, профилировщик',
      },
      {
        path: 'crates/platform-api',
        pkg: 'asrbox-platform-api',
        deps: 'core',
        unsafeAllowed: 'Запрещён',
        role: 'Трейты KeyboardHook, DockWindow, ScreenCapture, ProcessMonitor',
      },
      {
        path: 'crates/platform-windows',
        pkg: 'asrbox-platform-windows',
        deps: 'core, platform-api',
        unsafeAllowed: 'Разрешён (// SAFETY)',
        role: 'Реализация трейтов через windows-rs (единственный крейт с unsafe)',
      },
    ],
  },
  {
    layer: 'Инструментарий workspace',
    crates: [
      {
        path: 'xtask',
        pkg: 'xtask',
        deps: 'Только Rust std',
        unsafeAllowed: 'Запрещён',
        role: 'Команда cargo xtask atlas-check (проверка ATLAS.md и MAP.md)',
      },
    ],
  },
];

const CI_STEPS = [
  {order: '01', name: 'Check formatting', command: 'cargo fmt --check'},
  {order: '02', name: 'Run Clippy', command: 'cargo clippy --workspace -- -D warnings'},
  {order: '03', name: 'Verify ATLAS and MAP coverage', command: 'cargo xtask atlas-check'},
  {order: '04', name: 'Run tests', command: 'cargo test --workspace'},
  {order: '05', name: 'Build release binary', command: 'cargo build --release'},
  {order: '06', name: 'Check size limit (NF-005 < 25 MB)', command: 'target/release/asrbox.exe < 25 MB'},
  {order: '07', name: 'Smoke test (--selftest)', command: '.\\target\\release\\asrbox.exe --selftest'},
  {order: '08', name: 'Upload portable artifact', command: 'actions/upload-artifact@v4'},
];

function buildWorkspaceFiles(): WorkspaceFile[] {
  const staticFiles: WorkspaceFile[] = [
    {path: '.gitignore', content: gitignoreRaw, category: 'config', group: 'Корень и CI'},
    {path: 'Cargo.toml', content: rootCargoRaw, category: 'config', group: 'Корень и CI'},
    {path: '.cargo/config.toml', content: cargoConfigRaw, category: 'config', group: 'Корень и CI'},
    {path: '.github/workflows/ci.yml', content: ciYmlRaw, category: 'config', group: 'Корень и CI'},
    {path: '.github/workflows/release.yml', content: releaseYmlRaw, category: 'config', group: 'Корень и CI'},
    {path: 'AGENTS.md', content: agentsRaw, category: 'docs', group: 'Документация'},
    {path: 'SPEC.md', content: specRaw, category: 'docs', group: 'Документация'},
    {path: 'ATLAS.md', content: atlasRaw, category: 'docs', group: 'Документация'},
    {path: 'HANDOFF.yaml', content: handoffRaw, category: 'docs', group: 'Документация'},
    {path: 'ARCHITECTURE.md', content: architectureRaw, category: 'docs', group: 'Документация'},
    {path: 'DECISIONS.md', content: decisionsRaw, category: 'docs', group: 'Документация'},
    {path: 'BUGS.md', content: bugsRaw, category: 'docs', group: 'Документация'},
    {path: 'CHANGELOG.md', content: changelogRaw, category: 'docs', group: 'Документация'},
    {path: 'specs/README.md', content: specsReadmeRaw, category: 'docs', group: 'Спецификации и docs'},
    {path: 'docs/RELEASE_CHECKLIST.md', content: releaseChecklistRaw, category: 'docs', group: 'Спецификации и docs'},
    {path: 'docs/LICENSES.md', content: licensesRaw, category: 'docs', group: 'Спецификации и docs'},
    {path: 'docs/archive/bugs-closed.md', content: bugsClosedRaw, category: 'docs', group: 'Спецификации и docs'},
  ];

  const dynamicFiles: WorkspaceFile[] = Object.entries(crateModules).map(([rawKey, content]) => {
    const normalized = rawKey.replace(/^\.\.\//, '');
    const parts = normalized.split('/');
    const group = parts[0] === 'crates' ? `crates/${parts[1]}` : parts[0];
    return {
      path: normalized,
      content,
      category: 'crates',
      group,
    };
  });

  dynamicFiles.sort((a, b) => a.path.localeCompare(b.path));
  return [...staticFiles, ...dynamicFiles];
}

function runClientAtlasCheck(files: WorkspaceFile[]): AtlasCheckSummary {
  const fileMap = new Map(files.map((f) => [f.path, f.content]));
  const issues: AtlasCheckIssue[] = [];
  const atlas = fileMap.get('ATLAS.md');

  if (atlas === undefined) {
    return {
      passed: false,
      cratesChecked: 0,
      rustFilesChecked: 0,
      issues: [{scope: 'ATLAS.md', message: 'Файл ATLAS.md не найден в корне репозитория'}],
    };
  }

  const crateRoots = new Set<string>();
  for (const file of files) {
    if (file.path.endsWith('/Cargo.toml')) {
      crateRoots.add(file.path.slice(0, -'/Cargo.toml'.length));
    }
  }

  const sortedCrates = Array.from(crateRoots).sort();
  let rustFilesChecked = 0;

  for (const crateRoot of sortedCrates) {
    if (!atlas.includes(crateRoot)) {
      issues.push({
        scope: 'ATLAS.md',
        message: `Крейт '${crateRoot}' не упомянут в ATLAS.md`,
      });
    }

    const mapPath = `${crateRoot}/MAP.md`;
    const mapContent = fileMap.get(mapPath);
    if (mapContent === undefined) {
      issues.push({
        scope: mapPath,
        message: `Отсутствует файл карты ${mapPath}`,
      });
      continue;
    }

    const prefix = `${crateRoot}/`;
    const rsFiles = files
      .filter((f) => f.path.startsWith(prefix) && f.path.endsWith('.rs'))
      .map((f) => f.path.slice(prefix.length))
      .sort();

    for (const relRs of rsFiles) {
      rustFilesChecked += 1;
      if (!mapContent.includes(relRs)) {
        issues.push({
          scope: mapPath,
          message: `Файл '${relRs}' не упомянут в ${mapPath}`,
        });
      }
    }
  }

  return {
    passed: issues.length === 0,
    cratesChecked: sortedCrates.length,
    rustFilesChecked,
    issues,
  };
}

export default function App() {
  const allFiles = useMemo(() => buildWorkspaceFiles(), []);
  const atlasReport = useMemo(() => runClientAtlasCheck(allFiles), [allFiles]);

  const [activeTab, setActiveTab] = useState<TabKey>('files');
  const [selectedPath, setSelectedPath] = useState<string>('Cargo.toml');
  const [categoryFilter, setCategoryFilter] = useState<FileCategory>('all');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [copiedPath, setCopiedPath] = useState<string | null>(null);

  const filteredFiles = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    return allFiles.filter((file) => {
      if (categoryFilter !== 'all' && file.category !== categoryFilter) {
        return false;
      }
      if (!query) {
        return true;
      }
      return (
        file.path.toLowerCase().includes(query) ||
        file.group.toLowerCase().includes(query)
      );
    });
  }, [allFiles, categoryFilter, searchQuery]);

  const groupedFiles = useMemo(() => {
    const groups = new Map<string, WorkspaceFile[]>();
    for (const file of filteredFiles) {
      const list = groups.get(file.group) ?? [];
      list.push(file);
      groups.set(file.group, list);
    }
    return Array.from(groups.entries());
  }, [filteredFiles]);

  const activeFile = useMemo(
    () => allFiles.find((f) => f.path === selectedPath) ?? allFiles[0],
    [allFiles, selectedPath],
  );

  const activeLines = useMemo(() => {
    if (!activeFile || activeFile.content.length === 0) {
      return [];
    }
    const normalized = activeFile.content.replace(/\r\n/g, '\n');
    const lines = normalized.split('\n');
    if (lines.length > 0 && lines[lines.length - 1] === '') {
      lines.pop();
    }
    return lines;
  }, [activeFile]);

  const handleCopyContent = async (path: string, content: string) => {
    try {
      await navigator.clipboard.writeText(content);
      setCopiedPath(path);
      window.setTimeout(() => {
        setCopiedPath((prev) => (prev === path ? null : prev));
      }, 1800);
    } catch {
      setCopiedPath(null);
    }
  };

  return (
    <div className="min-h-screen bg-slate-50 text-slate-900 flex flex-col">
      {/* Three-zone Top Bar Contract */}
      <header className="sticky top-0 z-20 bg-white border-b border-slate-200">
        <div className="max-w-[1440px] mx-auto px-6 h-14 flex items-center justify-between gap-8">
          <button
            type="button"
            onClick={() => setActiveTab('files')}
            className="text-lg font-bold tracking-tight text-slate-950 whitespace-nowrap shrink-0 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-600 rounded"
          >
            asrbox
          </button>

          <nav className="flex items-center gap-6 overflow-x-auto" aria-label="Разделы проекта">
            {(
              [
                ['files', 'Файлы workspace'],
                ['architecture', 'Архитектура'],
                ['budgets', 'Бюджеты NF'],
                ['ci', 'CI и Atlas Check'],
              ] as const
            ).map(([key, label]) => {
              const isActive = activeTab === key;
              return (
                <button
                  key={key}
                  type="button"
                  onClick={() => setActiveTab(key)}
                  className={`py-4 text-sm font-medium border-b-2 transition-colors whitespace-nowrap shrink-0 focus-visible:outline-2 focus-visible:outline-blue-600 ${
                    isActive
                      ? 'border-blue-700 text-slate-950'
                      : 'border-transparent text-slate-600 hover:text-slate-900'
                  }`}
                >
                  {label}
                </button>
              );
            })}
          </nav>

          <div className="flex items-center shrink-0">
            <button
              type="button"
              onClick={() => {
                setActiveTab('ci');
              }}
              className="px-4 py-2 text-xs font-semibold text-white bg-slate-900 hover:bg-slate-800 rounded-md transition-colors whitespace-nowrap shrink-0 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-600"
            >
              Проверить Atlas
            </button>
          </div>
        </div>
      </header>

      {/* Summary strip */}
      <section className="bg-white border-b border-slate-200">
        <div className="max-w-[1440px] mx-auto px-6 py-4 flex flex-wrap items-center justify-between gap-4">
          <div>
            <h1 className="text-xl font-semibold tracking-tight text-slate-950">
              Каркас Rust workspace для Windows 10/11 (x64)
            </h1>
            <p className="mt-1 text-sm text-slate-600">
              Версия 0.0.1 · Портативная сборка <span className="font-mono text-slate-800">asrbox.exe</span> · Проверка на GitHub Actions (<span className="font-mono text-slate-800">windows-latest</span>)
            </p>
          </div>
          <div className="flex items-center gap-6 text-xs text-slate-600 font-mono tabular-nums">
            <span>Крейтов: <strong className="text-slate-950">{atlasReport.cratesChecked}</strong></span>
            <span aria-hidden="true">·</span>
            <span>Файлов каркаса: <strong className="text-slate-950">{allFiles.length}</strong></span>
            <span aria-hidden="true">·</span>
            <span>
              Atlas Check:{' '}
              <strong className={atlasReport.passed ? 'text-emerald-700' : 'text-red-700'}>
                {atlasReport.passed ? 'OK' : `Ошибок: ${atlasReport.issues.length}`}
              </strong>
            </span>
          </div>
        </div>
      </section>

      {/* Main Content */}
      <main className="flex-1 max-w-[1440px] w-full mx-auto px-6 py-6">
        {activeTab === 'files' && (
          <div className="grid grid-cols-1 lg:grid-cols-[280px_1fr] gap-6 items-start">
            {/* Sidebar File Tree */}
            <aside className="bg-white border border-slate-200 rounded-lg overflow-hidden">
              <div className="p-3 border-b border-slate-200 space-y-2.5">
                <div className="relative">
                  <Search className="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
                  <input
                    type="text"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    placeholder="Поиск по пути..."
                    aria-label="Поиск файлов по пути"
                    className="w-full pl-9 pr-3 py-1.5 text-xs bg-slate-50 border border-slate-200 rounded-md text-slate-900 placeholder:text-slate-400 focus:outline-none focus:border-blue-600 focus:bg-white"
                  />
                </div>

                <div className="grid grid-cols-4 gap-1 p-1 bg-slate-100 rounded-md">
                  {(
                    [
                      ['all', 'Все'],
                      ['crates', 'Крейты'],
                      ['docs', 'Доки'],
                      ['config', 'CI'],
                    ] as const
                  ).map(([key, label]) => (
                    <button
                      key={key}
                      type="button"
                      onClick={() => setCategoryFilter(key)}
                      className={`py-1 text-xs font-medium rounded transition-colors whitespace-nowrap ${
                        categoryFilter === key
                          ? 'bg-white text-slate-950 shadow-xs'
                          : 'text-slate-600 hover:text-slate-900'
                      }`}
                    >
                      {label}
                    </button>
                  ))}
                </div>
              </div>

              <div className="max-h-[calc(100vh-290px)] overflow-y-auto divide-y divide-slate-100">
                {groupedFiles.length === 0 ? (
                  <div className="p-6 text-center text-xs text-slate-500">
                    Файлы по запросу не найдены.
                  </div>
                ) : (
                  groupedFiles.map(([group, groupFiles]) => (
                    <div key={group} className="py-2">
                      <div className="px-3 py-1 flex items-center gap-1.5 text-xs font-semibold text-slate-500">
                        <FolderOpen className="w-3.5 h-3.5 text-slate-400 shrink-0" />
                        <span className="truncate font-mono">{group}</span>
                      </div>
                      <ul className="mt-0.5">
                        {groupFiles.map((file) => {
                          const isSelected = activeFile?.path === file.path;
                          const shortName = file.path.startsWith(`${group}/`)
                            ? file.path.slice(group.length + 1)
                            : file.path;
                          const isRust = file.path.endsWith('.rs');
                          return (
                            <li key={file.path}>
                              <button
                                type="button"
                                onClick={() => setSelectedPath(file.path)}
                                className={`w-full px-3 py-1.5 text-left flex items-center justify-between gap-2 text-xs font-mono transition-colors ${
                                  isSelected
                                    ? 'bg-blue-50 text-blue-800 font-medium'
                                    : 'text-slate-700 hover:bg-slate-50 hover:text-slate-950'
                                }`}
                              >
                                <span className="flex items-center gap-2 min-w-0">
                                  {isRust ? (
                                    <FileCode2 className="w-3.5 h-3.5 shrink-0 text-slate-400" />
                                  ) : (
                                    <FileText className="w-3.5 h-3.5 shrink-0 text-slate-400" />
                                  )}
                                  <span className="truncate">{shortName}</span>
                                </span>
                                {file.content.length === 0 && (
                                  <span className="text-[11px] text-slate-400 shrink-0">0 B</span>
                                )}
                              </button>
                            </li>
                          );
                        })}
                      </ul>
                    </div>
                  ))
                )}
              </div>
            </aside>

            {/* File Content Viewer */}
            <section className="bg-white border border-slate-200 rounded-lg overflow-hidden min-w-0">
              {activeFile && (
                <>
                  <div className="px-5 py-3 border-b border-slate-200 flex flex-wrap items-center justify-between gap-4">
                    <div className="min-w-0">
                      <div className="font-mono text-sm font-semibold text-slate-950 truncate">
                        {activeFile.path}
                      </div>
                      <div className="mt-0.5 text-xs text-slate-500 font-mono tabular-nums">
                        {activeLines.length} строк · {new TextEncoder().encode(activeFile.content).length} байт
                      </div>
                    </div>
                    <button
                      type="button"
                      onClick={() => handleCopyContent(activeFile.path, activeFile.content)}
                      className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-slate-700 bg-slate-100 hover:bg-slate-200 rounded-md transition-colors whitespace-nowrap shrink-0"
                    >
                      {copiedPath === activeFile.path ? (
                        <>
                          <Check className="w-3.5 h-3.5 text-emerald-700" />
                          <span>Скопировано</span>
                        </>
                      ) : (
                        <>
                          <Copy className="w-3.5 h-3.5" />
                          <span>Копировать файл</span>
                        </>
                      )}
                    </button>
                  </div>

                  {activeLines.length === 0 ? (
                    <div className="p-8 text-sm text-slate-500">
                      Пустой файл-каркас <span className="font-mono text-slate-800">{activeFile.path}</span> (0 байт), предусмотренный спецификацией v0.0.1.
                    </div>
                  ) : (
                    <div className="overflow-x-auto max-h-[calc(100vh-290px)] overflow-y-auto">
                      <table className="w-full border-collapse text-xs font-mono leading-5">
                        <tbody>
                          {activeLines.map((line, index) => (
                            <tr key={index} className="hover:bg-slate-50">
                              <td className="select-none text-right pr-4 pl-4 py-0.5 text-slate-400 border-r border-slate-100 w-12 align-top tabular-nums">
                                {index + 1}
                              </td>
                              <td className="pl-4 pr-6 py-0.5 text-slate-800 whitespace-pre">
                                {line.length > 0 ? line : ' '}
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  )}
                </>
              )}
            </section>
          </div>
        )}

        {activeTab === 'architecture' && (
          <div className="space-y-6">
            <div className="bg-white border border-slate-200 rounded-lg p-6">
              <h2 className="text-base font-semibold text-slate-950">
                Слои и границы зависимостей workspace
              </h2>
              <p className="mt-1 text-sm text-slate-600 max-w-3xl">
                Модули <span className="font-mono text-slate-800">crates/mod-*</span> могут зависеть только от <span className="font-mono text-slate-800">crates/core</span> и <span className="font-mono text-slate-800">crates/platform-api</span>. Весь <span className="font-mono text-slate-800">unsafe</span>-код и вызовы WinAPI изолированы в <span className="font-mono text-slate-800">crates/platform-windows</span>.
              </p>

              <div className="mt-6 space-y-6">
                {CRATE_LAYERS.map((group) => (
                  <div key={group.layer}>
                    <h3 className="text-sm font-semibold text-slate-900 mb-2">
                      {group.layer}
                    </h3>
                    <div className="overflow-x-auto border border-slate-200 rounded-md">
                      <table className="w-full text-left border-collapse text-sm">
                        <thead>
                          <tr className="bg-slate-50 border-b border-slate-200 text-xs font-semibold text-slate-600">
                            <th className="py-2.5 px-4">Крейт</th>
                            <th className="py-2.5 px-4">Пакет</th>
                            <th className="py-2.5 px-4">Разрешённые зависимости</th>
                            <th className="py-2.5 px-4">Политика unsafe</th>
                            <th className="py-2.5 px-4">Назначение</th>
                          </tr>
                        </thead>
                        <tbody className="divide-y divide-slate-100">
                          {group.crates.map((crate) => (
                            <tr key={crate.path} className="hover:bg-slate-50">
                              <td className="py-2.5 px-4 font-mono text-xs font-medium text-blue-800 whitespace-nowrap">
                                <button
                                  type="button"
                                  onClick={() => {
                                    setSelectedPath(`${crate.path}/MAP.md`);
                                    setActiveTab('files');
                                  }}
                                  className="hover:underline"
                                >
                                  {crate.path}
                                </button>
                              </td>
                              <td className="py-2.5 px-4 font-mono text-xs text-slate-700 whitespace-nowrap">
                                {crate.pkg}
                              </td>
                              <td className="py-2.5 px-4 font-mono text-xs text-slate-600">
                                {crate.deps}
                              </td>
                              <td className="py-2.5 px-4 text-xs text-slate-700 whitespace-nowrap">
                                {crate.unsafeAllowed}
                              </td>
                              <td className="py-2.5 px-4 text-xs text-slate-600">
                                {crate.role}
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {activeTab === 'budgets' && (
          <div className="bg-white border border-slate-200 rounded-lg overflow-hidden">
            <div className="px-6 py-4 border-b border-slate-200 flex flex-wrap items-center justify-between gap-4">
              <div>
                <h2 className="text-base font-semibold text-slate-950">
                  Нефункциональные бюджеты NF-001..NF-010
                </h2>
                <p className="mt-0.5 text-sm text-slate-600">
                  Зафиксированы в <span className="font-mono text-slate-800">SPEC.md</span> и проверяются при подготовке каждого релиза.
                </p>
              </div>
              <button
                type="button"
                onClick={() => {
                  setSelectedPath('SPEC.md');
                  setActiveTab('files');
                }}
                className="px-3 py-1.5 text-xs font-medium text-slate-700 bg-slate-100 hover:bg-slate-200 rounded-md transition-colors whitespace-nowrap"
              >
                Открыть SPEC.md
              </button>
            </div>

            <div className="overflow-x-auto">
              <table className="w-full text-left border-collapse text-sm">
                <thead>
                  <tr className="bg-slate-50 border-b border-slate-200 text-xs font-semibold text-slate-600">
                    <th className="py-2.5 px-6 w-24">ID</th>
                    <th className="py-2.5 px-4 w-56">Требование</th>
                    <th className="py-2.5 px-4">Бюджет и поведение</th>
                    <th className="py-2.5 px-6">Проверка</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-slate-100">
                  {NF_BUDGETS.map((item) => (
                    <tr key={item.id} className="hover:bg-slate-50">
                      <td className="py-3 px-6 font-mono text-xs font-semibold text-slate-950 tabular-nums whitespace-nowrap">
                        {item.id}
                      </td>
                      <td className="py-3 px-4 text-xs font-semibold text-slate-900">
                        {item.name}
                      </td>
                      <td className="py-3 px-4 text-xs text-slate-700">
                        {item.budget}
                      </td>
                      <td className="py-3 px-6 text-xs text-slate-600">
                        {item.verification}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {activeTab === 'ci' && (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <section className="bg-white border border-slate-200 rounded-lg p-6">
              <div className="flex items-center justify-between gap-4">
                <div className="flex items-center gap-2">
                  <ShieldCheck className="w-5 h-5 text-emerald-700 shrink-0" />
                  <h2 className="text-base font-semibold text-slate-950">
                    Проверка ATLAS.md и MAP.md
                  </h2>
                </div>
                <span className="font-mono text-xs text-slate-500 tabular-nums">
                  {atlasReport.cratesChecked} крейтов · {atlasReport.rustFilesChecked} .rs файлов
                </span>
              </div>

              <p className="mt-2 text-sm text-slate-600">
                Живая сверка реальных файлов каркаса, эквивалентная команде{' '}
                <code className="text-xs bg-slate-100 px-1.5 py-0.5 rounded text-slate-900">
                  cargo run -p xtask -- atlas-check
                </code>.
              </p>

              {atlasReport.passed ? (
                <div className="mt-4 p-4 bg-emerald-50 border border-emerald-200 rounded-md text-sm text-emerald-900">
                  Все {atlasReport.cratesChecked} крейтов из <span className="font-mono">crates/</span> и <span className="font-mono">xtask</span> упомянуты в <span className="font-mono">ATLAS.md</span>, и все {atlasReport.rustFilesChecked} исходных <span className="font-mono">.rs</span>-файлов отражены в соответствующих <span className="font-mono">MAP.md</span>.
                </div>
              ) : (
                <div className="mt-4 p-4 bg-red-50 border border-red-200 rounded-md text-sm text-red-900 space-y-1">
                  {atlasReport.issues.map((issue, idx) => (
                    <div key={idx} className="font-mono text-xs">
                      {issue.scope}: {issue.message}
                    </div>
                  ))}
                </div>
              )}

              <div className="mt-5 flex items-center gap-3">
                <button
                  type="button"
                  onClick={() => {
                    setSelectedPath('xtask/src/main.rs');
                    setActiveTab('files');
                  }}
                  className="px-3 py-1.5 text-xs font-medium text-slate-700 bg-slate-100 hover:bg-slate-200 rounded-md transition-colors"
                >
                  Открыть xtask/src/main.rs
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setSelectedPath('ATLAS.md');
                    setActiveTab('files');
                  }}
                  className="px-3 py-1.5 text-xs font-medium text-slate-700 bg-slate-100 hover:bg-slate-200 rounded-md transition-colors"
                >
                  Открыть ATLAS.md
                </button>
              </div>
            </section>

            <section className="bg-white border border-slate-200 rounded-lg p-6">
              <div className="flex items-center justify-between gap-4">
                <div className="flex items-center gap-2">
                  <Terminal className="w-5 h-5 text-slate-700 shrink-0" />
                  <h2 className="text-base font-semibold text-slate-950">
                    Конвейер GitHub Actions (windows-latest)
                  </h2>
                </div>
                <span className="font-mono text-xs text-amber-700">
                  Ожидает удалённого CI
                </span>
              </div>

              <p className="mt-2 text-sm text-slate-600">
                Согласно <span className="font-mono text-slate-800">AGENTS.md</span>, компиляция и прогон Rust-проверок выполняются только на раннере <span className="font-mono text-slate-800">windows-latest</span> с кешированием <span className="font-mono text-slate-800">Swatinem/rust-cache@v2</span>.
              </p>

              <ol className="mt-4 divide-y divide-slate-100 border border-slate-200 rounded-md">
                {CI_STEPS.map((step) => (
                  <li key={step.order} className="px-4 py-2.5 flex items-center justify-between gap-4">
                    <div className="min-w-0">
                      <div className="text-xs font-medium text-slate-900">
                        <span className="font-mono text-slate-400 mr-2 tabular-nums">{step.order}.</span>
                        {step.name}
                      </div>
                      <div className="mt-0.5 font-mono text-xs text-slate-600 truncate">
                        {step.command}
                      </div>
                    </div>
                  </li>
                ))}
              </ol>

              <div className="mt-5">
                <button
                  type="button"
                  onClick={() => {
                    setSelectedPath('.github/workflows/ci.yml');
                    setActiveTab('files');
                  }}
                  className="px-3 py-1.5 text-xs font-medium text-slate-700 bg-slate-100 hover:bg-slate-200 rounded-md transition-colors"
                >
                  Открыть .github/workflows/ci.yml
                </button>
              </div>
            </section>
          </div>
        )}
      </main>
    </div>
  );
}
