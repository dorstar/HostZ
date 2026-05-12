use anyhow::Result;
use regex::{Regex, RegexBuilder};

use crate::core::content_parser;
use crate::db::swhdb::SwhDb;
use crate::models::hosts::{FindItem, FindPosition, HostsType};

/// 查找选项
#[derive(Debug, Clone)]
pub struct FindOptions {
    /// 是否忽略大小写
    pub is_ignore_case: bool,
}

impl Default for FindOptions {
    fn default() -> Self {
        Self { is_ignore_case: false }
    }
}

/// 在全部 hosts 条目中查找匹配文本
///
/// 遍历所有 local 和 remote 条目（跳过 group），
/// 对每个条目解析其完整内容，查找匹配位置。
pub fn find_by(
    swhdb: &SwhDb,
    query: &str,
    options: FindOptions,
) -> Result<Vec<FindItem>> {
    let list = swhdb.get_list()?;
    let flat = content_parser::flatten(&list);

    // 字面搜索（非正则）
    let pattern = regex::escape(query);
    let regex = RegexBuilder::new(&pattern)
        .case_insensitive(options.is_ignore_case)
        .build()
        .map_err(|e| anyhow::anyhow!("正则表达式无效: {}", e))?;

    let mut results = Vec::new();

    for item in &flat {
        // 只搜索 local 和 remote 条目
        if item.type_ != HostsType::Local && item.type_ != HostsType::Remote {
            continue;
        }

        let content = match content_parser::get_content_of_hosts(swhdb, &list, &item.id) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if content.is_empty() {
            continue;
        }

        let positions = find_positions_in_content(&content, &regex);
        if !positions.is_empty() {
            results.push(FindItem {
                item_id: item.id.clone(),
                item_title: item.title.clone(),
                item_type: item.type_.clone(),
                positions,
            });
        }
    }

    Ok(results)
}

/// 在内容字符串中查找所有匹配位置
fn find_positions_in_content(content: &str, regex: &Regex) -> Vec<FindPosition> {
    let mut positions = Vec::new();

    // 预计算每行的起始字节偏移
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(content.match_indices('\n').map(|(i, _)| i + 1))
        .collect();

    for mat in regex.find_iter(content) {
        let start = mat.start();
        let end = mat.end();

        // 计算行号和行内位置
        let line = line_starts.iter()
            .enumerate()
            .rfind(|(_, &ls)| ls <= start)
            .map(|(i, _)| i + 1)
            .unwrap_or(1);

        let line_start = line_starts.get(line - 1).copied().unwrap_or(0);
        let line_pos = start - line_start;

        let end_line = line_starts.iter()
            .enumerate()
            .rfind(|(_, &ls)| ls <= end)
            .map(|(i, _)| i + 1)
            .unwrap_or(line);

        let end_line_start = line_starts.get(end_line - 1).copied().unwrap_or(0);
        let end_line_pos = end - end_line_start;

        // 上下文：match 开始行在匹配前的部分，以及所在行匹配后的部分
        let line_end = content[line_start..]
            .find('\n')
            .map(|p| line_start + p)
            .unwrap_or(content.len());

        let before = content.get(line_start..start).map(|s| s.to_string()).unwrap_or_default();
        // 如果匹配跨越多行，after 取最后一行匹配结束到行尾的部分
        let after = if end <= line_end {
            content.get(end..line_end).map(|s| s.to_string()).unwrap_or_default()
        } else {
            let last_line_start = line_starts.get(end_line - 1).copied().unwrap_or(0);
            let last_line_end = content[last_line_start..]
                .find('\n')
                .map(|p| last_line_start + p)
                .unwrap_or(content.len());
            content.get(end..last_line_end).map(|s| s.to_string()).unwrap_or_default()
        };
        let match_text = content.get(start..end).map(|s| s.to_string()).unwrap_or_default();

        positions.push(FindPosition {
            start,
            end,
            line,
            line_pos,
            end_line,
            end_line_pos,
            before,
            r#match: match_text,
            after,
        });
    }

    positions
}

/// 替换内容中的匹配项（替换单个匹配）
///
/// 返回替换后的完整内容。
/// position: 要替换的匹配的起始/结束字节偏移
/// replacement: 替换文本
pub fn replace_one(content: &str, start: usize, end: usize, replacement: &str) -> String {
    let mut result = String::with_capacity(content.len());
    result.push_str(&content[..start]);
    result.push_str(replacement);
    result.push_str(&content[end..]);
    result
}

/// 替换内容中的所有匹配项
pub fn replace_all(content: &str, regex: &Regex, replacement: &str) -> String {
    regex.replace_all(content, replacement).to_string()
}

/// 在内容中查找并替换全部匹配，保存到数据库
pub fn find_and_replace_all(
    swhdb: &SwhDb,
    query: &str,
    replacement: &str,
    options: FindOptions,
) -> Result<usize> {
    let pattern = regex::escape(query);
    let regex = RegexBuilder::new(&pattern)
        .case_insensitive(options.is_ignore_case)
        .build()
        .map_err(|e| anyhow::anyhow!("正则表达式无效: {}", e))?;

    let list = swhdb.get_list()?;
    let flat = content_parser::flatten(&list);
    let mut replaced_count = 0;

    for item in &flat {
        if item.type_ != HostsType::Local {
            continue;
        }

        let content = match content_parser::get_content_of_hosts(swhdb, &list, &item.id) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let match_count = regex.find_iter(&content).count();
        if match_count > 0 {
            let new_content = replace_all(&content, &regex, replacement);
            swhdb.set_content(&item.id, &new_content)?;
            replaced_count += match_count;
        }
    }

    Ok(replaced_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_positions_literal() {
        let content = "127.0.0.1 localhost\n# comment\n10.0.0.1 example.com\n";
        let regex = Regex::new("localhost").unwrap();
        let positions = find_positions_in_content(content, &regex);
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].line, 1);
        assert!(positions[0].r#match == "localhost");
    }

    #[test]
    fn test_find_positions_case_insensitive() {
        let content = "127.0.0.1 LocalHost\n";
        let regex = RegexBuilder::new("localhost")
            .case_insensitive(true)
            .build()
            .unwrap();
        let positions = find_positions_in_content(content, &regex);
        assert_eq!(positions.len(), 1);
    }

    #[test]
    fn test_replace_one() {
        let content = "127.0.0.1 oldhost\n";
        // "oldhost" starts at byte 10, ends at byte 17
        let result = replace_one(content, 10, 17, "newhost");
        assert_eq!(result, "127.0.0.1 newhost\n");
    }

    #[test]
    fn test_replace_all() {
        let content = "127.0.0.1 foo\n10.0.0.1 foo\n";
        let regex = Regex::new("foo").unwrap();
        let result = replace_all(content, &regex, "bar");
        assert_eq!(result, "127.0.0.1 bar\n10.0.0.1 bar\n");
    }

    #[test]
    fn test_find_multiline_positions() {
        let content = "line1 abc\nline2 abc\nline3\n";
        let regex = Regex::new("abc").unwrap();
        let positions = find_positions_in_content(content, &regex);
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].line, 1);
        assert_eq!(positions[1].line, 2);
        assert_eq!(positions[0].before, "line1 ");
        assert_eq!(positions[1].after, "");
    }
}
