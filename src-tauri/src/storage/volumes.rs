//! Mounted volumes and their capacity (Disk Overview).

use std::ffi::CStr;
use std::path::Path;
use std::sync::OnceLock;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total: u64,
    /// Free space available to the user. Does not include purgeable space (local snapshots, iCloud
    /// caches) that macOS frees on demand, so Finder may show more.
    pub free: u64,
    pub used: u64,
    pub is_root: bool,
    pub read_only: bool,
    pub local: bool,
}

/// Name of the startup volume ("Macintosh HD"), from `diskutil`, cached.
fn root_volume_name() -> String {
    static NAME: OnceLock<String> = OnceLock::new();
    NAME.get_or_init(|| {
        std::process::Command::new("/usr/sbin/diskutil")
            .args(["info", "-plist", "/"])
            .output()
            .ok()
            .and_then(|o| plist::Value::from_reader_xml(o.stdout.as_slice()).ok())
            .and_then(|v| v.as_dictionary()?.get("VolumeName")?.as_string().map(String::from))
            .unwrap_or_else(|| "Macintosh HD".into())
    })
    .clone()
}

pub fn list() -> Vec<Volume> {
    let mut mounts: *mut libc::statfs = std::ptr::null_mut();
    let count = unsafe { libc::getmntinfo(&mut mounts, libc::MNT_NOWAIT) };
    if count <= 0 || mounts.is_null() {
        return Vec::new();
    }
    let entries = unsafe { std::slice::from_raw_parts(mounts, count as usize) };
    let mut out = Vec::new();
    for st in entries {
        let mount = unsafe { CStr::from_ptr(st.f_mntonname.as_ptr()) }.to_string_lossy().into_owned();
        let fs = unsafe { CStr::from_ptr(st.f_fstypename.as_ptr()) }.to_string_lossy().into_owned();
        let is_root = mount == "/";
        // Only the startup volume and user-visible volumes; system volumes of the APFS container
        // (Data, Preboot, VM…) share the same space and are hidden.
        if !is_root && !mount.starts_with("/Volumes/") {
            continue;
        }
        if matches!(fs.as_str(), "devfs" | "autofs" | "nullfs") {
            continue;
        }
        let block = st.f_bsize as u64;
        let total = st.f_blocks * block;
        let free = st.f_bavail * block;
        let flags = st.f_flags;
        out.push(Volume {
            name: if is_root {
                root_volume_name()
            } else {
                Path::new(&mount).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| mount.clone())
            },
            mount_point: mount,
            file_system: fs,
            total,
            free,
            used: total.saturating_sub(st.f_bfree * block),
            is_root,
            read_only: flags & (libc::MNT_RDONLY as u32) != 0,
            local: flags & (libc::MNT_LOCAL as u32) != 0,
        });
    }
    out.sort_by_key(|v| (!v.is_root, v.name.clone()));
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn root_volume_is_listed() {
        let vols = super::list();
        let root = vols.iter().find(|v| v.is_root).expect("root volume");
        assert!(root.total > 0 && root.free <= root.total);
    }
}
