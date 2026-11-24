use asrbox_platform_api::WebDavDirEntry;

use crate::normalize_remote_dir;

pub fn parent_remote_dir(dir: &str) -> String {
    let norm = normalize_remote_dir(dir);
    if norm == "/" {
        return "/".to_owned();
    }
    let trimmed = norm.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(0) | None => "/".to_owned(),
        Some(pos) => trimmed[..pos].to_owned(),
    }
}

/// Pure-Rust WebDAV `PROPFIND` XML parser (Depth: 1).
/// Extracts folder and file entries relative to `current_dir`, skips self-entry,
/// percent-decodes UTF-8 paths, and sorts directories first.
pub fn parse_propfind_xml(
    xml: &str,
    base_url_path: &str,
    current_dir: &str,
) -> Vec<WebDavDirEntry> {
    let norm_cur = normalize_remote_dir(current_dir);
    let base_prefix = base_url_path.trim().trim_end_matches('/');
    let mut entries = Vec::new();
    let mut cursor = 0usize;

    while let Some(resp_block) = next_tag_block(xml, "response", &mut cursor) {
        let Some(raw_href) = extract_tag_text(resp_block, "href") else {
            continue;
        };
        let path_part = strip_scheme_host(raw_href.trim());
        let decoded = percent_decode_utf8(path_part);
        let stripped = strip_base_prefix(&decoded, base_prefix);
        let full_path = normalize_remote_dir(&stripped);
        if full_path == norm_cur {
            continue;
        }
        let is_dir = has_collection_tag(resp_block) || raw_href.trim().ends_with('/');
        let name = full_path
            .rsplit('/')
            .next()
            .unwrap_or(&full_path)
            .to_owned();
        if name.is_empty() {
            continue;
        }
        if !entries
            .iter()
            .any(|e: &WebDavDirEntry| e.full_path == full_path)
        {
            entries.push(WebDavDirEntry {
                name,
                full_path,
                is_dir,
            });
        }
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

fn next_tag_block<'a>(xml: &'a str, local_name: &str, cursor: &mut usize) -> Option<&'a str> {
    let bytes = xml.as_bytes();
    while *cursor < bytes.len() {
        let rel_open = xml[*cursor..].find('<')?;
        let open_pos = *cursor + rel_open;
        let after_lt = open_pos + 1;
        let rel_gt = xml[after_lt..].find('>')?;
        let tag_end = after_lt + rel_gt;
        let tag_inner = xml[after_lt..tag_end].trim();
        *cursor = tag_end + 1;

        if tag_inner.starts_with('/') || tag_inner.starts_with('?') || tag_inner.starts_with('!') {
            continue;
        }
        let tag_name = tag_inner.split_whitespace().next().unwrap_or("");
        let tag_local = tag_name.rsplit(':').next().unwrap_or(tag_name);
        if !tag_local.eq_ignore_ascii_case(local_name) {
            continue;
        }
        let close_pos = find_closing_tag(xml, local_name, *cursor)?;
        let block = &xml[*cursor..close_pos];
        if let Some(end_gt) = xml[close_pos..].find('>') {
            *cursor = close_pos + end_gt + 1;
        } else {
            *cursor = xml.len();
        }
        return Some(block);
    }
    None
}

fn find_closing_tag(xml: &str, local_name: &str, mut pos: usize) -> Option<usize> {
    while pos < xml.len() {
        let rel = xml[pos..].find("</")?;
        let close_start = pos + rel;
        let name_start = close_start + 2;
        let rel_gt = xml[name_start..].find('>')?;
        let tag_name = xml[name_start..name_start + rel_gt].trim();
        let tag_local = tag_name.rsplit(':').next().unwrap_or(tag_name);
        if tag_local.eq_ignore_ascii_case(local_name) {
            return Some(close_start);
        }
        pos = name_start + rel_gt + 1;
    }
    None
}

fn extract_tag_text<'a>(block: &'a str, local_name: &str) -> Option<&'a str> {
    let mut cursor = 0usize;
    next_tag_block(block, local_name, &mut cursor)
}

fn has_collection_tag(block: &str) -> bool {
    let lower = block.to_ascii_lowercase();
    lower.contains(":collection") || lower.contains("<collection")
}

fn strip_scheme_host(href: &str) -> &str {
    if let Some(idx) = href.find("://") {
        let rest = &href[idx + 3..];
        if let Some(slash) = rest.find('/') {
            return &rest[slash..];
        }
        return "/";
    }
    href
}

fn strip_base_prefix(path: &str, base_prefix: &str) -> String {
    if !base_prefix.is_empty() && path.starts_with(base_prefix) {
        let rest = &path[base_prefix.len()..];
        if rest.is_empty() {
            return "/".to_owned();
        }
        return rest.to_owned();
    }
    path.to_owned()
}

pub fn percent_decode_utf8(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_propfind_xml_with_utf8_and_sorts_dirs_first() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:">
  <d:response>
    <d:href>/asrbox-backups/</d:href>
    <d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat>
  </d:response>
  <d:response>
    <d:href>/asrbox-backups/backup.zip</d:href>
    <d:propstat><d:prop><d:resourcetype/><d:getcontentlength>4096</d:getcontentlength></d:prop></d:propstat>
  </d:response>
  <d:response>
    <d:href>/asrbox-backups/%D0%94%D0%BE%D0%BA%D1%83%D0%BC%D0%B5%D0%BD%D1%82%D1%8B/</d:href>
    <d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop></d:propstat>
  </d:response>
</d:multistatus>"#;
        let items = parse_propfind_xml(xml, "", "/asrbox-backups");
        assert_eq!(items.len(), 2);
        assert!(items[0].is_dir);
        assert_eq!(items[0].name, "Документы");
        assert_eq!(items[0].full_path, "/asrbox-backups/Документы");
        assert!(!items[1].is_dir);
        assert_eq!(items[1].name, "backup.zip");
        assert_eq!(
            parent_remote_dir("/asrbox-backups/Документы"),
            "/asrbox-backups"
        );
        assert_eq!(parent_remote_dir("/asrbox-backups"), "/");
    }
}
