//! Replacing the contents of a file the user saved over (decision 0011: Save and Save As write where the file is).
//!
//! `std::fs::write` truncates the file and then writes it, so a disk that fills, a quota that is used up or a program that stops
//! in between leaves a file that is empty or cut off, which is the user's netlist lost to the very key that was to keep it.
//! Where it can be done without changing anything else of the file, the new bytes are written to a new file of its folder
//! that has the owner, group and mode the file has, flushed to the disk, and renamed over it: a reader sees the old file or the
//! new, never half. A rename makes another file, so where that would change what the file is (it has other names, hard links;
//! it is another user's or another group's, which the user cannot give to a new one, as a project folder shared by a group
//! has them; it has extended attributes or an ACL; the folder is not the user's to write in or has the sticky bit; on Windows,
//! a program holds it open in a way that does not let it be replaced), the bytes are written in place instead, and still
//! never into a file that has been emptied: they go over the old ones from the start, the file is cut to length, and on any
//! failure the old bytes, which were kept in memory, are put back, which takes no block that the file has not got already.
//!
//! What a rename does not keep, and what is not known to be there, is not kept: on Windows, attributes other than read-only
//! (hidden, system), the ACL of the file and its other names (the file gets those of its folder); the extended attributes and
//! ACLs of Linux and macOS are known, and such a file is written in place.

use std::io::{Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// The largest file that is replaced in place: its old text is kept in memory while the new is written over it.
const MAX_IN_PLACE: u64 = 256 * 1024 * 1024;

/// Replace the contents of the file at `path` with `bytes`, or make it if it is not there. A link at `path` is followed, as
/// writing to it was: the file it points to is the one written.
pub fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let at = if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        std::fs::canonicalize(path)?
    } else {
        path.to_path_buf()
    };
    let old = match std::fs::metadata(&at) {
        Ok(meta) => meta,
        Err(e) if e.kind() == ErrorKind::NotFound => return std::fs::write(&at, bytes),
        Err(e) => return Err(e),
    };
    if !old.is_file() {
        return Err(Error::new(ErrorKind::InvalidInput, "it is not a file"));
    }
    // (a rename would replace a file the user may only read, as a write would not)
    std::fs::OpenOptions::new().write(true).open(&at)?;
    if replaced_by_rename(&at, bytes, &old)? {
        return Ok(());
    }
    write_in_place(&at, bytes)
}

/// A name for the new file, in the folder of the file, that is not there: made by the call that fails if it is.
fn temp_name(dir: &Path, attempt: u32) -> PathBuf {
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos());
    dir.join(format!(
        ".rnl-save-{}-{}-{nanos:08x}-{attempt}",
        std::process::id(),
        COUNT.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Make the new file: `create_new`, which fails if the name is taken, and does not follow a link.
fn create_temp(dir: &Path) -> Result<(PathBuf, std::fs::File)> {
    let mut last = Error::new(ErrorKind::AlreadyExists, "no name for a new file");
    for attempt in 0..8 {
        let path = temp_name(dir, attempt);
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => last = e,
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

/// Replace the file by a new one that has the same owner, group and mode: Ok(true) when it is, Ok(false) when that cannot
/// be done without changing something of the file (nothing is left of the attempt), and an error when it fails for another
/// reason, such as a disk that is full.
#[cfg(unix)]
fn replaced_by_rename(at: &Path, bytes: &[u8], old: &std::fs::Metadata) -> Result<bool> {
    use std::os::unix::fs::{fchown, MetadataExt};
    let Some(dir) = at.parent() else { return Ok(false) };
    // another name of the file would keep the old contents, and extended attributes and ACLs would not be on the new one
    if old.nlink() > 1 || has_extended_attributes(at) {
        return Ok(false);
    }
    let (temp, mut file) = match create_temp(dir) {
        Ok(made) => made,
        // (a folder that is not the user's to write in, or whose sticky bit would not let the file be replaced)
        Err(e) if e.kind() == ErrorKind::PermissionDenied || e.kind() == ErrorKind::ReadOnlyFilesystem => {
            return Ok(false)
        }
        Err(e) => return Err(e),
    };
    let done = (|| -> Result<bool> {
        let now = file.metadata()?;
        // the owner and group of the file, which the new one has not unless the user is the owner and the folder's group is the
        // file's: set them, and where the system does not let the user (another user's file, a group the user is not in), the
        // file is not replaced
        if (now.uid(), now.gid()) != (old.uid(), old.gid()) {
            match fchown(&file, Some(old.uid()), Some(old.gid())) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::PermissionDenied => return Ok(false),
                Err(e) => return Err(e),
            }
        }
        // (after the owner: changing it may clear the mode's set-user-id and set-group-id)
        file.set_permissions(old.permissions())?;
        file.write_all(bytes)?;
        file.sync_all()?;
        match std::fs::rename(&temp, at) {
            Ok(()) => {}
            // (a folder with the sticky bit does not let one user replace the file of another)
            Err(e) if e.kind() == ErrorKind::PermissionDenied => return Ok(false),
            Err(e) => return Err(e),
        }
        // (the rename is in the folder: flushed too, so that a crash does not undo it)
        if let Ok(folder) = std::fs::File::open(dir) {
            let _ = folder.sync_all();
        }
        Ok(true)
    })();
    drop(file);
    if !matches!(done, Ok(true)) {
        let _ = std::fs::remove_file(&temp);
    }
    done
}

/// On Windows the new file has the read-only attribute the old had, and nothing else of it; a rename that fails, as it does
/// where a program holds the file open without sharing it for deletion, is no failure of the save: the bytes go in place.
#[cfg(not(unix))]
fn replaced_by_rename(at: &Path, bytes: &[u8], old: &std::fs::Metadata) -> Result<bool> {
    let Some(dir) = at.parent() else { return Ok(false) };
    let (temp, mut file) = match create_temp(dir) {
        Ok(made) => made,
        Err(e) if e.kind() == ErrorKind::PermissionDenied => return Ok(false),
        Err(e) => return Err(e),
    };
    let done = (|| -> Result<()> {
        file.set_permissions(old.permissions())?;
        file.write_all(bytes)?;
        file.sync_all()
    })();
    drop(file);
    let renamed = done.and_then(|()| std::fs::rename(&temp, at));
    match renamed {
        Ok(()) => Ok(true),
        Err(e) => {
            let _ = std::fs::remove_file(&temp);
            // (a write that failed for want of room is the save's failure; the rename's is only that it cannot be done here)
            if e.kind() == ErrorKind::StorageFull {
                Err(e)
            } else {
                Ok(false)
            }
        }
    }
}

/// Whether a file has extended attributes that a new file would not (on Linux and macOS: the system's own, which every new
/// file gets, are not counted), or the system does not say, in which case it is taken to have: it is then written in place.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn has_extended_attributes(path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    use std::ffi::c_int;
    use std::ffi::{c_char, CString};
    use std::os::unix::ffi::OsStrExt;
    #[cfg(target_os = "linux")]
    extern "C" {
        fn listxattr(path: *const c_char, list: *mut c_char, size: usize) -> isize;
    }
    #[cfg(target_os = "macos")]
    extern "C" {
        fn listxattr(path: *const c_char, list: *mut c_char, size: usize, options: c_int) -> isize;
    }
    #[cfg(target_os = "linux")]
    const OWN: &[&[u8]] = &[b"security.selinux"];
    #[cfg(target_os = "macos")]
    const OWN: &[&[u8]] = &[b"com.apple.provenance"];
    let Ok(name) = CString::new(path.as_os_str().as_bytes()) else { return true };
    let call = |list: *mut c_char, size: usize| -> isize {
        // SAFETY: `name` is a C string that lives through the call, `list` is null with size 0 or a buffer of `size` bytes.
        #[cfg(target_os = "linux")]
        unsafe {
            listxattr(name.as_ptr(), list, size)
        }
        #[cfg(target_os = "macos")]
        unsafe {
            listxattr(name.as_ptr(), list, size, 0)
        }
    };
    let size = call(std::ptr::null_mut(), 0);
    if size < 0 {
        // (a file system without extended attributes says so; any other answer is not known to be none)
        return Error::last_os_error().kind() != ErrorKind::Unsupported;
    }
    if size == 0 {
        return false;
    }
    let mut buffer = vec![0u8; size as usize];
    let got = call(buffer.as_mut_ptr().cast(), buffer.len());
    if got < 0 {
        return true;
    }
    buffer[..got as usize].split(|b| *b == 0).filter(|n| !n.is_empty()).any(|n| !OWN.contains(&n))
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn has_extended_attributes(_: &Path) -> bool {
    false
}

/// What `overwrite` needs of a file, so that its failures can be tried: to read what it holds, to put bytes at its start, to
/// cut it at a length, and to flush it.
trait Target {
    fn read_all(&mut self) -> Result<Vec<u8>>;
    fn put(&mut self, bytes: &[u8]) -> Result<()>;
    fn set_len(&mut self, len: u64) -> Result<()>;
    fn sync(&mut self) -> Result<()>;
}

impl Target for std::fs::File {
    fn read_all(&mut self) -> Result<Vec<u8>> {
        self.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        self.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    fn put(&mut self, bytes: &[u8]) -> Result<()> {
        self.seek(SeekFrom::Start(0))?;
        self.write_all(bytes)
    }

    fn set_len(&mut self, len: u64) -> Result<()> {
        std::fs::File::set_len(self, len)
    }

    fn sync(&mut self) -> Result<()> {
        self.sync_all()
    }
}

/// Make a file hold `new`, in place, so that it holds that or what it held, and never less of either. The new bytes are
/// written over the old from the start, flushed, and only then is the file cut to the new length; read back, they must be
/// what was meant. On any failure the old bytes are put back and the length is restored, and the error is given.
fn overwrite<T: Target>(file: &mut T, new: &[u8]) -> Result<()> {
    let old = file.read_all()?;
    let written = (|| -> Result<()> {
        file.put(new)?;
        file.sync()?;
        file.set_len(new.len() as u64)?;
        file.sync()?;
        if file.read_all()? != new {
            return Err(Error::new(ErrorKind::InvalidData, "what was written is not what is read back"));
        }
        Ok(())
    })();
    if let Err(e) = written {
        // (what cannot be put back, the error says too)
        let restored = file.put(&old).and_then(|()| file.set_len(old.len() as u64)).and_then(|()| file.sync());
        return Err(match restored {
            Ok(()) => e,
            Err(again) => Error::new(e.kind(), format!("{e}; and the old text could not be put back: {again}")),
        });
    }
    Ok(())
}

/// Write the bytes over the file, in place (see `overwrite`).
fn write_in_place(at: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new().read(true).write(true).open(at)?;
    if file.metadata()?.len() > MAX_IN_PLACE {
        return Err(Error::new(ErrorKind::InvalidInput, "the file is too large to be saved in place"));
    }
    overwrite(&mut file, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own, removed when the test ends.
    struct Dir(PathBuf);

    impl Dir {
        fn new(name: &str) -> Dir {
            let dir = std::env::temp_dir().join(format!("rnl-atomic-test-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Dir(dir)
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }

        fn names(&self) -> Vec<String> {
            let mut names: Vec<String> = std::fs::read_dir(&self.0)
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
            }
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_file_is_replaced_whole_and_nothing_is_left_beside_it() {
        let dir = Dir::new("whole");
        let top = dir.file("top.sp");
        std::fs::write(&top, "a long old text, of many bytes\n").unwrap();
        write(&top, b"short\n").unwrap();
        assert_eq!(std::fs::read(&top).unwrap(), b"short\n");
        write(&top, b"a longer text again, than there was\n").unwrap();
        assert_eq!(std::fs::read(&top).unwrap(), b"a longer text again, than there was\n");
        write(&top, b"").unwrap();
        assert_eq!(std::fs::read(&top).unwrap(), b"");
        assert_eq!(dir.names(), vec!["top.sp".to_string()]);
    }

    #[test]
    fn a_file_that_is_not_there_is_made_and_a_folder_is_not_written() {
        let dir = Dir::new("new");
        write(&dir.file("new.sp"), b"x\n").unwrap();
        assert_eq!(std::fs::read(dir.file("new.sp")).unwrap(), b"x\n");
        std::fs::create_dir(dir.file("folder")).unwrap();
        assert!(write(&dir.file("folder"), b"x").is_err());
        assert!(write(&dir.file("no-folder/new.sp"), b"x").is_err());
    }

    #[test]
    fn a_file_the_user_may_not_write_is_not_replaced() {
        let dir = Dir::new("readonly");
        let top = dir.file("top.sp");
        std::fs::write(&top, "mine\n").unwrap();
        let mut mode = std::fs::metadata(&top).unwrap().permissions();
        mode.set_readonly(true);
        std::fs::set_permissions(&top, mode).unwrap();
        // (the super-user may write what it likes)
        if std::fs::OpenOptions::new().write(true).open(&top).is_err() {
            assert!(write(&top, b"x").is_err());
            assert_eq!(std::fs::read(&top).unwrap(), b"mine\n");
        }
        let mut mode = std::fs::metadata(&top).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        mode.set_readonly(false);
        std::fs::set_permissions(&top, mode).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_link_is_followed_to_the_file_it_points_to_which_is_written_and_the_link_stays() {
        let dir = Dir::new("link");
        let real = dir.file("real.sp");
        std::fs::write(&real, "old\n").unwrap();
        std::os::unix::fs::symlink(&real, dir.file("link.sp")).unwrap();
        write(&dir.file("link.sp"), b"new\n").unwrap();
        assert_eq!(std::fs::read(&real).unwrap(), b"new\n");
        assert!(std::fs::symlink_metadata(dir.file("link.sp")).unwrap().file_type().is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn a_save_keeps_the_owner_group_mode_and_other_names_of_the_file() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let dir = Dir::new("shared");
        let top = dir.file("top.sp");
        std::fs::write(&top, "old\n").unwrap();
        std::fs::set_permissions(&top, std::fs::Permissions::from_mode(0o660)).unwrap();
        let before = std::fs::metadata(&top).unwrap();
        write(&top, b"new\n").unwrap();
        let after = std::fs::metadata(&top).unwrap();
        assert_eq!((after.uid(), after.gid(), after.mode() & 0o7777), (before.uid(), before.gid(), 0o660));
        assert_eq!(std::fs::read(&top).unwrap(), b"new\n");
        // (a file with no extended attributes and no other name is replaced whole)
        if !has_extended_attributes(&top) {
            assert_ne!(after.ino(), before.ino());
        }
        // a group the user is in that is not the folder's: the new file is given it, as the old file had it
        let groups = std::process::Command::new("id")
            .arg("-G")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        if let Some(other) = groups.split_whitespace().filter_map(|g| g.parse::<u32>().ok()).find(|g| *g != after.gid())
        {
            std::os::unix::fs::chown(&top, None, Some(other)).unwrap();
            write(&top, b"newer\n").unwrap();
            assert_eq!(std::fs::metadata(&top).unwrap().gid(), other, "the group of the file stays");
        }
        // a file with another name is written in place, so that the other name has the text too
        let linked = dir.file("linked.sp");
        std::fs::hard_link(&top, &linked).unwrap();
        let inode = std::fs::metadata(&top).unwrap().ino();
        write(&top, b"both\n").unwrap();
        assert_eq!(std::fs::read(&linked).unwrap(), b"both\n");
        assert_eq!(std::fs::metadata(&top).unwrap().ino(), inode);
        assert_eq!(std::fs::metadata(&top).unwrap().nlink(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_in_a_folder_the_user_cannot_write_in_is_written_in_place() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let dir = Dir::new("readonly-folder");
        let top = dir.file("top.sp");
        std::fs::write(&top, "old\n").unwrap();
        std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o555)).unwrap();
        let inode = std::fs::metadata(&top).unwrap().ino();
        // (the super-user may write in any folder: there, the file is replaced, which is as well)
        let may_write_in_it = std::fs::File::create(dir.file("probe")).is_ok();
        let _ = std::fs::remove_file(dir.file("probe"));
        let saved = write(&top, b"new text\n");
        std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o755)).unwrap();
        saved.unwrap();
        assert_eq!(std::fs::read(&top).unwrap(), b"new text\n");
        if !may_write_in_it {
            assert_eq!(std::fs::metadata(&top).unwrap().ino(), inode, "the file was written, not replaced");
        }
        assert_eq!(dir.names(), vec!["top.sp".to_string()]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_file_with_extended_attributes_is_written_in_place_and_keeps_them() {
        use std::ffi::{c_char, c_int, c_void, CString};
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::MetadataExt;
        extern "C" {
            fn setxattr(
                path: *const c_char,
                name: *const c_char,
                value: *const c_void,
                size: usize,
                flags: c_int,
            ) -> c_int;
        }
        let dir = Dir::new("xattr");
        let top = dir.file("top.sp");
        std::fs::write(&top, "old\n").unwrap();
        let (path, name) = (CString::new(top.as_os_str().as_bytes()).unwrap(), CString::new("user.rnl-test").unwrap());
        // SAFETY: both are C strings that live through the call, and the value is a buffer of the size given.
        let set = unsafe { setxattr(path.as_ptr(), name.as_ptr(), b"x".as_ptr().cast(), 1, 0) };
        if set != 0 {
            // (a file system with no user attributes, such as a temporary folder in memory: nothing to try here)
            return;
        }
        assert!(has_extended_attributes(&top));
        let inode = std::fs::metadata(&top).unwrap().ino();
        write(&top, b"new\n").unwrap();
        assert_eq!(std::fs::read(&top).unwrap(), b"new\n");
        assert_eq!(std::fs::metadata(&top).unwrap().ino(), inode, "the file was written in place");
        assert!(has_extended_attributes(&top), "its attributes are still there");
    }

    #[cfg(windows)]
    #[test]
    fn a_file_held_open_without_sharing_for_deletion_is_written_in_place_on_windows() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = Dir::new("held");
        let top = dir.file("top.sp");
        std::fs::write(&top, "old\n").unwrap();
        // another program has it open for reading and writing, and does not let it be deleted or renamed over (share mode: read, write)
        let held = std::fs::OpenOptions::new().read(true).write(true).share_mode(1 | 2).open(&top).unwrap();
        write(&top, b"new text\n").unwrap();
        drop(held);
        assert_eq!(std::fs::read(&top).unwrap(), b"new text\n");
        assert_eq!(dir.names(), vec!["top.sp".to_string()], "no new file is left");
    }

    /// A file that is a few bytes in memory, that fails as a disk does: a write that would take a block the file has not got, a
    /// flush that fails.
    struct Failing {
        data: Vec<u8>,
        /// The most the file may hold: the blocks it has (what a full disk lets it write over, and no more).
        room: usize,
        fail_sync: bool,
        syncs: u32,
    }

    impl Target for Failing {
        fn read_all(&mut self) -> Result<Vec<u8>> {
            Ok(self.data.clone())
        }

        fn put(&mut self, bytes: &[u8]) -> Result<()> {
            for (i, b) in bytes.iter().enumerate() {
                if i >= self.room {
                    return Err(Error::new(ErrorKind::StorageFull, "no space left on device"));
                }
                if i < self.data.len() {
                    self.data[i] = *b;
                } else {
                    self.data.push(*b);
                }
            }
            Ok(())
        }

        fn set_len(&mut self, len: u64) -> Result<()> {
            self.data.resize(len as usize, 0);
            Ok(())
        }

        fn sync(&mut self) -> Result<()> {
            self.syncs += 1;
            if self.fail_sync && self.syncs == 1 {
                return Err(Error::other("input/output error"));
            }
            Ok(())
        }
    }

    #[test]
    fn a_write_in_place_that_fails_leaves_the_file_as_it_was() {
        // the disk is full: the new text is longer than the room the file has. Half of it has been written when it fails
        let mut file = Failing { data: b"the old text of the file\n".to_vec(), room: 25, fail_sync: false, syncs: 0 };
        let old = file.data.clone();
        let err = overwrite(&mut file, b"a new text that is longer than the old one, and does not fit\n").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::StorageFull);
        assert_eq!(file.data, old, "byte for byte");
        // a flush that fails after all of a shorter text was written over the start of the old one
        let mut file = Failing {
            data: b"the old text of the file, which is long\n".to_vec(),
            room: 100,
            fail_sync: true,
            syncs: 0,
        };
        let old = file.data.clone();
        assert!(overwrite(&mut file, b"short\n").is_err());
        assert_eq!(file.data, old, "byte for byte");
        // what fits is written whole, cut to its length, and a longer file is made shorter
        let mut file = Failing {
            data: b"the old text of the file, which is long\n".to_vec(),
            room: 100,
            fail_sync: false,
            syncs: 0,
        };
        overwrite(&mut file, b"short\n").unwrap();
        assert_eq!(file.data, b"short\n");
        let mut file = Failing { data: b"ab".to_vec(), room: 100, fail_sync: false, syncs: 0 };
        overwrite(&mut file, b"longer than it was\n").unwrap();
        assert_eq!(file.data, b"longer than it was\n");
    }

    #[test]
    fn a_write_in_place_on_a_real_file_goes_whole_and_is_cut_to_length() {
        let dir = Dir::new("inplace");
        let top = dir.file("top.sp");
        std::fs::write(&top, "a long old text, of many bytes\n").unwrap();
        write_in_place(&top, b"short\n").unwrap();
        assert_eq!(std::fs::read(&top).unwrap(), b"short\n");
        write_in_place(&top, b"a longer text again\n").unwrap();
        assert_eq!(std::fs::read(&top).unwrap(), b"a longer text again\n");
        write_in_place(&top, b"").unwrap();
        assert_eq!(std::fs::read(&top).unwrap(), b"");
    }
}
