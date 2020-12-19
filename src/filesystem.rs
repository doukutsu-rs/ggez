//! A cross-platform interface to the filesystem.
//!
//! This module provides access to files in specific places:
//!
//! * The `resources/` subdirectory in the same directory as the
//! program executable, if any,
//! * The `resources.zip` file in the same
//! directory as the program executable, if any,
//! * The root folder of the  game's "save" directory which is in a
//! platform-dependent location,
//! such as `~/.local/share/<gameid>/` on Linux.  The `gameid`
//! is the the string passed to
//! [`ContextBuilder::new()`](../struct.ContextBuilder.html#method.new).
//! Some platforms such as Windows also incorporate the `author` string into
//! the path.
//!
//! These locations will be searched for files in the order listed, and the first file
//! found used.  That allows game assets to be easily distributed as an archive
//! file, but locally overridden for testing or modding simply by putting
//! altered copies of them in the game's `resources/` directory.  It
//! is loosely based off of the `PhysicsFS` library.
//!
//! See the source of the [`files` example](https://github.com/ggez/ggez/blob/master/examples/files.rs) for more details.
//!
//! Note that the file lookups WILL follow symlinks!  This module's
//! directory isolation is intended for convenience, not security, so
//! don't assume it will be secure.

use std::env;
use std::fmt;
use std::io;
use std::io::SeekFrom;
use std::path;
use std::path::PathBuf;

use directories::ProjectDirs;

use crate::{Context, GameError, GameResult};
use crate::conf;
use crate::vfs::{self, VFS};
pub use crate::vfs::OpenOptions;

const CONFIG_NAME: &str = "/conf.toml";

/// A structure that contains the filesystem state and cache.
#[derive(Debug)]
pub struct Filesystem {
    vfs: vfs::OverlayFS,
    user_vfs: vfs::OverlayFS,
    //resources_path: path::PathBuf,
    //user_config_path: path::PathBuf,
    user_data_path: path::PathBuf,
}

/// Represents a file, either in the filesystem, or in the resources zip file,
/// or whatever.
pub enum File {
    /// A wrapper for a VFile trait object.
    VfsFile(Box<dyn vfs::VFile>),
}

impl fmt::Debug for File {
    // Make this more useful?
    // But we can't seem to get a filename out of a file,
    // soooooo.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            File::VfsFile(ref _file) => write!(f, "VfsFile"),
        }
    }
}

impl io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match *self {
            File::VfsFile(ref mut f) => f.read(buf),
        }
    }
}

impl io::Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match *self {
            File::VfsFile(ref mut f) => f.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match *self {
            File::VfsFile(ref mut f) => f.flush(),
        }
    }
}

impl io::Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match *self {
            File::VfsFile(ref mut f) => f.seek(pos),
        }
    }
}

impl Filesystem {
    /// Create a new `Filesystem` instance, using the given `id` and (on
    /// some platforms) the `author` as a portion of the user
    /// directory path.  This function is called automatically by
    /// ggez, the end user should never need to call it.
    pub fn new(id: &str) -> GameResult<Filesystem> {
        let mut root_path = env::current_exe()?;

        // Ditch the filename (if any)
        if root_path.file_name().is_some() {
            let _ = root_path.pop();
        }

        // Set up VFS to merge resource path, root path, and zip path.
        let overlay = vfs::OverlayFS::new();
        // User data VFS.
        let mut user_overlay = vfs::OverlayFS::new();

        let user_data_path: PathBuf;
        //let user_config_path;
        // let mut resources_path;
        // let mut resources_zip_path;

        #[cfg(not(target_os = "android"))]
            let project_dirs = match ProjectDirs::from("", "", id) {
            Some(dirs) => dirs,
            None => {
                return Err(GameError::FilesystemError(String::from(
                    "No valid home directory path could be retrieved.",
                )));
            }
        };


        // <game exe root>/resources/
        /*{
            resources_path = root_path.clone();
            resources_path.push("resources");
            trace!("Resources path: {:?}", resources_path);
            let physfs = vfs::PhysicalFS::new(&resources_path, true);
            overlay.push_back(Box::new(physfs));
        }

        // <root>/resources.zip
        {
            resources_zip_path = root_path.clone();
            resources_zip_path.push("resources.zip");
            if resources_zip_path.exists() {
                trace!("Resources zip file: {:?}", resources_zip_path);
                let zipfs = vfs::ZipFS::new(&resources_zip_path)?;
                overlay.push_back(Box::new(zipfs));
            } else {
                trace!("No resources zip file found");
            }
        }*/

        // Per-user data dir,
        // ~/.local/share/whatever/
        {
            user_data_path = {
                #[cfg(not(target_os = "android"))]
                    { project_dirs.data_local_dir().to_path_buf() }

                #[cfg(target_os = "android")]
                    { PathBuf::from(ndk_glue::native_activity().internal_data_path().to_string_lossy().to_string()) }
            };
            log::trace!("User-local data path: {:?}", user_data_path);
            let physfs = vfs::PhysicalFS::new(&user_data_path, false);
            user_overlay.push_back(Box::new(physfs));
        }

        // Writeable local dir, ~/.config/whatever/
        // Save game dir is read-write
        /*{
            user_config_path = project_dirs.config_dir();
            log::trace!("User-local configuration path: {:?}", user_config_path);
            let physfs = vfs::PhysicalFS::new(&user_config_path, false);
            overlay.push_back(Box::new(physfs));
        }*/

        let fs = Filesystem {
            vfs: overlay,
            user_vfs: user_overlay,
            //user_config_path: user_config_path.to_path_buf(),
            user_data_path,
        };

        Ok(fs)
    }

    /// Opens the given `path` and returns the resulting `File`
    /// in read-only mode.
    pub(crate) fn open<P: AsRef<path::Path>>(&mut self, path: P) -> GameResult<File> {
        self.vfs.open(path.as_ref()).map(|f| File::VfsFile(f))
    }

    /// Opens the given `path` from user directory and returns the resulting `File`
    /// in read-only mode.
    pub(crate) fn user_open<P: AsRef<path::Path>>(&mut self, path: P) -> GameResult<File> {
        self.user_vfs.open(path.as_ref()).map(|f| File::VfsFile(f))
    }

    /// Opens a file in the user directory with the given
    /// [`filesystem::OpenOptions`](struct.OpenOptions.html).
    /// Note that even if you open a file read-write, it can only
    /// write to files in the "user" directory.
    pub(crate) fn open_options<P: AsRef<path::Path>>(
        &mut self,
        path: P,
        options: OpenOptions,
    ) -> GameResult<File> {
        self.user_vfs
            .open_options(path.as_ref(), options)
            .map(|f| File::VfsFile(f))
            .map_err(|e| {
                GameError::ResourceLoadError(format!(
                    "Tried to open {:?} but got error: {:?}",
                    path.as_ref(),
                    e
                ))
            })
    }

    /// Creates a new file in the user directory and opens it
    /// to be written to, truncating it if it already exists.
    pub(crate) fn user_create<P: AsRef<path::Path>>(&mut self, path: P) -> GameResult<File> {
        self.user_vfs.create(path.as_ref()).map(|f| File::VfsFile(f))
    }

    /// Create an empty directory in the user dir
    /// with the given name.  Any parents to that directory
    /// that do not exist will be created.
    pub(crate) fn user_create_dir<P: AsRef<path::Path>>(&mut self, path: P) -> GameResult<()> {
        self.user_vfs.mkdir(path.as_ref())
    }

    /// Deletes the specified file in the user dir.
    pub(crate) fn user_delete<P: AsRef<path::Path>>(&mut self, path: P) -> GameResult<()> {
        self.user_vfs.rm(path.as_ref())
    }

    /// Deletes the specified directory in the user dir,
    /// and all its contents!
    pub(crate) fn user_delete_dir<P: AsRef<path::Path>>(&mut self, path: P) -> GameResult<()> {
        self.user_vfs.rmrf(path.as_ref())
    }

    /// Check whether a file or directory in the user directory exists.
    pub(crate) fn user_exists<P: AsRef<path::Path>>(&self, path: P) -> bool {
        self.user_vfs.exists(path.as_ref())
    }

    /// Check whether a file or directory exists.
    pub(crate) fn exists<P: AsRef<path::Path>>(&self, path: P) -> bool {
        self.vfs.exists(path.as_ref())
    }

    /// Check whether a path points at a file.
    pub(crate) fn user_is_file<P: AsRef<path::Path>>(&self, path: P) -> bool {
        self.user_vfs
            .metadata(path.as_ref())
            .map(|m| m.is_file())
            .unwrap_or(false)
    }

    /// Check whether a path points at a file.
    pub(crate) fn is_file<P: AsRef<path::Path>>(&self, path: P) -> bool {
        self.vfs
            .metadata(path.as_ref())
            .map(|m| m.is_file())
            .unwrap_or(false)
    }

    /// Check whether a path points at a directory.
    pub(crate) fn user_is_dir<P: AsRef<path::Path>>(&self, path: P) -> bool {
        self.user_vfs
            .metadata(path.as_ref())
            .map(|m| m.is_dir())
            .unwrap_or(false)
    }

    /// Check whether a path points at a directory.
    pub(crate) fn is_dir<P: AsRef<path::Path>>(&self, path: P) -> bool {
        self.vfs
            .metadata(path.as_ref())
            .map(|m| m.is_dir())
            .unwrap_or(false)
    }

    /// Returns a list of all files and directories in the user directory,
    /// in no particular order.
    ///
    /// Lists the base directory if an empty path is given.
    pub(crate) fn user_read_dir<P: AsRef<path::Path>>(
        &mut self,
        path: P,
    ) -> GameResult<Box<dyn Iterator<Item=path::PathBuf>>> {
        let itr = self.user_vfs.read_dir(path.as_ref())?.map(|fname| {
            fname.expect("Could not read file in read_dir()?  Should never happen, I hope!")
        });
        Ok(Box::new(itr))
    }

    /// Returns a list of all files and directories in the resource directory,
    /// in no particular order.
    ///
    /// Lists the base directory if an empty path is given.
    pub(crate) fn read_dir<P: AsRef<path::Path>>(
        &mut self,
        path: P,
    ) -> GameResult<Box<dyn Iterator<Item=path::PathBuf>>> {
        let itr = self.vfs.read_dir(path.as_ref())?.map(|fname| {
            fname.expect("Could not read file in read_dir()?  Should never happen, I hope!")
        });
        Ok(Box::new(itr))
    }

    fn write_to_string(&mut self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        for vfs in self.vfs.roots() {
            write!(s, "Source {:?}", vfs).expect("Could not write to string; should never happen?");
            match vfs.read_dir(path::Path::new("/")) {
                Ok(files) => {
                    for itm in files {
                        write!(s, "  {:?}", itm)
                            .expect("Could not write to string; should never happen?");
                    }
                }
                Err(e) => write!(s, " Could not read source: {:?}", e)
                    .expect("Could not write to string; should never happen?"),
            }
        }
        s
    }

    /// Prints the contents of all data directories
    /// to standard output.  Useful for debugging.
    pub(crate) fn print_all(&mut self) {
        println!("{}", self.write_to_string());
    }

    /// Outputs the contents of all data directories,
    /// using the "info" log level of the [`log`](https://docs.rs/log/) crate.
    /// Useful for debugging.
    pub(crate) fn log_all(&mut self) {
        info!("{}", self.write_to_string());
    }

    /// Adds the given (absolute) path to the list of directories
    /// it will search to look for resources.
    ///
    /// You probably shouldn't use this in the general case, since it is
    /// harder than it looks to make it bulletproof across platforms.
    /// But it can be very nice for debugging and dev purposes, such as
    /// by pushing `$CARGO_MANIFEST_DIR/resources` to it
    pub(crate) fn mount(&mut self, path: &path::Path, readonly: bool) {
        let physfs = vfs::PhysicalFS::new(path, readonly);
        trace!("Mounting new path: {:?}", physfs);
        self.vfs.push_back(Box::new(physfs));
    }

    pub(crate) fn mount_vfs(&mut self, vfs: Box<dyn vfs::VFS>) {
        self.vfs.push_back(vfs);
    }
}

/// Opens the given path and returns the resulting `File`
/// in read-only mode.
pub fn open<P: AsRef<path::Path>>(ctx: &mut Context, path: P) -> GameResult<File> {
    ctx.filesystem.open(path)
}

/// Opens the given path in the user directory and returns the resulting `File`
/// in read-only mode.
pub fn user_open<P: AsRef<path::Path>>(ctx: &mut Context, path: P) -> GameResult<File> {
    ctx.filesystem.user_open(path)
}

/// Opens a file in the user directory with the given `filesystem::OpenOptions`.
pub fn open_options<P: AsRef<path::Path>>(
    ctx: &mut Context,
    path: P,
    options: OpenOptions,
) -> GameResult<File> {
    ctx.filesystem.open_options(path, options)
}

/// Creates a new file in the user directory and opens it
/// to be written to, truncating it if it already exists.
pub fn user_create<P: AsRef<path::Path>>(ctx: &mut Context, path: P) -> GameResult<File> {
    ctx.filesystem.user_create(path)
}

/// Create an empty directory in the user dir
/// with the given name.  Any parents to that directory
/// that do not exist will be created.
pub fn user_create_dir<P: AsRef<path::Path>>(ctx: &mut Context, path: P) -> GameResult {
    ctx.filesystem.user_create_dir(path.as_ref())
}

/// Deletes the specified file in the user dir.
pub fn user_delete<P: AsRef<path::Path>>(ctx: &mut Context, path: P) -> GameResult {
    ctx.filesystem.user_delete(path.as_ref())
}

/// Deletes the specified directory in the user dir,
/// and all its contents!
pub fn user_delete_dir<P: AsRef<path::Path>>(ctx: &mut Context, path: P) -> GameResult {
    ctx.filesystem.user_delete_dir(path.as_ref())
}

/// Check whether a file or directory exists.
pub fn user_exists<P: AsRef<path::Path>>(ctx: &Context, path: P) -> bool {
    ctx.filesystem.user_exists(path.as_ref())
}

/// Check whether a path points at a file.
pub fn user_is_file<P: AsRef<path::Path>>(ctx: &Context, path: P) -> bool {
    ctx.filesystem.user_is_file(path)
}

/// Check whether a path points at a directory.
pub fn user_is_dir<P: AsRef<path::Path>>(ctx: &Context, path: P) -> bool {
    ctx.filesystem.user_is_dir(path)
}

/// Returns a list of all files and directories in the user directory,
/// in no particular order.
///
/// Lists the base directory if an empty path is given.
pub fn user_read_dir<P: AsRef<path::Path>>(
    ctx: &mut Context,
    path: P,
) -> GameResult<Box<dyn Iterator<Item=path::PathBuf>>> {
    ctx.filesystem.user_read_dir(path)
}

/// Check whether a file or directory exists.
pub fn exists<P: AsRef<path::Path>>(ctx: &Context, path: P) -> bool {
    ctx.filesystem.exists(path.as_ref())
}

/// Check whether a path points at a file.
pub fn is_file<P: AsRef<path::Path>>(ctx: &Context, path: P) -> bool {
    ctx.filesystem.is_file(path)
}

/// Check whether a path points at a directory.
pub fn is_dir<P: AsRef<path::Path>>(ctx: &Context, path: P) -> bool {
    ctx.filesystem.is_dir(path)
}

/// Returns a list of all files and directories in the resource directory,
/// in no particular order.
///
/// Lists the base directory if an empty path is given.
pub fn read_dir<P: AsRef<path::Path>>(
    ctx: &mut Context,
    path: P,
) -> GameResult<Box<dyn Iterator<Item=path::PathBuf>>> {
    ctx.filesystem.read_dir(path)
}

/// Prints the contents of all data directories.
/// Useful for debugging.
pub fn print_all(ctx: &mut Context) {
    ctx.filesystem.print_all()
}

/// Outputs the contents of all data directories,
/// using the "info" log level of the `log` crate.
/// Useful for debugging.
///
/// See the [`logging` example](https://github.com/ggez/ggez/blob/master/examples/eventloop.rs)
/// for how to collect log information.
pub fn log_all(ctx: &mut Context) {
    ctx.filesystem.log_all()
}

/// Adds the given (absolute) path to the list of directories
/// it will search to look for resources.
///
/// You probably shouldn't use this in the general case, since it is
/// harder than it looks to make it bulletproof across platforms.
/// But it can be very nice for debugging and dev purposes, such as
/// by pushing `$CARGO_MANIFEST_DIR/resources` to it
pub fn mount(ctx: &mut Context, path: &path::Path, readonly: bool) {
    ctx.filesystem.mount(path, readonly)
}

/// Adds a VFS to the list of resource search locations.
pub fn mount_vfs(ctx: &mut Context, vfs: Box<dyn vfs::VFS>) {
    ctx.filesystem.mount_vfs(vfs)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::path;

    use crate::conf;
    use crate::error::*;
    use crate::filesystem::*;
    use crate::vfs;

    fn dummy_fs_for_tests() -> Filesystem {
        let mut path = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources");
        let physfs = vfs::PhysicalFS::new(&path, false);
        let mut ofs = vfs::OverlayFS::new();
        ofs.push_front(Box::new(physfs.clone()));

        let mut user_ofs = vfs::OverlayFS::new();
        user_ofs.push_front(Box::new(physfs.clone()));

        Filesystem {
            vfs: ofs,
            user_vfs: user_ofs,
            //user_config_path: path,
            user_data_path: path,
        }
    }

    #[test]
    fn headless_test_file_exists() {
        let f = dummy_fs_for_tests();

        let tile_file = path::Path::new("/tile.png");
        assert!(f.exists(tile_file));
        assert!(f.is_file(tile_file));

        let tile_file = path::Path::new("/oglebog.png");
        assert!(!f.exists(tile_file));
        assert!(!f.is_file(tile_file));
        assert!(!f.is_dir(tile_file));
    }

    #[test]
    fn headless_test_read_dir() {
        let mut f = dummy_fs_for_tests();

        let dir_contents_size = f.read_dir("/").unwrap().count();
        assert!(dir_contents_size > 0);
    }

    #[test]
    fn headless_test_create_delete_file() {
        let mut fs = dummy_fs_for_tests();
        let test_file = path::Path::new("/testfile.txt");
        let bytes = "test".as_bytes();

        {
            let mut file = fs.user_create(test_file).unwrap();
            let _ = file.write(bytes).unwrap();
        }
        {
            let mut buffer = Vec::new();
            let mut file = fs.open(test_file).unwrap();
            let _ = file.read_to_end(&mut buffer).unwrap();
            assert_eq!(bytes, buffer.as_slice());
        }

        fs.user_delete(test_file).unwrap();
    }

    #[test]
    fn headless_test_file_not_found() {
        let mut fs = dummy_fs_for_tests();
        {
            let rel_file = "testfile.txt";
            match fs.open(rel_file) {
                Err(GameError::ResourceNotFound(_, _)) => (),
                Err(e) => panic!("Invalid error for opening file with relative path: {:?}", e),
                Ok(f) => panic!("Should have gotten an error but instead got {:?}!", f),
            }
        }

        {
            // This absolute path should work on Windows too since we
            // completely remove filesystem roots.
            match fs.open("/ooglebooglebarg.txt") {
                Err(GameError::ResourceNotFound(_, _)) => (),
                Err(e) => panic!("Invalid error for opening nonexistent file: {}", e),
                Ok(f) => panic!("Should have gotten an error but instead got {:?}", f),
            }
        }
    }
}
