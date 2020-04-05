use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use icon::prepend_grep_icon;

use crate::light_command::{set_current_dir, LightCommand};
use crate::utils::{get_cached_entry, read_first_lines};

fn prepare_grep_and_args(cmd_str: &str, cmd_dir: Option<PathBuf>) -> (Command, Vec<&str>) {
    let args = cmd_str.split_whitespace().collect::<Vec<&str>>();

    let mut cmd = Command::new(args[0]);

    set_current_dir(&mut cmd, cmd_dir);

    (cmd, args)
}

pub fn run(
    grep_cmd: String,
    grep_query: &str,
    glob: Option<&str>,
    cmd_dir: Option<PathBuf>,
    number: Option<usize>,
    enable_icon: bool,
) -> Result<()> {
    let (mut cmd, mut args) = prepare_grep_and_args(&grep_cmd, cmd_dir);

    // We split out the grep opts and query in case of the possible escape issue of clap.
    args.push(grep_query);

    if let Some(g) = glob {
        args.push("-g");
        args.push(g);
    }

    // currently vim-clap only supports rg.
    // Ref https://github.com/liuchengxu/vim-clap/pull/60
    if cfg!(windows) {
        args.push(".");
    }

    cmd.args(&args[1..]);

    let mut light_cmd = LightCommand::new_grep(&mut cmd, number, enable_icon);

    light_cmd.execute(&args)?;

    Ok(())
}

fn is_git_repo(dir: &Path) -> bool {
    let mut gitdir = dir.to_owned();
    gitdir.push(".git");
    gitdir.exists()
}

fn cache_exists(args: &[&str], cmd_dir: &PathBuf) -> bool {
    if let Ok(cached_entry) = get_cached_entry(args, cmd_dir) {
        let tempfile = cached_entry.path();
        if let Some(path_str) = cached_entry.file_name().to_str() {
            let info = path_str.split('_').collect::<Vec<_>>();
            if info.len() == 2 {
                let total = info[1].parse::<u64>().unwrap();
                let using_cache = true;
                if let Ok(lines_iter) = read_first_lines(&tempfile, 100) {
                    let lines = lines_iter
                        .map(|x| prepend_grep_icon(&x))
                        .collect::<Vec<_>>();
                    print_json_with_length!(total, lines, tempfile, using_cache);
                } else {
                    print_json_with_length!(total, tempfile, using_cache);
                }
                // TODO: refresh the cache or mark it as outdated?
                return true;
            }
        }
    }
    false
}

pub fn dyn_grep(
    grep_query: &str,
    cmd_dir: Option<PathBuf>,
    input: Option<PathBuf>,
    number: Option<usize>,
    enable_icon: bool,
) -> Result<()> {
    let rg_cmd = "rg --column --line-number --no-heading --color=never --smart-case ''";

    let args = [
        "rg",
        "--column",
        "--line-number",
        "--no-heading",
        "--color=never",
        "--smart-case",
        "''",
    ];

    let source: fuzzy_filter::Source<std::iter::Empty<_>> = if let Some(tempfile) = input {
        fuzzy_filter::Source::File(tempfile)
    } else if let Some(dir) = cmd_dir {
        if cache_exists(&args, &dir) {
            return Ok(());
        }
        fuzzy_filter::subprocess::Exec::shell(rg_cmd)
            .cwd(dir)
            .into()
    } else {
        fuzzy_filter::subprocess::Exec::shell(rg_cmd).into()
    };

    crate::cmd::filter::dyn_run(grep_query, source, None, number, enable_icon, None, true)
}

pub fn run_forerunner(
    cmd_dir: Option<PathBuf>,
    number: Option<usize>,
    enable_icon: bool,
) -> Result<()> {
    let mut cmd = Command::new("rg");
    let args = [
        "--column",
        "--line-number",
        "--no-heading",
        "--color=never",
        "--smart-case",
        "",
    ];
    // Do not use --vimgrep here.
    cmd.args(&args);

    // Only spawn the forerunner job for git repo for now.
    if let Some(dir) = &cmd_dir {
        if !is_git_repo(dir) {
            return Ok(());
        }
    } else if let Ok(dir) = std::env::current_dir() {
        if !is_git_repo(&dir) {
            return Ok(());
        }
    }

    set_current_dir(&mut cmd, cmd_dir);

    let mut light_cmd = LightCommand::new_grep(&mut cmd, number, enable_icon);

    light_cmd.execute(&args)?;

    Ok(())
}

#[test]
fn test_git_repo() {
    let mut cmd_dir: PathBuf = "/Users/xuliucheng/.vim/plugged/vim-clap".into();
    cmd_dir.push(".git");
    if cmd_dir.exists() {
        println!("{:?} exists", cmd_dir);
    } else {
        println!("{:?} does not exist", cmd_dir);
    }
}
