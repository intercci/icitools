use std::fs::{read_dir, read_to_string, write};
use std::io;
use std::path::Path;

/// 递归遍历指定目录，对所有 .rs 文件调用 check_file
fn check_files(path: &Path) -> io::Result<()> {
    for e in read_dir(path)? {
        // 跳过读取失败的目录项
        let Ok(d) = e else {
            continue;
        };
        if d.file_type().is_ok_and(|ft| ft.is_dir()) {
            // 如果是目录，则递归处理
            check_files(&d.path())?;
        } else {
            let path = d.path();
            // 只处理 .rs 扩展名的文件
            if path.extension().is_some_and(|ext| ext == "rs") {
                check_file(&path)?;
            }
        }
    }
    Ok(())
}

/// 检查并修改单个 .rs 文件，按需添加 todo_macro_uses 的 allow 标注
fn check_file(path: &Path) -> io::Result<()> {
    let orig_text = read_to_string(path)?;

    // 如果文件不包含 todo!( 调用，或者已经包含 todo_macro_uses，则跳过
    if !orig_text.contains("todo!(") || orig_text.contains("todo_macro_uses") {
        return Ok(());
    }

    let text = if let Some(pos) = orig_text.find("#![allow(") {
        // 情况一：文件中已有 #![allow(..)]，在其中追加新的 lint 名称
        let Some(insert_pos) = orig_text[pos..].find(")]") else {
            panic!("未闭合的 #![allow()]");
        };
        let (before, after) = orig_text.split_at(pos + insert_pos);
        // 在已有的 allow 列表末尾插入 todo_macro_uses
        format!("{before}, todo_macro_uses{after}")
    } else {
        // 情况二：文件中没有 #![allow(..)]，需要新增一行
        // 找到所有开头 // 注释之后的位置（compiletest 要求注释在最前面）
        let mut pos = 0;
        while orig_text[pos..].starts_with("//") {
            let Some(nl) = orig_text[pos..].find("\n") else {
                pos = orig_text.len();
                break;
            };
            pos += nl + 1;
        }
        let (before, after) = orig_text.split_at(pos);
        // 避免插入多余的空行：如果已经在行首或前面以换行结尾，则不额外加换行
        let nl = if pos == 0 || before.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        format!("{before}{nl}#![allow(todo_macro_uses)]\n{after}")
    };

    // 将修改后的内容写回文件
    write(path, text)
}

fn main() -> io::Result<()> {
    // 从 Rust 源码仓库的 tests/ui 目录开始递归处理
    check_files(&Path::new("../rust/tests/ui"))
}
