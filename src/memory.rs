// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! How much memory a run may take, and how much it has taken.
//!
//! A run that outgrows its machine is killed by the kernel with nothing said - the
//! process is gone before it can report - so what it may take has to be known up front
//! and kept to.  The limit is read here from the most local source that has one: the
//! amount the caller gave (`--memory`, `GDSCHECK_MEMORY`), else the tightest cgroup
//! limit above the process (a container, a `systemd-run` scope, a CI runner), else the
//! machine's `MemTotal`; a found limit is taken less a tenth, for the rest of the system.
//! Reading `MemTotal` alone was how a run inside a 12 GB container planned for 30 GB of
//! merge cache and was killed at rule 663 of 905 (issue #32).

use std::path::{Path, PathBuf};

/// What one polygon copy in the merge cache is planned at.  Measured on the gf180
/// reference design at 165-253 bytes over 986 rule steps, with up to 5 GB of the
/// resident set beyond what the copies account for - a rule's transient working set -
/// so the plan keeps twice the measured figure as that reserve.
pub const BYTES_PER_COPY: u64 = 500;

/// A memory limit, and where it came from - said in the run's log, so a run that is
/// slower or dies for its memory can be read back to the number it planned with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limit {
    pub bytes: u64,
    pub source: String,
}

/// A size as a person writes it: `12G`, `512M`, `1.5G`, `2T`, or plain bytes.
pub fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let (num, unit) = match s.char_indices().find(|(_, c)| c.is_ascii_alphabetic()) {
        Some((i, _)) => (&s[..i], &s[i..]),
        None => (s, ""),
    };
    let value: f64 = num
        .trim()
        .parse()
        .map_err(|_| format!("`{s}` is not a size (try 12G, 512M or a number of bytes)"))?;
    let scale: f64 = match unit.trim().to_ascii_uppercase().as_str() {
        "" | "B" => 1.0,
        "K" | "KB" | "KIB" => 1024.0,
        "M" | "MB" | "MIB" => 1024.0 * 1024.0,
        "G" | "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
        "T" | "TB" | "TIB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        other => return Err(format!("unknown size unit `{other}` in `{s}`")),
    };
    if value <= 0.0 {
        return Err(format!("a size must be positive, not `{s}`"));
    }
    Ok((value * scale) as u64)
}

/// Bytes in gigabytes, for the log.
pub fn gb(bytes: u64) -> f64 {
    bytes as f64 / 1e9
}

/// The limit a run has to keep under: `given` when the caller set one, else the
/// tightest cgroup limit above this process, else `MemTotal` - the last two less a
/// tenth.  A machine without `/proc` (none of the supported ones) is planned at 32 GB.
pub fn limit(given: Option<u64>) -> Limit {
    if let Some(bytes) = given {
        return Limit {
            bytes,
            source: "given".into(),
        };
    }
    let root = Path::new("/");
    let cgroup = std::fs::read_to_string(root.join("proc/self/cgroup")).unwrap_or_default();
    if let Some(bytes) = cgroup_limit(root, &cgroup) {
        return Limit {
            bytes: bytes / 10 * 9,
            source: format!("cgroup limit {:.1} GB less a tenth", gb(bytes)),
        };
    }
    match mem_total(root) {
        Some(bytes) => Limit {
            bytes: bytes / 10 * 9,
            source: format!("MemTotal {:.1} GB less a tenth", gb(bytes)),
        },
        None => Limit {
            bytes: 32 << 30,
            source: "no /proc/meminfo, assuming 32 GB".into(),
        },
    }
}

/// The resident set of this process, in bytes.
pub fn rss_bytes() -> u64 {
    std::fs::read_to_string("/proc/self/statm")
        .ok()
        .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
        .unwrap_or(0)
        * 4096
}

/// How many polygon copies the merge cache may hold between rules under `limit` with
/// `resident` bytes already taken by what stays for the whole run - the flattened
/// layout and the nets.
pub fn cache_budget_polys(limit: &Limit, resident: u64) -> usize {
    (limit.bytes.saturating_sub(resident) / BYTES_PER_COPY) as usize
}

/// The tightest memory limit set on this process's cgroup or any cgroup above it, from
/// the text of `/proc/self/cgroup`: cgroup v2 (`0::/path`, `memory.max` up the path),
/// else v1 (`N:memory:/path`, `memory.limit_in_bytes` up the path).  A limit is usually
/// set on a scope or slice above the process's own group, whose file says `max`.
fn cgroup_limit(root: &Path, cgroup: &str) -> Option<u64> {
    let mut best: Option<u64> = None;
    for line in cgroup.lines() {
        let mut parts = line.splitn(3, ':');
        let (Some(_), Some(controllers), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let (base, file) = if controllers.is_empty() {
            (root.join("sys/fs/cgroup"), "memory.max")
        } else if controllers.split(',').any(|c| c == "memory") {
            (root.join("sys/fs/cgroup/memory"), "memory.limit_in_bytes")
        } else {
            continue;
        };
        let mut dir: PathBuf = base.join(path.trim_start_matches('/'));
        loop {
            if let Some(v) = read_limit(&dir.join(file)) {
                best = Some(best.map_or(v, |b: u64| b.min(v)));
            }
            if dir == base || !dir.pop() {
                break;
            }
        }
    }
    best
}

/// A cgroup limit file's number, or `None` for `max`, a missing file, or the v1 way of
/// saying none (a value of the order of 2^63).
fn read_limit(path: &Path) -> Option<u64> {
    let v = std::fs::read_to_string(path).ok()?;
    let v = v.trim().parse::<u64>().ok()?;
    (v < 1 << 60).then_some(v)
}

fn mem_total(root: &Path) -> Option<u64> {
    let s = std::fs::read_to_string(root.join("proc/meminfo")).ok()?;
    let kb = s
        .lines()
        .find(|l| l.starts_with("MemTotal:"))?
        .split_whitespace()
        .nth(1)?
        .parse::<u64>()
        .ok()?;
    Some(kb * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_as_written() {
        assert_eq!(parse_size("12G").unwrap(), 12 << 30);
        assert_eq!(parse_size("1.5g").unwrap(), 3 << 29);
        assert_eq!(parse_size("512 MiB").unwrap(), 512 << 20);
        assert_eq!(parse_size("4096").unwrap(), 4096);
        assert!(parse_size("0G").is_err());
        assert!(parse_size("twelve").is_err());
        assert!(parse_size("12X").is_err());
    }

    /// A fake `/`: the process in a v2 scope whose own `memory.max` is `max`, under a
    /// slice with 12 GB and a parent with 64 GB - the tightest one counts.
    #[test]
    fn the_tightest_cgroup_above_the_process() {
        let root = std::env::temp_dir().join(format!("gdscheck-cgroup-{}", std::process::id()));
        let scope = root.join("sys/fs/cgroup/user.slice/app.slice/run.scope");
        std::fs::create_dir_all(&scope).unwrap();
        std::fs::write(scope.join("memory.max"), "max\n").unwrap();
        std::fs::write(
            root.join("sys/fs/cgroup/user.slice/app.slice/memory.max"),
            format!("{}\n", 12u64 << 30),
        )
        .unwrap();
        std::fs::write(
            root.join("sys/fs/cgroup/user.slice/memory.max"),
            format!("{}\n", 64u64 << 30),
        )
        .unwrap();
        assert_eq!(
            cgroup_limit(&root, "0::/user.slice/app.slice/run.scope\n"),
            Some(12 << 30)
        );
        // v1 beside it: a limit of 2^63-ish is none, 8 GB on the parent is one.
        let v1 = root.join("sys/fs/cgroup/memory/docker/abc");
        std::fs::create_dir_all(&v1).unwrap();
        std::fs::write(v1.join("memory.limit_in_bytes"), "9223372036854771712\n").unwrap();
        std::fs::write(
            root.join("sys/fs/cgroup/memory/docker/memory.limit_in_bytes"),
            format!("{}\n", 8u64 << 30),
        )
        .unwrap();
        assert_eq!(
            cgroup_limit(&root, "12:memory:/docker/abc\n11:cpu:/docker/abc\n"),
            Some(8 << 30)
        );
        assert_eq!(cgroup_limit(&root, "0::/nowhere\n"), None);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_budget_is_what_the_limit_leaves() {
        let limit = Limit {
            bytes: 10 << 30,
            source: "given".into(),
        };
        assert_eq!(
            cache_budget_polys(&limit, 7 << 30),
            ((3u64 << 30) / BYTES_PER_COPY) as usize
        );
        assert_eq!(cache_budget_polys(&limit, 11 << 30), 0);
    }
}
