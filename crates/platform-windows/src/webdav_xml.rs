use asrbox_platform_api::WebDavDirEntry;

pub(crate) fn parse_propfind_xml(
    xml: &str,
    base_url_path: &str,
    current_dir: &str,
) -> Vec<WebDavDirEntry> {
    let norm_cur = normalize_dir(current_dir);
    let base_prefix = base_url_path.trim().trim_end_matches('/');
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while let Some(block) = next_tag(xml, "response", &mut cursor) {
        let mut hcur = 0usize;
        let Some(raw_href) = next_tag(block, "href", &mut hcur) else {
            continue;
        };
        let path_part = strip_host(raw_href.trim());
        let decoded = percent_decode(path_part);
        let stripped = if !base_prefix.is_empty() && decoded.starts_with(base_prefix) {
            &decoded[base_prefix.len()..]
        } else {
            &decoded
        };
        let full_path = normalize_dir(stripped);
        if full_path == norm_cur {
            continue;
        }
        let low = block.to_ascii_lowercase();
        let is_dir = low.contains(":collection")
            || low.contains("<collection")
            || raw_href.trim().ends_with('/');
        let name = full_path
            .rsplit('/')
            .next()
            .unwrap_or(&full_path)
            .to_owned();
        if !name.is_empty()
            && !out
                .iter()
                .any(|e: &WebDavDirEntry| e.full_path == full_path)
        {
            out.push(WebDavDirEntry {
                name,
                full_path,
                is_dir,
            });
        }
    }
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    out
}

fn next_tag<'a>(xml: &'a str, local: &str, cursor: &mut usize) -> Option<&'a str> {
    while *cursor < xml.len() {
        let open = *cursor + xml[*cursor..].find('<')?;
        let end_gt = open + 1 + xml[open + 1..].find('>')?;
        let inner = xml[open + 1..end_gt].trim();
        *cursor = end_gt + 1;
        if inner.starts_with('/') || inner.starts_with('?') || inner.starts_with('!') {
            continue;
        }
        let name = inner.split_whitespace().next().unwrap_or("");
        if !name
            .rsplit(':')
            .next()
            .unwrap_or(name)
            .eq_ignore_ascii_case(local)
        {
            continue;
        }
        let mut p = *cursor;
        while p < xml.len() {
            let cs = p + xml[p..].find("</")?;
            let cg = cs + 2 + xml[cs + 2..].find('>')?;
            let cn = xml[cs + 2..cg].trim();
            if cn
                .rsplit(':')
                .next()
                .unwrap_or(cn)
                .eq_ignore_ascii_case(local)
            {
                let block = &xml[*cursor..cs];
                *cursor = cg + 1;
                return Some(block);
            }
            p = cg + 1;
        }
        return None;
    }
    None
}

fn strip_host(href: &str) -> &str {
    href.find("://").map_or(href, |i| {
        href[i + 3..].find('/').map_or("/", |s| &href[i + 3 + s..])
    })
}

pub(crate) fn normalize_dir(raw: &str) -> String {
    let parts: Vec<&str> = raw
        .trim()
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect();
    if parts.is_empty() {
        "/".to_owned()
    } else {
        format!("/{}", parts.join("/"))
    }
}

fn percent_decode(input: &str) -> String {
    let b = input.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
