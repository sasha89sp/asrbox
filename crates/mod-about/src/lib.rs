use std::path::PathBuf;

const EMBEDDED_CHANGELOG: &str = include_str!("../../../CHANGELOG.md");

pub fn changelog_text() -> &'static str {
    EMBEDDED_CHANGELOG
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub exe_path: PathBuf,
    pub exe_dir: PathBuf,
    pub data_dir: PathBuf,
    pub docks_cfg: PathBuf,
    pub widgets_cfg: PathBuf,
    pub sysmon_cfg: PathBuf,
    pub overlay_cfg: PathBuf,
    pub backup_cfg: PathBuf,
    pub webdav_cfg: PathBuf,
    pub appearance_cfg: PathBuf,
    pub sync_cfg: PathBuf,
    pub backups_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppPaths {
    pub fn from_exe_and_data(exe_path: PathBuf, data_dir: PathBuf) -> Self {
        let exe_dir = exe_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let config_dir = data_dir.join("config");
        Self {
            exe_path,
            exe_dir,
            docks_cfg: config_dir.join("docks.cfg"),
            widgets_cfg: config_dir.join("widgets.cfg"),
            sysmon_cfg: config_dir.join("sysmon.cfg"),
            overlay_cfg: config_dir.join("overlay.cfg"),
            backup_cfg: config_dir.join("backup.cfg"),
            webdav_cfg: config_dir.join("webdav.cfg"),
            appearance_cfg: config_dir.join("appearance.cfg"),
            sync_cfg: config_dir.join("sync.cfg"),
            backups_dir: data_dir.join("backups"),
            cache_dir: data_dir.join("cache"),
            logs_dir: data_dir.join("logs"),
            data_dir,
        }
    }
}

pub fn build_about_text(version: &str, paths: &AppPaths) -> String {
    let mut out = String::new();
    out.push_str(&format!("asrbox v{version} (Windows x64)\r\n"));
    out.push_str("============================================================\r\n");
    out.push_str("Режим работы: ПОРТАТИВНАЯ СБОРКА (Portable)\r\n");
    out.push_str(
        "Все настройки, кэш, резервные копии и логи хранятся в папке data рядом с asrbox.exe.\r\n",
    );
    out.push_str(
        "Автозагрузка (по желанию) пишет 1 ключ HKCU\\...\\Run; пароли WebDAV — в Диспетчере учётных данных Windows.\r\n",
    );
    out.push_str("При обновлении программы достаточно закрыть asrbox.exe и заменить файл\r\n");
    out.push_str("новой версией в той же папке — все настройки сохраняются автоматически.\r\n\r\n");

    out.push_str("ТЕКУЩИЕ ПУТИ НА ДИСКЕ\r\n");
    out.push_str("------------------------------------------------------------\r\n");
    out.push_str(&build_paths_clipboard(paths));
    out.push_str("\r\nИСТОРИЯ ВЕРСИЙ (CHANGELOG)\r\n");
    out.push_str("------------------------------------------------------------\r\n");
    for line in EMBEDDED_CHANGELOG.lines() {
        out.push_str(line);
        out.push_str("\r\n");
    }
    out
}

pub fn build_paths_clipboard(paths: &AppPaths) -> String {
    format!(
        "Исполняемый файл : {}\r\n\
         Папка программы  : {}\r\n\
         Папка данных     : {}\r\n\
         Оформление       : {}\r\n\
         Настройки доков  : {}\r\n\
         Настройки календ.: {}\r\n\
         Системн. монитор : {}\r\n\
         Игровой оверлей  : {}\r\n\
         Настройки бэкапа : {}\r\n\
         Настройки WebDAV : {}\r\n\
         Синхронизация    : {}\r\n\
         Папка бэкапов    : {}\r\n\
         Кэш календарей   : {}\r\n\
         Логи и отчёты    : {}\r\n",
        paths.exe_path.display(),
        paths.exe_dir.display(),
        paths.data_dir.display(),
        paths.appearance_cfg.display(),
        paths.docks_cfg.display(),
        paths.widgets_cfg.display(),
        paths.sysmon_cfg.display(),
        paths.overlay_cfg.display(),
        paths.backup_cfg.display(),
        paths.webdav_cfg.display(),
        paths.sync_cfg.display(),
        paths.backups_dir.display(),
        paths.cache_dir.display(),
        paths.logs_dir.display(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_text_contains_version_portability_paths_and_changelog() {
        let paths = AppPaths::from_exe_and_data(
            PathBuf::from("D:\\Tools\\asrbox\\asrbox.exe"),
            PathBuf::from("D:\\Tools\\asrbox\\data"),
        );
        let text = build_about_text("0.0.5", &paths);
        assert!(text.contains("asrbox v0.0.5"));
        assert!(text.contains("ПОРТАТИВНАЯ СБОРКА"));
        assert!(text.contains("appearance.cfg"));
        assert!(text.contains("sync.cfg"));
        assert!(text.contains("widgets.cfg"));
        assert!(text.contains("docks.cfg"));
        assert!(text.contains("overlay.cfg"));
        assert!(text.contains("backup.cfg"));
        assert!(text.contains("webdav.cfg"));
        assert!(text.contains("CHANGELOG"));
        assert!(!changelog_text().is_empty());
    }
}
